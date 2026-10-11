//! Small public resident-runtime diagnostic; not the fixed-shape kernel lane.
//! Uses only public compiler, activation, input, turn, and snapshot interfaces.

use std::{collections::BTreeMap, env, fs, io::Write, path::PathBuf, time::Instant};

use mech_core::{CellSlotId, ReactiveInstanceId, SlotIndex, Value, ValueData, snapshot::SequenceView};
use mech_engine::{
    ProgramArtifact, SlotRole,
    resident::{
        ActivationFacts, CapturedValueInput, ReactiveInstance, ResidentActivationOptions,
        ResidentIntegrityMode, ResidentValueBorrow, activate_with_options,
    },
};
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue};

const SOURCE: &str = include_str!("../../mech-driver/ekf-numerical.mec");
const REVISION: &str = "0d85319d60c460cdecb81e03c181d62ecfe1ef39";
const INPUT_NAMES: [&str; 5] = [
    "dt",
    "linear-velocity",
    "angular-velocity",
    "bearing",
    "measurement-noise",
];

struct Options {
    instances: usize,
    turns: u32,
    warmup: u32,
    input: PathBuf,
    dump: Option<PathBuf>,
}

impl Options {
    fn parse() -> Self {
        let mut values = BTreeMap::new();
        let mut args = env::args().skip(1);
        while let Some(flag) = args.next() {
            assert!(
                ["--instances", "--turns", "--warmup", "--input", "--dump"]
                    .contains(&flag.as_str()),
                "unknown option: {flag}"
            );
            assert!(
                values
                    .insert(flag, args.next().expect("option needs a value"))
                    .is_none()
            );
        }
        let result = Self {
            instances: values
                .remove("--instances")
                .unwrap_or_else(|| "4".into())
                .parse()
                .unwrap(),
            turns: values
                .remove("--turns")
                .unwrap_or_else(|| "20".into())
                .parse()
                .unwrap(),
            warmup: values
                .remove("--warmup")
                .unwrap_or_else(|| "5".into())
                .parse()
                .unwrap(),
            input: values
                .remove("--input")
                .expect("--input is required")
                .into(),
            dump: values.remove("--dump").map(Into::into),
        };
        assert!(
            (1..=256).contains(&result.instances),
            "diagnostic is capped at 256 independent runtimes"
        );
        assert!(result.turns > 0);
        result
    }
}

struct Filter {
    instance: ReactiveInstance,
    inputs: Vec<(SlotIndex, Value)>,
}

fn activate_filters(
    artifact: &ProgramArtifact,
    catalog: &mech_core::FunctionCatalog,
    inputs: &BTreeMap<String, Vec<f32>>,
    count: usize,
) -> Vec<Filter> {
    (0..count)
        .map(|lane| {
            let instance = activate_with_options(
                ReactiveInstanceId::new(lane as u32, 0),
                artifact,
                catalog,
                &ActivationFacts::default(),
                ResidentActivationOptions {
                    integrity: ResidentIntegrityMode::Unchecked,
                    ..ResidentActivationOptions::default()
                },
            )
            .expect("public resident activation must admit the same f32 EKF artifact");
            assert_eq!(instance.plan.inputs.len(), 5);
            let values = instance
                .plan
                .inputs
                .iter()
                .map(|declared| {
                    let name = &artifact
                        .inputs()
                        .iter()
                        .find(|input| input.slot == declared.artifact_slot)
                        .expect("activated slot is a declared input")
                        .name;
                    let source = &inputs[name];
                    let number = source[if source.len() == 1 { 0 } else { lane }];
                    let value = RuntimeHostInputValue::F32(number)
                        .into_value()
                        .unwrap()
                        .rebind(declared.schema, &declared.shape, artifact.schemas())
                        .unwrap();
                    (declared.slot, value)
                })
                .collect();
            Filter {
                instance,
                inputs: values,
            }
        })
        .collect()
}

