use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use mech_core::{
    LegacyValue, MResult, MechFunctionImpl, MechMap, MechRecord, MechSet, MechTuple, Ref, ToMatrix,
    hash_str,
};
use mech_engine::Interpreter;
use mech_engine::{MechProgram, MechProgramConfig};
use std::hint::black_box;

const NESTED_MATRIX_SIDE: usize = 8;
const MUTATION_MATRIX_SIDE: usize = 64;

struct BenchNode {
    out: Ref<f64>,
}

impl MechFunctionImpl for BenchNode {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::F64(self.out.clone())
    }
    fn to_string(&self) -> String {
        "ProgramCheckpointBenchNode".to_string()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "compiler")]
impl mech_core::MechFunctionCompiler for BenchNode {
    fn compile(
        &self,
        _ctx: &mut dyn mech_core::BytecodeCompilerContext,
    ) -> mech_core::MResult<mech_core::Register> {
        Ok(0)
    }
}

fn insert_symbol(interpreter: &Interpreter, name: &str, value: LegacyValue) {
    let id = hash_str(name);
    let symbols = interpreter.symbols();
    {
        let mut symbols = symbols.borrow_mut();
        symbols.insert(id, value, true);
        symbols.dictionary.borrow_mut().insert(id, name.to_string());
    }
    interpreter
        .dictionary()
        .borrow_mut()
        .insert(id, name.to_string());
}

fn append_plan_nodes(interpreter: &Interpreter, count: usize) {
    let plan = interpreter.plan();
    let mut plan = plan.borrow_mut();
    for index in 0..count {
        plan.push(Box::new(BenchNode {
            out: Ref::new(index as f64),
        }));
    }
}

fn empty_program() -> MechProgram {
    MechProgram::new(MechProgramConfig::default())
}

fn small_scalar_program() -> MechProgram {
    let mut program = empty_program();
    program
        .run_string(
            "x := 1.0
y := x + 2.0",
        )
        .unwrap();
    program
}

fn plan_program(node_count: usize) -> MechProgram {
    let program = empty_program();
    append_plan_nodes(program.interpreter(), node_count);
    program
}

fn plan_100_program() -> MechProgram {
    plan_program(100)
}

fn plan_1000_program() -> MechProgram {
    plan_program(1_000)
}

fn nested_containers_program() -> MechProgram {
    let program = empty_program();
    let shared = Ref::new(1.0);
    let map = LegacyValue::Map(Ref::new(MechMap::from_vec(vec![
        (LegacyValue::Id(1), LegacyValue::F64(shared.clone())),
        (LegacyValue::Id(2), LegacyValue::F64(Ref::new(2.0))),
    ])));
    let set = LegacyValue::Set(Ref::new(MechSet::from_vec(vec![
        LegacyValue::Id(10),
        LegacyValue::Id(20),
        LegacyValue::Id(30),
    ])));
    let tuple = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![
        LegacyValue::F64(shared.clone()),
        map,
        set,
    ])));
    let matrix_elements = (0..NESTED_MATRIX_SIDE * NESTED_MATRIX_SIDE)
        .map(|index| {
            if index % 8 == 0 {
                LegacyValue::F64(shared.clone())
            } else {
                LegacyValue::F64(Ref::new(index as f64))
            }
        })
        .collect();
    let value_matrix = LegacyValue::MatrixValue(<LegacyValue as ToMatrix>::to_matrixd(
        matrix_elements,
        NESTED_MATRIX_SIDE,
        NESTED_MATRIX_SIDE,
    ));
    let root = LegacyValue::Record(Ref::new(MechRecord::new(vec![
        ("shared", LegacyValue::F64(shared)),
        ("tuple", tuple),
        ("matrix", value_matrix),
    ])));
    insert_symbol(program.interpreter(), "nested", root);
    program
}

fn recursive_interpreters_program() -> MechProgram {
    let program = empty_program();
    let child = Interpreter::new(101, 10_000);
    let grandchild = Interpreter::new(202, 10_000);

    insert_symbol(&child, "child_value", LegacyValue::F64(Ref::new(101.0)));
    insert_symbol(
        &grandchild,
        "grandchild_value",
        LegacyValue::F64(Ref::new(202.0)),
    );
    append_plan_nodes(&child, 8);
    append_plan_nodes(&grandchild, 8);

    child
        .sub_interpreters
        .borrow_mut()
        .insert(grandchild.id, Ref::new(Box::new(grandchild)));
    program
        .interpreter()
        .sub_interpreters
        .borrow_mut()
        .insert(child.id, Ref::new(Box::new(child)));
    program
}

fn structural_additions_program() -> MechProgram {
    let program = empty_program();
    append_plan_nodes(program.interpreter(), 16);
    insert_symbol(
        program.interpreter(),
        "retained",
        LegacyValue::F64(Ref::new(1.0)),
    );
    program
}

