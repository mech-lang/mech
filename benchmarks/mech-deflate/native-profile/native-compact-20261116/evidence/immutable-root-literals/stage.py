"""Prepare an audited entry-literal path for the checked native root."""

from difflib import unified_diff
from hashlib import sha256
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
ROOT = Q.parent
BASE = 'ed99ade92836761bebd2f7357bf46817204433d4'
CHECKOUT = Q / 'native-review-pr/checkout'
GPU = Path('hosts/gpu/src/native_artifact')
WORDS = GPU / 'codegen/common/ssa_words.rs'
WORD_TESTS = GPU / 'codegen/common/ssa_words_tests.rs'
EMIT = GPU / 'program/compact/emit.rs'
COMPILER_GLUE = GPU / 'program/compiler.rs'
PACKAGE = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')


def replace(text, before, after, count=1):
    assert text.count(before) == count, (text.count(before), before)
    return text.replace(before, after)


def git(*args):
    return subprocess.check_output(['git', *map(str, args)], cwd=ROOT)


def words(text):
    text = replace(text, 'use std::cell::RefCell;', 'use std::cell::{Cell, RefCell};')
    text = replace(text, '''pub(in crate::native_artifact) struct Words {
    variables: BTreeMap<usize, (Variable, MemFlags)>,
''', '''#[derive(Clone, Copy)]
enum Home {
    Variable(Variable),
    Literal(Value),
}

pub(in crate::native_artifact) struct Words {
    variables: BTreeMap<usize, (Home, MemFlags)>,
    literal_write: Cell<bool>,
''')
    text = replace(text, 'Self::build(b, locals, code, slots, initial, flags)',
                   'Self::build(b, locals, code, slots, initial, flags, None)', 2)
    text = replace(text, 'Self::build(b, locals, code, slots, initial, |_| MemFlags::new())',
                   'Self::build(b, locals, code, slots, initial, |_| MemFlags::new(), None)')
    text = replace(text, 'Self::build(b, locals, code, slots, |_, _| Ok(None), flags)',
                   'Self::build(b, locals, code, slots, |_, _| Ok(None), flags, None)')
    text = replace(text, '''    fn build(
''', '''    /// Seed selected immutable literal lanes in the dominating entry block.
    /// The enclosing compiler discards this attempt if lowering writes any
    /// selected lane, or if the usual IR checks reject the result.
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn new_with_entry_literals(
        b: &mut FunctionBuilder<'_>,
        locals: Value,
        code: CodePlan<'_>,
        slots: BTreeSet<usize>,
        initial: impl FnMut(
            &mut FunctionBuilder<'_>,
            usize,
        ) -> Result<Option<Value>, NativeCompileError>,
        literals: &BTreeSet<usize>,
    ) -> Result<Self, NativeCompileError> {
        Self::build(b, locals, code, slots, initial, |_| MemFlags::new(), Some(literals))
    }
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn attempted_literal_write(&self) -> bool {
        self.literal_write.get()
    }

    fn build(
''')
    text = replace(text, '''        mut flags: impl FnMut(usize) -> MemFlags,
    ) -> Result<Self, NativeCompileError>''', '''        mut flags: impl FnMut(usize) -> MemFlags,
        literals: Option<&BTreeSet<usize>>,
    ) -> Result<Self, NativeCompileError>''')
    text = replace(text, '''        for slot in slots {
            let variable = b.declare_var(types::I64);
            let flags = flags(slot);''', '''        for slot in slots {
            let is_literal = literals.is_some_and(|literals| literals.contains(&slot));
            let variable = (!is_literal).then(|| b.declare_var(types::I64));
            let flags = flags(slot);''')
    text = replace(text, '''            b.def_var(variable, value);
            variables.insert(slot, (variable, flags));
        }
        Ok(Self {
            variables,
''', '''            let home = if let Some(variable) = variable {
                b.def_var(variable, value);
                Home::Variable(variable)
            } else {
                Home::Literal(value)
            };
            variables.insert(slot, (home, flags));
        }
        Ok(Self {
            variables,
            literal_write: Cell::new(false),
''')
    text = replace(text, '''        let value = b.use_var(self.variables[&slot].0);
''', '''        let value = match self.variables[&slot].0 {
            Home::Variable(variable) => b.use_var(variable),
            Home::Literal(value) => value,
        };
''')
    return replace(text, '''        b.def_var(self.variables[&slot].0, value);
''', '''        match self.variables[&slot].0 {
            Home::Variable(variable) => b.def_var(variable, value),
            Home::Literal(_) => self.literal_write.set(true),
        }
''')


