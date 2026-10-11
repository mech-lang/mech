//! Fresh CPU-only driver for the archived EKF equations.
//!
//! Input: three contiguous little-endian f32 vectors (v, omega, bearing), each
//! of length `--instances`. Output: state (N, 3), then covariance (N, 3, 3),
//! both in C/row-major order. File I/O, parsing, compilation, preparation,
//! warm-up, fresh-state recreation, checksum, and output are outside timing.

use std::{collections::BTreeMap, env, fs, io::Write, path::PathBuf, time::Instant};

use mech_core::{Body, CellSlotId, MechCode, Program, Section, SectionElement};
use mech_gpu::ComputeLowerer;
use mech_runtime::{RuntimeBuilder, RuntimeHostInputValue};

const SOURCE: &str = include_str!("../../../../hosts/gpu/fixtures/ekf-kernel.mec");
const INPUT_NAMES: [&str; 7] = [
    "dt",
    "linear-velocity",
    "angular-velocity",
    "bearing",
    "measurement-noise",
    "finite-limit",
    "covariance-symmetry-tolerance",
];

struct Options {
    backend: String,
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
                [
                    "--backend",
                    "--instances",
                    "--turns",
                    "--warmup",
                    "--input",
                    "--dump"
                ]
                .contains(&flag.as_str()),
                "unknown argument: {flag}"
            );
            assert!(
                values
                    .insert(flag.clone(), args.next().expect("flag needs a value"))
                    .is_none(),
                "duplicate argument: {flag}"
            );
        }
        let result = Self {
            backend: values
                .remove("--backend")
                .unwrap_or_else(|| "scalar".into()),
            instances: values
                .remove("--instances")
                .unwrap_or_else(|| "10000".into())
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
        assert!(matches!(result.backend.as_str(), "scalar" | "jit"));
        assert!(result.instances > 0 && result.instances <= u32::MAX as usize);
        assert!(result.turns > 0);
        result
    }
}

fn compute_tree() -> Program {
    let tree = mech_syntax::parse(SOURCE).expect("archived EKF source parses");
    let imports = tree
        .body
        .sections
        .iter()
        .flat_map(|section| &section.elements)
        .filter_map(|element| {
            let SectionElement::MechCode(code) = element else {
                return None;
            };
            let imports = code
                .iter()
                .filter(|(code, _)| matches!(code, MechCode::Import(_)))
                .cloned()
                .collect::<Vec<_>>();
            (!imports.is_empty()).then_some(SectionElement::MechCode(imports))
        })
        .collect();
    let compute = tree
        .body
        .sections
        .iter()
        .find(|section| !section.annotations.is_empty())
        .expect("EKF has a compute section")
        .clone();
    Program {
        title: tree.title,
        body: Body {
            sections: vec![
                Section {
                    subtitle: None,
                    annotations: Vec::new(),
                    elements: imports,
                },
                compute,
            ],
        },
    }
}

fn inputs(options: &Options) -> BTreeMap<String, Vec<f32>> {
    let bytes = fs::read(&options.input).expect("read shared input file");
    assert_eq!(
        bytes.len(),
        options.instances.checked_mul(12).unwrap(),
        "input has wrong length"
    );
    let values = bytes
        .chunks_exact(4)
        .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert!(
        values.iter().all(|v| v.is_finite()),
        "inputs must be finite"
    );
    let n = options.instances;
    BTreeMap::from([
        ("dt".into(), vec![0.1]),
        ("linear-velocity".into(), values[..n].to_vec()),
        ("angular-velocity".into(), values[n..2 * n].to_vec()),
        ("bearing".into(), values[2 * n..].to_vec()),
        ("measurement-noise".into(), vec![0.25]),
        ("finite-limit".into(), vec![f32::MAX]),
        ("covariance-symmetry-tolerance".into(), vec![0.0001]),
    ])
}

