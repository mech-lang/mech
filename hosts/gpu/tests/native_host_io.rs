//! One compiled arithmetic graph with explicit secure and trusted host policies.
#![cfg(feature = "native-io")]

use mech_core::MResult;
use mech_gpu::sequence::{
    CliHostIo, Context, JitProgram, NativeHostIo, NativeIoGrant, NativeIoPolicy,
    NativeResourceIntent, NativeResourceRequirement, compile,
};
use mech_terminal::CliBackend;
use std::{
    ffi::OsString,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
struct State {
    reads: usize,
    stdin_reads: usize,
    output: String,
}

#[derive(Clone, Debug, Default)]
struct Backend(Arc<Mutex<State>>);

impl CliBackend for Backend {
    fn stdin_text(&self) -> MResult<String> {
        self.0.lock().unwrap().stdin_reads += 1;
        Ok("héllo e\u{301} 👋\n\nlast\r\n".into())
    }

    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn argument(&self, index: usize) -> MResult<Option<OsString>> {
        self.0.lock().unwrap().reads += 1;
        Ok(match index {
            1 => Some("2".into()),
            2 => Some("suffix".into()),
            _ => None,
        })
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.0.lock().unwrap().output.push_str(text);
        Ok(())
    }
    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        unreachable!()
    }
}

fn grants() -> Vec<NativeIoGrant> {
    vec![
        NativeIoGrant {
            target: "cli/args".into(),
            operations: vec!["read".into()],
            paths: vec!["1".into(), "2".into()],
        },
        NativeIoGrant {
            target: "cli/stdout".into(),
            operations: vec!["write".into()],
            paths: vec!["line".into()],
        },
    ]
}

const SOURCE: &str = "+> @args := cli/args\n+> @out := cli/stdout\nstep(x<u64>) => <u64> := x + 3\nn := number/parse-u64(@args/1)\nr := n |19> step\n@out/line <- \"result: \" + number/format(r, 0) + \" \" + @args/2\n";

fn stdin_grants() -> Vec<NativeIoGrant> {
    vec![
        NativeIoGrant {
            target: "cli/stdin".into(),
            operations: vec!["read".into()],
            paths: vec!["text".into()],
        },
        NativeIoGrant {
            target: "cli/stdout".into(),
            operations: vec!["write".into()],
            paths: vec!["text".into()],
        },
    ]
}

#[test]
fn stdin_snapshot_is_shared_by_secure_trusted_jit_and_resident_routes() {
    let source = "+> @in := cli/stdin\n+> @out := cli/stdout\na := @in/text\nb := @in/text\n@out/text <- a + \"|\" + b\n";
    let program = compile(source, 256).unwrap();
    let requirements = program.resource_requirements().unwrap();
    for resident in [false, true] {
        for trusted in [false, true] {
            let backend = Backend::default();
            let policy = if trusted {
                NativeIoPolicy::Trusted
            } else {
                NativeIoPolicy::Secure {
                    grants: stdin_grants(),
                }
            };
            let host = CliHostIo::activate(&requirements, policy, backend.clone()).unwrap();
            assert_eq!(backend.0.lock().unwrap().stdin_reads, 0);
            let mut context = Context::activated(&program.constants, Box::new(host)).unwrap();
            if resident {
                context.run_resident(&program).unwrap();
            } else {
                JitProgram::compile(&program, 256, true)
                    .unwrap()
                    .run(&mut context)
                    .unwrap();
            }
            let state = backend.0.lock().unwrap();
            assert_eq!(state.stdin_reads, 1);
            assert_eq!(state.reads, 0);
            let text = "héllo e\u{301} 👋\n\nlast\r\n";
            assert_eq!(state.output, format!("{text}|{text}"));
        }
    }
    for grants in [
        Vec::new(),
        vec![stdin_grants()[1].clone()],
        vec![
            NativeIoGrant {
                paths: vec!["line".into()],
                ..stdin_grants()[0].clone()
            },
            stdin_grants()[1].clone(),
        ],
        vec![
            NativeIoGrant {
                operations: vec!["write".into()],
                ..stdin_grants()[0].clone()
            },
            stdin_grants()[1].clone(),
        ],
    ] {
        let backend = Backend::default();
        assert!(
            CliHostIo::activate(
                &requirements,
                NativeIoPolicy::Secure { grants },
                backend.clone()
            )
            .is_err()
        );
        let state = backend.0.lock().unwrap();
        assert_eq!(state.stdin_reads, 0);
        assert!(state.output.is_empty());
    }
}