def tests(text):
    text = replace(text, '''                variables: BTreeMap::from([(3, (variable, flags))]),
''', '''                variables: BTreeMap::from([(3, (Home::Variable(variable), flags))]),
                literal_write: Cell::new(false),
''')
    return text + '''
#[test]
fn entry_literal_needs_no_merge_parameter_and_flags_unexpected_writes() {
    let mut function = Function::new();
    let mut context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut function, &mut context);
    let entry = builder.create_block();
    let alternative = builder.create_block();
    let merge = builder.create_block();
    builder.switch_to_block(entry);
    let literal = builder.ins().iconst(types::I64, 41);
    let condition = builder.ins().icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, literal, 0);
    let words = Words {
        variables: BTreeMap::from([(0, (Home::Literal(literal), MemFlags::new()))]),
        literal_write: Cell::new(false),
        native_extent_slots: BTreeMap::new(),
        native_extent_values: RefCell::new(BTreeMap::new()),
    };
    builder.ins().brif(condition, alternative, &[], merge, &[]);
    builder.switch_to_block(alternative);
    builder.ins().jump(merge, &[]);
    builder.switch_to_block(merge);

    assert_eq!(words.read(&mut builder, 0), literal);
    assert!(builder.block_params(merge).is_empty());
    assert!(!words.attempted_literal_write());
    let changed = builder.ins().iconst(types::I64, 42);
    words.write(&mut builder, 0, changed);
    assert!(words.attempted_literal_write());

    builder.ins().return_(&[]);
    builder.seal_all_blocks();
    builder.finalize();
}
'''


def emit(text):
    text = replace(text, '''        None,
        compile_root,
    )
}



pub(in crate::native_artifact::program) fn define_with_isa'''.replace('\n\n\n\n','\n\n'),
        '''        None,
        compile_root,
        false,
    )
}

pub(in crate::native_artifact::program) fn define_with_isa''')
    start = text.index('pub(in crate::native_artifact::program) fn define_with_isa')
    stop = text.index('\nfn define_with_pool', start)
    text = text[:start] + '''pub(in crate::native_artifact::program) fn define_with_isa<M: Module>(
    module: &mut M,
    prepared: &PreparedNativeProgram,
    observe: bool,
    share_phase_bodies: bool,
    isa: cranelift_codegen::isa::OwnedTargetIsa,
    threads: usize,
    direct_runtime_calls: super::object::DirectRuntimeCalls,
    compile_root: Option<CompileRoot<'_, M>>,
    direct_literals: bool,
) -> Result<serde_json::Value, NativeCompileError> {
    let pool = (threads != 0)
        .then(|| super::compile_jobs::Pool::new(isa, threads, direct_runtime_calls))
        .transpose()?;
    define_with_pool(
        module,
        prepared,
        observe,
        share_phase_bodies,
        pool.as_ref(),
        compile_root,
        direct_literals,
    )
}
''' + text[stop:]
    text = replace(text, '''    pool: Option<&super::compile_jobs::Pool>,
    compile_root: Option<CompileRoot<'_, M>>,
)''', '''    pool: Option<&super::compile_jobs::Pool>,
    compile_root: Option<CompileRoot<'_, M>>,
    direct_literals: bool,
)''')
    text = replace(text, '''define_entry(module, prepared, observe, &shared, Some(&entry), &[], None)?''',
                   '''define_entry(module, prepared, observe, &shared, Some(&entry), &[], None, false)?''')
    text = replace(text, '''        &workers,
        compile_root,
    )?''', '''        &workers,
        compile_root,
        direct_literals,
    )?''')
    text = replace(text, '''    workers: &[super::parallel::Worker],
    compile_root: Option<CompileRoot<'_, M>>,
)''', '''    workers: &[super::parallel::Worker],
    compile_root: Option<CompileRoot<'_, M>>,
    direct_literals: bool,
)''')
    start = text.index('        let words = Words::new_with_regions_and_initial(')
    stop = text.index('        if let Some(arrays) = &arrays {', start)
    body = text[start:stop]
    source = body[body.index('            |b, slot| {') + len('            |b, slot| {'):body.rindex('\n            },')]
    update = '''        let initialize = |b: &mut FunctionBuilder<'_>, slot| {''' + source + '''
        };
        let literals = (direct_literals && outline_allowed)
            .then(|| scalar_literal_slots(plan, layout, &flow, &inputs, initial))
            .filter(|literals| !literals.is_empty());
        let words = if let Some(literals) = &literals {
            eprintln!("compact-stage: direct scalar literals={}", literals.len());
            Words::new_with_entry_literals(
                &mut b, zero, CodePlan::Program(prepared), slots, initialize, literals,
            )?
        } else {
            Words::new_with_regions_and_initial(
                &mut b, zero, CodePlan::Program(prepared), slots, initialize, |_| MemFlags::new(),
            )?
        };
'''
    text = text[:start] + update + text[stop:]
    text = replace(text, '''        eprintln!("compact-stage: finalize SSA");
''', '''        if words.attempted_literal_write() {
            return Err(backend("native lowering wrote an admitted entry literal"));
        }
        eprintln!("compact-stage: finalize SSA");
''')
    text = replace(text, 'fn define_entry<M: Module>(', '''fn scalar_literal_slots(
    plan: &mech_engine::native_artifact::NativeProgramArtifactPlan,
    layout: &Layout,
    flow: &Flow,
    inputs: &BTreeMap<usize, usize>,
    initial: &BTreeMap<usize, Initial>,
) -> BTreeSet<usize> {
    let mut written = vec![false; plan.values().len()];
    for &value in flow.defs.iter().flatten() {
        written[value as usize] = true;
    }
    plan.constants()
        .iter()
        .filter_map(|constant| {
            let value = constant.value().get();
            let ty = layout.ty(value);
            let slot = layout.slot(value);
            (!written[value as usize]
                && matches!(ty.kind, super::layout::LayoutKind::Scalar { .. })
                && ty.owners.is_empty()
                && !inputs.contains_key(&slot)
                && matches!(initial.get(&slot), Some(Initial::Bits(_))))
            .then_some(slot)
        })
        .collect()
}

fn define_entry<M: Module>(''')
    return text


