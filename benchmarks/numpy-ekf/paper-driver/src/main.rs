//! Existing paper numeric backends, with one worker and matched inputs.
//! Source revision and isolated dependencies are pinned by ../prepare.py.

use std::{collections::BTreeMap, env, fs, io::Write, path::PathBuf, time::Instant};

use mech_core::CellSlotId;
use mech_gpu::ComputeLowerer;
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
                .unwrap_or_else(|| "jit-simd".into()),
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
        assert!(matches!(
            result.backend.as_str(),
            "scalar" | "simd" | "jit" | "jit-simd" | "aot-simd"
        ));
        assert!(result.instances > 0 && result.instances <= u32::MAX as usize);
        if matches!(result.backend.as_str(), "jit-simd" | "aot-simd") {
            assert_eq!(
                result.instances % 4,
                0,
                "existing compiled SIMD APIs require a multiple of four filters"
            );
        }
        assert!(result.turns > 0);
        result
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
    ])
}

fn main() {
    let options = Options::parse();
    let inputs = inputs(&options);
    let compile_started = Instant::now();
    let scalar_inputs = inputs
        .iter()
        .map(|(name, values)| {
            (
                format!("lane-{name}"),
                RuntimeHostInputValue::F32(values[0]),
            )
        })
        .collect();
    let tree = mech_syntax::parse(SOURCE).expect("numerical EKF source parses");
    let artifact = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()
        .expect("build source compiler")
        .compile_tree_artifact_with_inputs(
            &tree,
            &scalar_inputs,
            &INPUT_NAMES.into_iter().map(str::to_owned).collect(),
        )
        .expect("compile source artifact")
        .into_artifact();
    let program = ComputeLowerer
        .compile_broadcast(&artifact, &inputs)
        .expect("lower broadcast artifact");
    assert_eq!(program.instances() as usize, options.instances);
    assert_eq!(program.integrity_constraints().count(), 0);
    let compilation_seconds = compile_started.elapsed().as_secs_f64();
    let preparation_started = Instant::now();
    let (elapsed_seconds, state, aot_path) = match options.backend.as_str() {
        "scalar" => {
            if options.warmup > 0 {
                let mut warmup = program.prepare_cpu(&inputs).unwrap();
                warmup.dispatch_turns(options.warmup).unwrap();
                std::hint::black_box(warmup.state());
            }
            let mut session = program.prepare_cpu(&inputs).unwrap();
            let started = Instant::now();
            session.dispatch_turns(options.turns).unwrap();
            std::hint::black_box(session.state());
            let elapsed = started.elapsed().as_secs_f64();
            (elapsed, session.state().clone(), None)
        }
        "simd" => {
            if options.warmup > 0 {
                let mut warmup = program.prepare_simd_cpu(&inputs).unwrap();
                warmup.dispatch_turns(options.warmup).unwrap();
                std::hint::black_box(warmup.state());
            }
            let mut session = program.prepare_simd_cpu(&inputs).unwrap();
            let started = Instant::now();
            session.dispatch_turns(options.turns).unwrap();
            std::hint::black_box(session.state());
            let elapsed = started.elapsed().as_secs_f64();
            (elapsed, session.state().clone(), None)
        }
        "jit" => {
            let artifact = program
                .compile_jit_cpu()
                .expect("compile scalar JIT kernel");
            if options.warmup > 0 {
                let mut warmup = artifact.prepare(&inputs).unwrap();
                warmup.dispatch_turns(options.warmup).unwrap();
                std::hint::black_box(warmup.state());
            }
            let mut session = artifact.prepare(&inputs).unwrap();
            let started = Instant::now();
            session.dispatch_turns(options.turns).unwrap();
            std::hint::black_box(session.state());
            let elapsed = started.elapsed().as_secs_f64();
            (elapsed, session.state().clone(), None)
        }
        "jit-simd" => {
            if options.warmup > 0 {
                let mut warmup = program.prepare_jit_simd_cpu(&inputs).unwrap();
                warmup.dispatch_turns(options.warmup).unwrap();
                std::hint::black_box(warmup.state());
            }
            let mut session = program.prepare_jit_simd_cpu(&inputs).unwrap();
            let started = Instant::now();
            session.dispatch_turns(options.turns).unwrap();
            // This public API materializes the lazy host-facing state. Include
            // it so SIMD JIT does not get a cheaper endpoint than SIMD AOT.
            std::hint::black_box(session.state());
            let elapsed = started.elapsed().as_secs_f64();
            (elapsed, session.state().clone(), None)
        }
        "aot-simd" => {
            let artifact = program
                .compile_aot_simd_cpu()
                .expect("compile SIMD AOT library");
            if options.warmup > 0 {
                let mut warmup = artifact.prepare(&inputs).unwrap();
                warmup.dispatch_turns(options.warmup).unwrap();
                std::hint::black_box(warmup.state());
            }
            let mut session = artifact.prepare(&inputs).unwrap();
            let started = Instant::now();
            // dispatch_turns includes final unpacking to host-facing state.
            session.dispatch_turns(options.turns).unwrap();
            std::hint::black_box(session.state());
            let elapsed = started.elapsed().as_secs_f64();
            (
                elapsed,
                session.state().clone(),
                Some(artifact.path().to_path_buf()),
            )
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
            "backend_revision": REVISION,
            "instances": options.instances,
            "turns": options.turns,
            "warmup_turns": options.warmup,
            "reset_after_warmup": true,
            "checked": false,
            "workers": 1,
            "simd_lanes": if matches!(options.backend.as_str(), "simd" | "jit-simd" | "aot-simd") { 4 } else { 1 },
            "elapsed_s": elapsed_seconds,
            "filter_turns_per_second": options.instances as f64 * options.turns as f64 / elapsed_seconds,
            "compilation_seconds": compilation_seconds,
            "preparation_and_warmup_seconds": preparation_and_warmup_seconds,
            "aot_library": aot_path,
            "checksum": checksum,
            "first_state": &pose[..3],
            "first_covariance": &covariance[..9],
            "dump_layout": "little-endian f32: C-order state[N,3], then covariance[N,3,3]",
            "timing": "single-worker dispatch_turns; per-turn state publication plus final host-readable state materialization; excludes compilation, preparation, warmup, reset, file I/O, checksum, and reporting"
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
    assert_eq!(
        row_major_covariances(&[
            1., 4., 7., 2., 5., 8., 3., 6., 9., 11., 14., 17., 12., 15., 18., 13., 16., 19.,
        ]),
        [
            1., 2., 3., 4., 5., 6., 7., 8., 9., 11., 12., 13., 14., 15., 16., 17., 18., 19.
        ]
    );
}
