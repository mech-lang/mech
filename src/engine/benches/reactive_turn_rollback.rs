use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use mech_core::{
    BytecodeCompilerContext, GenericError, MResult, MechError, MechFunctionCompiler,
    MechFunctionImpl, NoMechExecutionServices, ReactiveCellId, ReactiveNodeKind,
    ReactiveRegisterCommit, ReactiveRegisterWrite, ReactiveSolveStatus, Ref, Register, Value,
    hash_str,
};
use mech_engine::{
    MechProgram, MechProgramConfig, ProgramInputId, ProgramInputUpdate, ProgramTurnFinalization,
};
use mech_interpreter::Interpreter;
use std::hint::black_box;

struct BenchCombinational {
    output: Ref<f64>,
    fail: bool,
}

impl MechFunctionImpl for BenchCombinational {
    fn solve(&self) {}
    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        *self.output.borrow_mut() += 1.0;
        if self.fail {
            return Err(MechError::new(
                mech_core::GenericError {
                    msg: "benchmark tail failure".into(),
                },
                None,
            ));
        }
        Ok(ReactiveSolveStatus::Changed)
    }
    fn solve_result(&self) -> MResult<()> {
        self.solve_reactive().map(|_| ())
    }
    fn out(&self) -> Value {
        Value::F64(self.output.clone())
    }
    fn to_string(&self) -> String {
        "ReactiveTurnRollbackBenchCombinational".into()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

impl MechFunctionCompiler for BenchCombinational {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

struct BenchRegister {
    source: Ref<f64>,
    sink: Ref<f64>,
}

impl MechFunctionImpl for BenchRegister {
    fn solve(&self) {}
    fn out(&self) -> Value {
        Value::F64(self.sink.clone())
    }
    fn reactive_node_kind(&self) -> ReactiveNodeKind {
        ReactiveNodeKind::Register
    }
    fn stage_register(&self) -> MResult<Box<dyn ReactiveRegisterCommit>> {
        Ok(Box::new(ReactiveRegisterWrite::new(
            self.sink.clone(),
            *self.source.borrow(),
            vec![ReactiveCellId::new(self.sink.id())],
        )))
    }
    fn to_string(&self) -> String {
        "ReactiveTurnRollbackBenchRegister".into()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}

impl MechFunctionCompiler for BenchRegister {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

fn update(input: ProgramInputId, value: f64) -> ProgramInputUpdate {
    ProgramInputUpdate {
        input,
        value: Value::F64(Ref::new(value)),
    }
}

fn root_input(program: &mut MechProgram, name: &str, value: f64) -> (ProgramInputId, Ref<f64>) {
    let id = hash_str(name);
    let input = program
        .ensure_input(
            program.interpreter().id,
            id,
            name,
            Value::F64(Ref::new(value)),
        )
        .unwrap();
    let outer = program.interpreter().symbols().borrow().get(id).unwrap();
    let inner = match &*outer.borrow() {
        Value::F64(value) => value.clone(),
        other => panic!("expected benchmark f64 input, got {other:?}"),
    };
    (input, inner)
}

fn add_combinational(interpreter: &Interpreter, input: Ref<f64>, output: Ref<f64>, fail: bool) {
    interpreter
        .plan()
        .0
        .borrow_mut()
        .register(
            Box::new(BenchCombinational { output, fail }),
            &[Value::F64(input)],
        )
        .unwrap();
}

fn chain_fixture(length: usize, fail_tail: bool) -> (MechProgram, ProgramInputId) {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let (input, mut previous) = root_input(&mut program, "input", 1.0);
    for index in 0..length {
        let output = Ref::new(index as f64);
        add_combinational(
            program.interpreter(),
            previous,
            output.clone(),
            fail_tail && index + 1 == length,
        );
        previous = output;
    }
    (program, input)
}

fn independent_fixture(length: usize) -> (MechProgram, ProgramInputId) {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let mut selected = None;
    for index in 0..length {
        let (input, source) = root_input(&mut program, &format!("input-{index}"), index as f64);
        add_combinational(program.interpreter(), source, Ref::new(index as f64), false);
        if index == length / 2 {
            selected = Some(input);
        }
    }
    (program, selected.unwrap())
}

fn register_fixture(count: usize) -> (MechProgram, ProgramInputId) {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let (input, source) = root_input(&mut program, "register-input", 1.0);
    for _ in 0..count {
        program
            .interpreter()
            .plan()
            .0
            .borrow_mut()
            .register(
                Box::new(BenchRegister {
                    source: source.clone(),
                    sink: Ref::new(0.0),
                }),
                &[Value::F64(source.clone())],
            )
            .unwrap();
    }
    (program, input)
}

fn two_interpreter_fixture() -> (MechProgram, Vec<ProgramInputUpdate>) {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let mut updates = Vec::new();
    for id in [10, 20] {
        let child = Interpreter::new(id, 10_000);
        program
            .interpreter()
            .sub_interpreters
            .borrow_mut()
            .insert(id, Ref::new(Box::new(child)));
        let input = program
            .ensure_input(id, hash_str("input"), "input", Value::F64(Ref::new(1.0)))
            .unwrap();
        let child = {
            let children = program.interpreter().sub_interpreters.borrow();
            children.get(&id).unwrap().clone()
        };
        let source = {
            let child = child.borrow();
            let outer = child.symbols().borrow().get(hash_str("input")).unwrap();
            let source = match &*outer.borrow() {
                Value::F64(value) => value.clone(),
                other => panic!("expected child benchmark input, got {other:?}"),
            };
            source
        };
        {
            let child = child.borrow();
            add_combinational(child.as_ref(), source, Ref::new(0.0), false);
        }
        updates.push(update(input, 2.0));
    }
    (program, updates)
}

fn step_failure_fixture() -> MechProgram {
    let program = MechProgram::new(MechProgramConfig::default());
    for index in 0..100 {
        program
            .interpreter()
            .plan()
            .add_function(Box::new(BenchCombinational {
                output: Ref::new(index as f64),
                fail: index == 99,
            }));
    }
    program
}

fn reactive_turn_rollback_benchmarks(c: &mut Criterion) {
    c.bench_function("reactive_turn_rollback/one_scalar_one_node", |b| {
        b.iter_batched(
            || chain_fixture(1, false),
            |(mut program, input)| {
                black_box(
                    program
                        .update_inputs_and_advance_turn(&[update(input, 2.0)])
                        .unwrap(),
                )
            },
            BatchSize::SmallInput,
        )
    });
    for length in [100usize, 1_000] {
        c.bench_function(&format!("reactive_turn_rollback/chain_{length}"), |b| {
            b.iter_batched(
                || chain_fixture(length, false),
                |(mut program, input)| {
                    black_box(
                        program
                            .update_inputs_and_advance_turn(&[update(input, 2.0)])
                            .unwrap(),
                    )
                },
                BatchSize::LargeInput,
            )
        });
    }
    c.bench_function(
        "reactive_turn_rollback/independent_1000_one_affected",
        |b| {
            b.iter_batched(
                || independent_fixture(1_000),
                |(mut program, input)| {
                    black_box(
                        program
                            .update_inputs_and_advance_turn(&[update(input, 2.0)])
                            .unwrap(),
                    )
                },
                BatchSize::LargeInput,
            )
        },
    );
    c.bench_function("reactive_turn_rollback/one_register_turn", |b| {
        b.iter_batched(
            || register_fixture(1),
            |(mut program, input)| {
                black_box(
                    program
                        .update_inputs_and_advance_turn(&[update(input, 2.0)])
                        .unwrap(),
                )
            },
            BatchSize::SmallInput,
        )
    });
    c.bench_function("reactive_turn_rollback/pending_registers_100", |b| {
        b.iter_batched(
            || register_fixture(100),
            |(mut program, input)| {
                black_box(
                    program
                        .update_inputs_and_advance_turn(&[update(input, 2.0)])
                        .unwrap(),
                )
            },
            BatchSize::LargeInput,
        )
    });
    c.bench_function("reactive_turn_rollback/two_affected_interpreters", |b| {
        b.iter_batched(
            two_interpreter_fixture,
            |(mut program, updates)| {
                black_box(program.update_inputs_and_advance_turn(&updates).unwrap())
            },
            BatchSize::SmallInput,
        )
    });
    c.bench_function("reactive_turn_rollback/failed_tail_with_rollback", |b| {
        b.iter_batched(
            || chain_fixture(100, true),
            |(mut program, input)| {
                black_box(
                    program
                        .update_inputs_and_advance_turn(&[update(input, 2.0)])
                        .unwrap_err(),
                )
            },
            BatchSize::LargeInput,
        )
    });
    c.bench_function("reactive_turn_rollback/whole_plan_step_failure", |b| {
        b.iter_batched(
            step_failure_fixture,
            |mut program| black_box(program.step(0).unwrap_err()),
            BatchSize::LargeInput,
        )
    });
    c.bench_function(
        "reactive_turn_rollback/coordinated_success_then_rollback",
        |b| {
            b.iter_batched(
                || chain_fixture(100, false),
                |(mut program, input)| {
                    let mut services = NoMechExecutionServices;
                    program
                        .update_inputs_and_advance_turn_coordinated(
                            &[update(input, 2.0)],
                            &mut services,
                            |_| {
                                ProgramTurnFinalization::Rollback(MechError::new(
                                    GenericError {
                                        msg: "benchmark rollback".into(),
                                    },
                                    None,
                                ))
                            },
                        )
                        .unwrap_err();
                    black_box(program)
                },
                BatchSize::LargeInput,
            )
        },
    );
    c.bench_function(
        "reactive_turn_rollback/full_checkpoint_independent_1000",
        |b| {
            b.iter_batched(
                || independent_fixture(1_000).0,
                |program| black_box(program.checkpoint().unwrap()),
                BatchSize::LargeInput,
            )
        },
    );
}

criterion_group!(benches, reactive_turn_rollback_benchmarks);
criterion_main!(benches);
