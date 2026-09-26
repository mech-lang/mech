use mech::kernel::{Backend, Error, Kernel, KernelBuilder};
use mech_gpu::BatchedExecutionError;

const COUNTER: &str = r#"
Counter @compute
----------------
increment := 1f32
~state := 0f32
candidate := state + increment
nonnegative! := candidate >= 0f32
state = candidate
"#;

fn backends() -> Vec<Backend> {
    vec![
        Backend::Scalar,
        Backend::Simd,
        #[cfg(feature = "kernel-jit")]
        Backend::Jit,
        #[cfg(feature = "kernel-aot")]
        Backend::Aot,
        #[cfg(feature = "kernel-aot")]
        Backend::AotSimd,
    ]
}

fn compile_counter(builder: KernelBuilder, backend: Backend) -> Kernel {
    builder
        .input("increment", [1.0; 4])
        .export("state")
        .compile(backend)
        .unwrap()
}

#[test]
fn inline_source_matches_builder_and_completes_turns_synchronously() {
    for backend in backends() {
        // Keep this as an inline multiline literal: newlines and the `~` and `!`
        // operators must reach the Mech parser without Rust token stringification.
        let embedded = compile_counter(
            mech::mech!(
                r#"
Counter @compute
----------------
increment := 1f32
~state := 0f32
candidate := state + increment
nonnegative! := candidate >= 0f32
state = candidate
"#
            ),
            backend,
        );
        let direct = compile_counter(Kernel::from_source(COUNTER), backend);
        let mut embedded = embedded.start().unwrap();
        let mut direct = direct.start().unwrap();

        assert_eq!(embedded.state("state").unwrap(), &[0.0; 4]);
        for (increment, expected) in [(3.0, 3.0), (5.0, 8.0)] {
            embedded.turn([("increment", [increment])]).unwrap();
            direct.turn([("increment", [increment])]).unwrap();
            assert_eq!(embedded.state("state").unwrap(), &[expected; 4]);
            assert_eq!(
                embedded.state("state").unwrap(),
                direct.state("state").unwrap()
            );
        }

        assert!(matches!(
            embedded.turn([("increment", [-9.0])]),
            Err(Error::Execution(BatchedExecutionError::Integrity(_)))
        ));
        assert!(matches!(
            direct.turn([("increment", [-9.0])]),
            Err(Error::Execution(BatchedExecutionError::Integrity(_)))
        ));
        assert_eq!(embedded.state("state").unwrap(), &[8.0; 4]);
        assert_eq!(
            embedded.state("state").unwrap(),
            direct.state("state").unwrap()
        );

        embedded.turn([("increment", [2.0])]).unwrap();
        direct.turn([("increment", [2.0])]).unwrap();
        assert_eq!(embedded.state("state").unwrap(), &[10.0; 4]);
        assert_eq!(
            embedded.state("state").unwrap(),
            direct.state("state").unwrap()
        );
    }
}

#[test]
fn include_str_matches_explicit_file_builder() {
    let compile = |builder: KernelBuilder| {
        builder
            .input("bearing", [-0.55; 4])
            .export("state")
            .compile(Backend::Scalar)
            .unwrap()
    };
    let embedded = compile(mech::mech!(include_str!(
        "../examples/embedded_ekf/ekf.mec"
    ),));
    let direct = compile(Kernel::from_source(include_str!(
        "../examples/embedded_ekf/ekf.mec"
    )));
    let mut embedded = embedded.start().unwrap();
    let mut direct = direct.start().unwrap();
    for bearing in [-0.54, -0.53, -0.52] {
        embedded.turn([("bearing", [bearing; 4])]).unwrap();
        direct.turn([("bearing", [bearing; 4])]).unwrap();
        assert_eq!(
            embedded.state("state").unwrap(),
            direct.state("state").unwrap()
        );
    }
}

#[test]
fn source_expression_is_evaluated_once_and_compilation_is_explicit() {
    let mut evaluations = 0;
    let builder: KernelBuilder = mech::mech!({
        evaluations += 1;
        String::from("this is not a self-contained compute section")
    },);
    assert_eq!(evaluations, 1);
    assert!(matches!(
        builder.compile(Backend::Scalar),
        Err(Error::Source(_))
    ));
}

#[test]
fn macro_is_callable_through_aliases_with_shadowed_names() {
    use embedding::mech as inline;
    use mech as embedding;

    // Local names must not affect the macro's expansion.
    struct Kernel;
    let _ = Kernel;
    let builder: embedding::kernel::KernelBuilder = inline!(COUNTER);
    let compiled = compile_counter(builder, Backend::Scalar);
    assert_eq!(compiled.instances(), 4);
}