def compiler(text):
    start = text.index('    let (object, isa) = codegen_backend::object_with_isa("mech_compact_closed_diagnostic"')
    end = text.index('\n/// Diagnostic compilation only: compact typed numeric functions', start)
    body = text[start:end]
    head = '''    let direct_literals = !observe
        && llvm_root.is_none()
        && std::env::var_os("MECH_COMPACT_DIRECT_ENTRY_LITERALS").is_some();
    let emit = |direct| emit_prepared_compact_object(&prepared, observe, llvm_root, &flags, direct);
    match emit(direct_literals) {
        Err(error) if direct_literals => {
            eprintln!("compact-stage: direct literal attempt withdrew: {error}");
            emit(false)
        }
        result => result,
    }
}

fn emit_prepared_compact_object(
    prepared: &PreparedNativeProgram,
    observe: bool,
    llvm_root: Option<&super::llvm_backend::NativeLlvmToolchain>,
    flags: &[(&str, &str)],
    direct_literals: bool,
) -> Result<(Vec<u8>, serde_json::Value), NativeCompileError> {
'''
    body = replace(body, '\n        let compile =', '\n        let compile =')
    body = body.replace('&prepared,', 'prepared,').replace(', &flags)?', ', flags)?')
    body = replace(body, '''            Some(&compile),
        )?''', '''            Some(&compile),
            false,
        )?''')
    body = replace(body, '''    } else if threads == 0 {
''', '''    } else if threads == 0 && !direct_literals {
''')
    body = replace(body, '''            None,
        )?''', '''            None,
            direct_literals,
        )?''')
    return text[:start] + head + body + text[end:]


FILES = {WORDS: words, WORD_TESTS: tests, EMIT: emit, COMPILER_GLUE: compiler}


def digest(data):
    return sha256(data).hexdigest()


def main():
    all_pins = {}
    patch = []
    for rel, transform in FILES.items():
        original = git('show', f'{BASE}:{rel}')
        assert (CHECKOUT / rel).read_bytes() == original, rel
        target = HERE / 'source' / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(transform(original.decode()))
        subprocess.run(['rustfmt', '+nightly-2026-03-03', '--edition=2024',
                        '--config', 'skip_children=true', target], check=True)
        candidate = target.read_bytes()
        all_pins[str(rel)] = {'baseline': digest(original), 'probe': digest(candidate)}
        patch.extend(unified_diff(original.decode().splitlines(keepends=True),
                                  candidate.decode().splitlines(keepends=True), f'a/{rel}', f'b/{rel}'))
    (HERE / 'compiler.patch').write_text(''.join(patch))
    (HERE / 'source.json').write_text(json.dumps({'base': BASE, 'files': all_pins}, indent=2) + '\n')
    print('Prepared', len(all_pins), 'files for the same-input literal probe.')


if __name__ == '__main__':
    main()