fn scalar_matrix_program() -> MechProgram {
    let program = empty_program();
    insert_symbol(
        program.interpreter(),
        "checkpoint_scalar",
        LegacyValue::F64(Ref::new(1.0)),
    );
    insert_symbol(
        program.interpreter(),
        "checkpoint_matrix",
        LegacyValue::MatrixF64(<f64 as ToMatrix>::to_matrixd(
            vec![1.0; MUTATION_MATRIX_SIDE * MUTATION_MATRIX_SIDE],
            MUTATION_MATRIX_SIDE,
            MUTATION_MATRIX_SIDE,
        )),
    );
    program
}

fn no_mutation(_program: &mut MechProgram) {}

fn add_structures(program: &mut MechProgram) {
    append_plan_nodes(program.interpreter(), 64);
    insert_symbol(
        program.interpreter(),
        "temporary",
        LegacyValue::F64(Ref::new(99.0)),
    );
    program.interpreter().out_values.borrow_mut().insert(
        hash_str("temporary-output"),
        LegacyValue::F64(Ref::new(100.0)),
    );
    let child = Interpreter::new(303, 10_000);
    program
        .interpreter()
        .sub_interpreters
        .borrow_mut()
        .insert(child.id, Ref::new(Box::new(child)));
    program.config.name = "mutated-after-checkpoint".to_string();
}

fn mutate_scalar_and_matrix(program: &mut MechProgram) {
    let symbols = program.interpreter().symbols();
    let scalar = symbols
        .borrow()
        .get(hash_str("checkpoint_scalar"))
        .unwrap()
        .clone();
    let matrix = symbols
        .borrow()
        .get(hash_str("checkpoint_matrix"))
        .unwrap()
        .clone();

    match &*scalar.borrow() {
        LegacyValue::F64(value) => *value.borrow_mut() = 10_000.0,
        other => panic!("expected scalar benchmark value, got {other:?}"),
    }
    match &*matrix.borrow() {
        LegacyValue::MatrixF64(value) => {
            value.set(vec![10_000.0; MUTATION_MATRIX_SIDE * MUTATION_MATRIX_SIDE]);
        }
        other => panic!("expected matrix benchmark value, got {other:?}"),
    }
}

type ProgramFixture = fn() -> MechProgram;
type ProgramMutation = fn(&mut MechProgram);

fn batch_size(large_input: bool) -> BatchSize {
    if large_input {
        BatchSize::LargeInput
    } else {
        BatchSize::SmallInput
    }
}

fn benchmark_family(
    c: &mut Criterion,
    name: &str,
    fixture: ProgramFixture,
    mutation: ProgramMutation,
    large_input: bool,
    throughput: Option<u64>,
) {
    let mut group = c.benchmark_group(format!("program_checkpoint/{name}"));
    if let Some(elements) = throughput {
        group.throughput(Throughput::Elements(elements));
    }

    group.bench_function("capture", |b| {
        b.iter_batched_ref(
            fixture,
            |program| black_box(program.checkpoint().unwrap()),
            batch_size(large_input),
        )
    });

    group.bench_function("restore", |b| {
        b.iter_batched(
            || {
                let mut program = fixture();
                let checkpoint = program.checkpoint().unwrap();
                mutation(&mut program);
                (program, checkpoint)
            },
            |(mut program, checkpoint)| {
                program.restore(checkpoint).unwrap();
                black_box(program)
            },
            batch_size(large_input),
        )
    });

    group.finish();
}

fn program_checkpoint_benchmarks(c: &mut Criterion) {
    benchmark_family(c, "empty_program", empty_program, no_mutation, false, None);
    benchmark_family(
        c,
        "small_scalar",
        small_scalar_program,
        no_mutation,
        false,
        None,
    );
    benchmark_family(
        c,
        "plan_100_nodes",
        plan_100_program,
        no_mutation,
        false,
        Some(100),
    );
    benchmark_family(
        c,
        "plan_1000_nodes",
        plan_1000_program,
        no_mutation,
        true,
        Some(1_000),
    );
    benchmark_family(
        c,
        "nested_containers",
        nested_containers_program,
        no_mutation,
        false,
        None,
    );
    benchmark_family(
        c,
        "recursive_interpreters",
        recursive_interpreters_program,
        no_mutation,
        false,
        None,
    );
    benchmark_family(
        c,
        "structural_additions",
        structural_additions_program,
        add_structures,
        false,
        None,
    );
    benchmark_family(
        c,
        "scalar_matrix_mutation",
        scalar_matrix_program,
        mutate_scalar_and_matrix,
        true,
        None,
    );
}

criterion_group!(benches, program_checkpoint_benchmarks);
criterion_main!(benches);