fn turns(filters: &mut [Filter], count: u32) {
    for _ in 0..count {
        for filter in &mut *filters {
            let inputs: [CapturedValueInput<'_>; 5] =
                std::array::from_fn(|index| CapturedValueInput {
                    slot: filter.inputs[index].0,
                    value: &filter.inputs[index].1,
                });
            std::hint::black_box(
                filter
                    .instance
                    .prepare_turn_values(&inputs)
                    .unwrap()
                    .publish()
                    .unwrap(),
            );
        }
    }
}

fn state_arrays(filters: &[Filter], slots: &[CellSlotId]) -> (Vec<f32>, Vec<f32>) {
    let mut pose = Vec::with_capacity(filters.len() * 3);
    let mut covariance = Vec::with_capacity(filters.len() * 9);
    for filter in filters {
        let mut found = [false; 2];
        for slot in slots {
            let ResidentValueBorrow::Snapshot { values, .. } =
                filter.instance.state_borrow(*slot).unwrap()
            else {
                panic!("resident f32 state must use canonical snapshots")
            };
            let [Some(value)] = values else {
                panic!("one matrix snapshot per state slot")
            };
            let ValueData::Matrix(matrix) = value.data() else {
                panic!("EKF state must be a matrix")
            };
            let SequenceView::F32(values) = matrix.elements() else {
                panic!("EKF must retain f32 values")
            };
            let (target, index) = match values.len() {
                3 => (&mut pose, 0),
                9 => (&mut covariance, 1),
                other => panic!("unexpected persistent matrix length {other}"),
            };
            assert!(!found[index]);
            found[index] = true;
            // Canonical snapshot matrix elements are row-major, unlike the
            // fixed-shape backend's internal column-major per-filter storage.
            target.extend(values.iter().map(|value| value.to_f32()));
        }
        assert_eq!(found, [true, true]);
    }
    (pose, covariance)
}

fn main() {
    let options = Options::parse();
    let bytes = fs::read(&options.input).unwrap();
    assert_eq!(bytes.len(), options.instances * 12);
    let raw = bytes
        .chunks_exact(4)
        .map(|value| f32::from_le_bytes(value.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert!(raw.iter().all(|value| value.is_finite()));
    let n = options.instances;
    let inputs = BTreeMap::from([
        ("dt".to_string(), vec![0.1]),
        ("linear-velocity".to_string(), raw[..n].to_vec()),
        ("angular-velocity".to_string(), raw[n..2 * n].to_vec()),
        ("bearing".to_string(), raw[2 * n..].to_vec()),
        ("measurement-noise".to_string(), vec![0.25]),
    ]);
    let started = Instant::now();
    let catalog = mech_stdlib::source_native_plan_catalog();
    let planning_inputs = inputs
        .iter()
        .map(|(name, values)| {
            (
                format!("lane-{name}"),
                RuntimeHostInputValue::F32(values[0]),
            )
        })
        .collect();
    let tree = mech_syntax::parse(SOURCE).unwrap();
    let artifact = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .unwrap()
        .compile_tree_artifact_with_inputs(
            &tree,
            &planning_inputs,
            &INPUT_NAMES.into_iter().map(str::to_owned).collect(),
        )
        .unwrap()
        .into_artifact();
    assert!(artifact.constraints().is_empty());
    let state_slots = artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .map(|slot| slot.slot)
        .collect::<Vec<_>>();
    assert_eq!(state_slots.len(), 2);
    let compilation_seconds = started.elapsed().as_secs_f64();
    let prepared = Instant::now();
    if options.warmup > 0 {
        let mut warmup = activate_filters(&artifact, &catalog, &inputs, n);
        turns(&mut warmup, options.warmup);
        std::hint::black_box(state_arrays(&warmup, &state_slots));
    }
    let mut filters = activate_filters(&artifact, &catalog, &inputs, n);
    let (initial_pose, initial_covariance) = state_arrays(&filters, &state_slots);
    assert!(
        initial_pose
            .chunks_exact(3)
            .all(|state| state == [55.0, 25.0, 0.4])
    );
    assert!(
        initial_covariance
            .chunks_exact(9)
            .all(|cov| cov == [100.0, 0.0, 0.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.15])
    );
    let preparation_and_warmup_seconds = prepared.elapsed().as_secs_f64();
    let timed = Instant::now();
    turns(&mut filters, options.turns);
    let (pose, covariance) = state_arrays(&filters, &state_slots);
    std::hint::black_box((&pose, &covariance));
    let elapsed_s = timed.elapsed().as_secs_f64();
    assert!(
        pose.iter()
            .chain(&covariance)
            .all(|value| value.is_finite())
    );
    if let Some(path) = options.dump {
        let mut file = fs::File::create(path).unwrap();
        for value in pose.iter().chain(&covariance) {
            file.write_all(&value.to_le_bytes()).unwrap();
        }
    }
    println!(
        "{}",
        serde_json::json!({
            "implementation": "mech-public-resident-f32",
            "backend_revision": REVISION,
            "instances": n,
            "turns": options.turns,
            "warmup_turns": options.warmup,
            "reset_after_warmup": true,
            "checked": false,
            "workers": 1,
            "elapsed_s": elapsed_s,
            "filter_turns_per_second": n as f64 * options.turns as f64 / elapsed_s,
            "compilation_seconds": compilation_seconds,
            "preparation_and_warmup_seconds": preparation_and_warmup_seconds,
            "checksum": pose.iter().chain(&covariance).map(|&v| f64::from(v)).sum::<f64>(),
            "first_state": &pose[..3],
            "first_covariance": &covariance[..9],
            "execution": "Public resident f32 snapshot interpreter; one independent reactive instance per filter; not the nalgebra compiler planner and not the fixed-shape kernel executor",
            "timing": "Full public prepare_turn_values + publish, including scheduler, snapshot arithmetic, state hash/summary and final host-readable state extraction; excludes compilation, activation, input rebinding, warmup, reset and file I/O",
            "dump_layout": "little-endian f32: C-order state[N,3], then covariance[N,3,3]"
        })
    );
}