#[test]
fn one_native_program_has_equal_secure_and_trusted_outputs() {
    let program = compile(SOURCE, 256).unwrap();
    assert!(!program.object(256).unwrap().is_empty());
    let requirements = program.resource_requirements().unwrap();
    for resident in [false, true] {
        for trusted in [false, true] {
            let backend = Backend::default();
            let policy = if trusted {
                NativeIoPolicy::Trusted
            } else {
                NativeIoPolicy::Secure { grants: grants() }
            };
            let host = CliHostIo::activate(&requirements, policy, backend.clone()).unwrap();
            assert_eq!(
                backend.0.lock().unwrap().reads,
                0,
                "activation observed argv"
            );
            let mut context = Context::activated(&program.constants, Box::new(host)).unwrap();
            if resident {
                context.run_resident(&program).unwrap();
            } else {
                JitProgram::compile(&program, 256, true)
                    .unwrap()
                    .run(&mut context)
                    .unwrap();
            }
            let state = backend.0.lock().unwrap();
            assert_eq!(state.reads, 2);
            assert_eq!(state.output, "result: 59 suffix\n");
        }
    }
}

#[test]
fn every_port_is_authorized_before_any_observation_or_output() {
    let program = compile(SOURCE, 256).unwrap();
    for allowed in [
        Vec::new(),
        vec![grants()[0].clone()],
        vec![grants()[1].clone()],
    ] {
        let backend = Backend::default();
        let result = CliHostIo::activate(
            &program.resource_requirements().unwrap(),
            NativeIoPolicy::Secure { grants: allowed },
            backend.clone(),
        );
        assert!(result.is_err());
        let state = backend.0.lock().unwrap();
        assert_eq!(state.reads, 0);
        assert!(state.output.is_empty());
    }
}

#[test]
fn activated_context_never_falls_back_to_legacy_process_io() {
    for source in [
        "io/write(\"bypass\")\n",
        "n := io/argument-u64(1)\n",
        "x := io/read()\n",
    ] {
        let program = compile(source, 256).unwrap();
        for resident in [false, true] {
            let backend = Backend::default();
            let host = CliHostIo::activate(&[], NativeIoPolicy::Trusted, backend.clone()).unwrap();
            let mut context = Context::activated(&program.constants, Box::new(host)).unwrap();
            let result = if resident {
                context.run_resident(&program)
            } else {
                JitProgram::compile(&program, 256, false)
                    .unwrap()
                    .run(&mut context)
            };
            assert!(
                result.unwrap_err().contains("legacy process IO"),
                "{source}"
            );
            let state = backend.0.lock().unwrap();
            assert_eq!(state.reads, 0);
            assert!(state.output.is_empty());
        }
    }
}

#[test]
fn trusted_ports_still_reject_unbound_identity_and_use_after_shutdown() {
    let program = compile(SOURCE, 256).unwrap();
    let requirements = program.resource_requirements().unwrap();
    let backend = Backend::default();
    let mut host =
        CliHostIo::activate(&requirements, NativeIoPolicy::Trusted, backend.clone()).unwrap();
    let unbound = NativeResourceRequirement {
        base_uri: "cli://cli/args".into(),
        context_name: "args".into(),
        path: "3".into(),
        intent: NativeResourceIntent::Read,
    };
    assert!(host.read(&unbound).is_err());
    assert_eq!(backend.0.lock().unwrap().reads, 0);
    host.finish(true).unwrap();
    assert!(host.read(&requirements[0]).is_err());
}

#[test]
fn secure_execution_publishes_effects_only_after_all_computation_succeeds() {
    let source =
        "+> @out := cli/stdout\n@out/line <- \"staged\"\nn := number/parse-u64(\"invalid\")\n";
    let program = compile(source, 256).unwrap();
    for resident in [false, true] {
        let backend = Backend::default();
        let host = CliHostIo::activate(
            &program.resource_requirements().unwrap(),
            NativeIoPolicy::Secure { grants: grants() },
            backend.clone(),
        )
        .unwrap();
        let mut context = Context::activated(&program.constants, Box::new(host)).unwrap();
        let result = if resident {
            context.run_resident(&program)
        } else {
            JitProgram::compile(&program, 256, false)
                .unwrap()
                .run(&mut context)
        };
        assert!(result.unwrap_err().contains("number/parse-u64"));
        assert!(backend.0.lock().unwrap().output.is_empty());
    }
}