fn main() {
    let options = Options::parse();
    let inputs = inputs(&options);
    let compile_started = Instant::now();
    let scalar_inputs = inputs
        .iter()
        .map(|(name, values)| {
            let name = match name.as_str() {
                "finite-limit" | "covariance-symmetry-tolerance" => name.clone(),
                _ => format!("lane-{name}"),
            };
            (name, RuntimeHostInputValue::F32(values[0]))
        })
        .collect();
    let artifact = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("build source compiler")
        .compile_tree_artifact_with_inputs(
            &compute_tree(),
            &scalar_inputs,
            &INPUT_NAMES.into_iter().map(str::to_owned).collect(),
        )
        .expect("compile source artifact")
        .into_artifact();
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .expect("lower broadcast artifact")
        .without_integrity_constraints()
        .expect("remove integrity checks for historical unchecked comparison");
    assert_eq!(program.instances() as usize, options.instances);
    assert_eq!(program.integrity_constraints().count(), 0);
    let compilation_seconds = compile_started.elapsed().as_secs_f64();
    let preparation_started = Instant::now();
    let (elapsed_seconds, state) = match options.backend.as_str() {
        "scalar" => {
            if options.warmup > 0 {
                let mut warmup = program.prepare_cpu(&inputs).unwrap();
                warmup.dispatch_turns(options.warmup).unwrap();
                std::hint::black_box(warmup.state());
            }
            let mut session = program.prepare_cpu(&inputs).unwrap();
            let started = Instant::now();
            session.dispatch_turns(options.turns).unwrap();
            let elapsed = started.elapsed().as_secs_f64();
            (elapsed, session.state().clone())
        }
        "jit" => {
            if options.warmup > 0 {
                let mut warmup = program.prepare_jit_cpu(&inputs).unwrap();
                warmup.dispatch_turns(options.warmup).unwrap();
                std::hint::black_box(warmup.state());
            }
            let mut session = program.prepare_jit_cpu(&inputs).unwrap();
            let started = Instant::now();
            session.dispatch_turns(options.turns).unwrap();
            let elapsed = started.elapsed().as_secs_f64();
            (elapsed, session.state().clone())
        }
        _ => unreachable!(),
    };
    let preparation_and_warmup_seconds =
        preparation_started.elapsed().as_secs_f64() - elapsed_seconds;
    let (pose, covariance_columns) = state_arrays(&state, options.instances);
    let covariance = row_major_covariances(covariance_columns);
    assert!(
        pose.iter()
            .chain(&covariance)
            .all(|value| value.is_finite())
    );
    if let Some(path) = &options.dump {
        let mut file = fs::File::create(path).expect("create state dump");
        for value in pose.iter().chain(&covariance) {
            file.write_all(&value.to_le_bytes())
                .expect("write state dump");
        }
    }
    let checksum = pose
        .iter()
        .chain(&covariance)
        .map(|&v| f64::from(v))
        .sum::<f64>();
    println!(
        "{}",
        serde_json::json!({
            "implementation": format!("mech-{}", options.backend),
            "instances": options.instances,
            "turns": options.turns,
            "warmup_turns": options.warmup,
            "reset_after_warmup": true,
            "checked": false,
            "workers": 1,
            "elapsed_s": elapsed_seconds,
            "filter_turns_per_second": options.instances as f64 * options.turns as f64 / elapsed_seconds,
            "compilation_seconds": compilation_seconds,
            "preparation_and_warmup_seconds": preparation_and_warmup_seconds,
            "checksum": checksum,
            "first_state": &pose[..3],
            "first_covariance": &covariance[..9],
            "dump_layout": "little-endian f32: C-order state[N,3], then covariance[N,3,3]",
            "timing": "single dispatch_turns call; per-turn candidate publication; JIT includes final AoS snapshot refresh; excludes compilation, preparation, warmup, reset, file I/O, checksum, and reporting"
        })
    );
}

fn state_arrays(state: &BTreeMap<CellSlotId, Vec<f32>>, instances: usize) -> (&[f32], &[f32]) {
    assert_eq!(state.len(), 2, "EKF has two persistent states");
    let pose = state
        .values()
        .find(|values| values.len() == instances * 3)
        .expect("pose has three components per filter");
    let covariance = state
        .values()
        .find(|values| values.len() == instances * 9)
        .expect("covariance has nine components per filter");
    (pose, covariance)
}

fn row_major_covariances(column_major: &[f32]) -> Vec<f32> {
    assert_eq!(column_major.len() % 9, 0);
    let mut row_major = vec![0.0; column_major.len()];
    for (source, destination) in column_major
        .chunks_exact(9)
        .zip(row_major.chunks_exact_mut(9))
    {
        for row in 0..3 {
            for column in 0..3 {
                destination[row * 3 + column] = source[column * 3 + row];
            }
        }
    }
    row_major
}

#[test]
fn covariance_dump_transposes_each_lane_not_the_batch() {
    let columns = [
        1., 4., 7., 2., 5., 8., 3., 6., 9., 11., 14., 17., 12., 15., 18., 13., 16., 19.,
    ];
    assert_eq!(
        row_major_covariances(&columns),
        [
            1., 2., 3., 4., 5., 6., 7., 8., 9., 11., 12., 13., 14., 15., 16., 17., 18., 19.
        ]
    );
}
