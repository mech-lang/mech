"""Prepare opt-in compiler attribution without changing emitted artifacts."""

from difflib import unified_diff
from hashlib import sha256
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
REPO = Q.parent
CHECKOUT = Q / 'native-review-pr/checkout'
HEAD = '3cde720cdf8b24d6bc4f1818988479d8eb5ae960'
PACKAGE = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')
SEM = Path('src/engine/src/source_semantics')
GPU = Path('hosts/gpu/src/native_artifact')


def git(*args):
    return subprocess.check_output(['git', *map(str, args)], cwd=REPO)


def digest(value):
    return sha256(value).hexdigest()


def replace(source, old, new):
    assert source.count(old) == 1, old
    return source.replace(old, new)


def within(source, name, change):
    start = source.index(name)
    return source[:start] + change(source[start:])


PHASE_HELPER = '''
struct SourceProfile(Option<std::time::Instant>);

impl SourceProfile {
    fn start() -> Self {
        Self(std::env::var_os("MECH_PROFILE_SOURCE_SEMANTICS").map(|_| std::time::Instant::now()))
    }

    fn mark(&mut self, label: &str) {
        if let Some(start) = self.0 {
            eprintln!("profile-source-semantic {label} {}", start.elapsed().as_nanos());
            self.0 = Some(std::time::Instant::now());
        }
    }
}

'''


def semantic_frontend(source):
    source = replace(source, 'struct SemanticBuilder {', PHASE_HELPER.lstrip() + 'struct SemanticBuilder {')
    edits = (
        ('    fn finish(self) -> Result<CanonicalSourceProgram, SourceSemanticError> {',
         '    fn finish(self) -> Result<CanonicalSourceProgram, SourceSemanticError> {\n'
         '        let mut profile = SourceProfile::start();'),
        ('        let schemas = SourceSchemas::build(', '        profile.mark("resolve_pending");\n        let schemas = SourceSchemas::build('),
        ('        let constant_schema_ids = self', '        profile.mark("schema_finalization");\n        let constant_schema_ids = self'),
        ('        let inputs = self\n            .inputs', '        profile.mark("constant_store");\n        let inputs = self\n            .inputs'),
        ('        let mut requirement_entries = self', '        profile.mark("pack_inputs");\n        let mut requirement_entries = self'),
        ('        let mut contracts = Vec::with_capacity(self.nodes.len());',
         '''        profile.mark("requirement_canonicalization");
        let mut lookups = 0u64;
        let mut comparisons = 0u64;
        let mut lookup_time = 0u128;
        let mut contracts = Vec::with_capacity(self.nodes.len());'''),
        ('''                                requirements.iter().find_map(|(id, candidate)| {
                                    (candidate == requirement).then_some(id)
                                })''',
         '''                                if profile.0.is_some() {
                                    let start = std::time::Instant::now();
                                    let matched = requirements.iter().find_map(|(id, candidate)| {
                                        comparisons += 1;
                                        (candidate == requirement).then_some(id)
                                    });
                                    lookup_time += start.elapsed().as_nanos();
                                    lookups += 1;
                                    matched
                                } else {
                                    requirements.iter().find_map(|(id, candidate)| {
                                        (candidate == requirement).then_some(id)
                                    })
                                }'''),
        ('        let outputs = self\n            .outputs',
         '        profile.mark("node_materialization_includes_requirements");\n        let outputs = self\n            .outputs'),
        ('        Ok(CanonicalSourceProgram {\n',
         '        let program = CanonicalSourceProgram {\n'),
        ('''            source_nominal_declarations: schemas.declarations,
        })
    }
}''', '''            source_nominal_declarations: schemas.declarations,
        };
        profile.mark("pack_outputs_graph");
        if profile.0.is_some() {
            eprintln!(
                "profile-source-requirement-lookups {lookups} {comparisons} {lookup_time}"
            );
        }
        Ok(program)
    }
}'''),
    )
    def change(body):
        # End before tests and any unrelated frontend entry.
        end = body.index('\nfn pending_schema(')
        main, rest = body[:end], body[end:]
        for before, after in edits:
            main = replace(main, before, after)
        return main + rest
    return within(source, '    fn finish(self) -> Result<CanonicalSourceProgram, SourceSemanticError>', change)


def document(source):
    start = source.index('pub(super) fn compile_document_with_capture_options(')
    end = source.index('pub(super) fn compile_named_document_scope(', start)
    body = source[start:end]
    body = replace(body, '    let anchor = SourceSemanticAnchor::for_node(document.syntax());',
        '    let mut profile = SourceProfile::start();\n    let anchor = SourceSemanticAnchor::for_node(document.syntax());')
    body = replace(body, '    compile_collected_document(', '    profile.mark("collect_document_units");\n    compile_collected_document(')
    source = source[:start] + body + source[end:]
    start = source.index('fn compile_collected_document(')
    end = source.index('pub(super) fn document_compute_regions(', start)
    body = source[start:end]
    edits = (
        ('    let mut builder = match catalog {', '    let mut profile = SourceProfile::start();\n    let mut builder = match catalog {'),
        ('    builder.register_document_types(&units, nominal_origin)?;',
         '    profile.mark("create_semantic_builder");\n    builder.register_document_types(&units, nominal_origin)?;'),
        ('    builder.register_document_functions(&units)?;',
         '    profile.mark("register_types_and_fsms");\n    builder.register_document_functions(&units)?;'),
        ('    let mut bindings = BTreeSet::new();', '    profile.mark("register_functions_and_imports");\n    let mut bindings = BTreeSet::new();'),
        ('    let last = compile_document_units(', '    profile.mark("declare_inputs_and_scan_units");\n    let last = compile_document_units('),
        ('    let Some(last) = last else {', '    profile.mark("lower_document_units");\n    let Some(last) = last else {'),
        ('    builder.order_document_state_writers();\n    let mut program = builder.finish()?;',
         '    builder.order_document_state_writers();\n    profile.mark("publish_and_order_lowered_document");\n    let mut program = builder.finish()?;'),
    )
    for before, after in edits:
        body = replace(body, before, after)
    return source[:start] + body + source[end:]


def catalog(source):
    head = '''
#[cfg(all(feature = "source", not(feature = "no_std")))]
fn catalog_profile_start() -> Option<std::time::Instant> {
    std::env::var_os("MECH_PROFILE_SOURCE_SEMANTICS").map(|_| std::time::Instant::now())
}

#[cfg(all(feature = "source", not(feature = "no_std")))]
fn catalog_profile_mark(start: &mut Option<std::time::Instant>, label: &str) {
    if let Some(at) = start {
        eprintln!("profile-source-catalog {label} {}", at.elapsed().as_nanos());
        *start = Some(std::time::Instant::now());
    }
}
'''
    source = replace(source, 'use std::sync::{Arc, OnceLock};\n', 'use std::sync::{Arc, OnceLock};\n' + head)
    source = replace(source, '''    let thread = std::thread::Builder::new()
''', '''    #[cfg(feature = "source")]
    let mut startup = (name == "source").then(catalog_profile_start).flatten();
    let thread = std::thread::Builder::new()
''')
    source = replace(source, '''        .spawn(move || {
            Arc::new(
''', '''        .spawn(move || {
            #[cfg(feature = "source")]
            catalog_profile_mark(&mut startup, "thread_start");
            Arc::new(
''')
    start = source.index('pub fn build_source_catalog() -> MResult<FunctionCatalog> {')
    end = source.index('\n/// Builds a source catalog', start)
    body = source[start:end]
    body = replace(body, '    let mut builder = FunctionCatalogBuilder::new();',
        '''    #[cfg(not(feature = "no_std"))]
    let mut profile = catalog_profile_start();
    let mut builder = FunctionCatalogBuilder::new();
    #[cfg(not(feature = "no_std"))]
    catalog_profile_mark(&mut profile, "create_builder");''')
    body = replace(body, '    install_source(&mut builder)?;\n    builder.build()',
        '''    #[cfg(not(feature = "no_std"))]
    catalog_profile_mark(&mut profile, "install_runtime");
    install_source(&mut builder)?;
    #[cfg(not(feature = "no_std"))]
    catalog_profile_mark(&mut profile, "install_source");
    let result = builder.build();
    #[cfg(not(feature = "no_std"))]
    catalog_profile_mark(&mut profile, "finalize_validate");
    result''')
    return source[:start] + body + source[end:]


def ssa(source):
    marker = '// Delay the per-variable hash allocation'
    addition = '''
#[cfg(feature = "std")]
fn profile_ssa_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("MECH_PROFILE_SSA_CONSTRUCTION").is_some())
}

#[cfg(feature = "std")]
#[derive(Default)]
struct SsaProbe {
    definitions: usize,
    use_requests: usize,
    local_hits: usize,
    nonlocal_requests: usize,
    definition_samples: usize,
    definition_ns: u128,
    use_samples: usize,
    use_ns: u128,
}

'''
    source = replace(source, marker, addition.lstrip() + marker)
    source = replace(source, '    inst_pool: ListPool<Inst>,\n}', '''    inst_pool: ListPool<Inst>,
    #[cfg(feature = "std")]
    probe: SsaProbe,
}''')
    source = replace(source, '''    len: usize,
}
impl VariableDefs''', '''    len: usize,
    #[cfg(feature = "std")]
    sparse_grows: usize,
    #[cfg(feature = "std")]
    dense_grows: usize,
}
impl VariableDefs''')
    source = replace(source, '''        let previous = if index < self.dense.len() {''', '''        #[cfg(feature = "std")]
        let capacity_before = profile_ssa_enabled().then(|| match &self.sparse {
            SparseDefs::Many(map) => map.capacity(),
            _ => 0,
        });
        let previous = if index < self.dense.len() {''')
    source = replace(source, '''        if previous.is_none() {
            self.len += 1;
        }''', '''        #[cfg(feature = "std")]
        if let (Some(before), SparseDefs::Many(map)) = (capacity_before, &self.sparse) {
            self.sparse_grows += usize::from(map.capacity() > before);
        }
        if previous.is_none() {
            self.len += 1;
        }''')
    source = replace(source, '''                self.dense.resize(size, None);''', '''                #[cfg(feature = "std")]
                if profile_ssa_enabled() {
                    self.dense_grows += 1;
                }
                self.dense.resize(size, None);''')
    source = replace(source, '''    pub fn clear(&mut self) {
        self.variables.clear();''', '''    pub fn clear(&mut self) {
        #[cfg(feature = "std")]
        if profile_ssa_enabled() {
            let mut occupancy = [0usize; 6];
            let mut dense_maps = 0usize;
            let mut sparse_grows = 0;
            let mut dense_grows = 0;
            let mut entries = 0usize;
            for defs in self.variables.values() {
                let bucket = match defs.len {
                    0..=1 => defs.len,
                    2..=4 => 2,
                    5..=15 => 3,
                    16..=63 => 4,
                    _ => 5,
                };
                occupancy[bucket] += 1;
                dense_maps += usize::from(!defs.dense.is_empty());
                sparse_grows += defs.sparse_grows;
                dense_grows += defs.dense_grows;
                entries += defs.len;
            }
            let p = mem::take(&mut self.probe);
            if p.use_requests + p.definitions != 0 {
                let [empty, one, few, short, medium, large] = occupancy;
                std::eprintln!(
                    "profile-ssa {empty} {one} {few} {short} {medium} {large} {entries} {dense_maps} {sparse_grows} {dense_grows} {} {} {} {} {} {} {} {}",
                    p.definitions, p.use_requests, p.local_hits, p.nonlocal_requests,
                    p.definition_samples, p.definition_ns, p.use_samples, p.use_ns,
                );
            }
        }
        self.variables.clear();''')
    source = replace(source, '''        self.variables[var].insert(block, PackedOption::from(val));
    }

    /// Declares a use''', '''        #[cfg(feature = "std")]
        let sample = if profile_ssa_enabled() {
            self.probe.definitions += 1;
            (self.probe.definitions % 64 == 1).then(std::time::Instant::now)
        } else {
            None
        };
        self.variables[var].insert(block, PackedOption::from(val));
        #[cfg(feature = "std")]
        if let Some(at) = sample {
            self.probe.definition_samples += 1;
            self.probe.definition_ns += at.elapsed().as_nanos();
        }
    }

    /// Declares a use''')
    source = replace(source, '''        // Prepare the 'calls' and 'results' stacks for the state machine.''', '''        #[cfg(feature = "std")]
        let sample = if profile_ssa_enabled() {
            self.probe.use_requests += 1;
            (self.probe.use_requests % 64 == 1).then(std::time::Instant::now)
        } else {
            None
        };

        // Prepare the 'calls' and 'results' stacks for the state machine.''')
    source = replace(source, '''        let side_effects = mem::take(&mut self.side_effects);
        (value, side_effects)
''', '''        let side_effects = mem::take(&mut self.side_effects);
        #[cfg(feature = "std")]
        if let Some(at) = sample {
            self.probe.use_samples += 1;
            self.probe.use_ns += at.elapsed().as_nanos();
        }
        (value, side_effects)
''')
    source = replace(source, '''            self.results.push(val);
            return;
        }

        // Otherwise, use Global Value Numbering''', '''            #[cfg(feature = "std")]
            if profile_ssa_enabled() {
                self.probe.local_hits += 1;
            }
            self.results.push(val);
            return;
        }

        #[cfg(feature = "std")]
        if profile_ssa_enabled() {
            self.probe.nonlocal_requests += 1;
        }
        // Otherwise, use Global Value Numbering''')
    return source


def words(source):
    source = replace(source, 'use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, types};',
                              'use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, ValueDef, types};')
    source = replace(source, '    variables: BTreeMap<usize, (Variable, MemFlags)>,',
        '''    variables: BTreeMap<usize, (Variable, MemFlags)>,
    provenance: Option<(Block, RefCell<BTreeMap<usize, WordProfile>>)>,''')
    source = replace(source, '#[cfg(all(test, feature = "canonical-native-program"))]',
        '''#[derive(Default)]
struct WordProfile {
    origin: &'static str,
    reads: usize,
    writes: usize,
    parameter_reads: usize,
}

#[cfg(all(test, feature = "canonical-native-program"))]''')
    source = replace(source, '            variables,\n            native_extent_slots:',
        '            variables,\n            provenance: None,\n            native_extent_slots:')
    source = replace(source, '        let value = b.use_var(self.variables[&slot].0);', '''        let value = b.use_var(self.variables[&slot].0);
        if let Some((entry, provenance)) = &self.provenance {
            let mut profiles = provenance.borrow_mut();
            let profile = profiles.get_mut(&slot).expect("profiled SSA lane");
            profile.reads += 1;
            if let ValueDef::Param(block, _) = b.func.dfg.value_def(value)
                && block != *entry
            {
                profile.parameter_reads += 1;
            }
        }''')
    source = replace(source, '        b.def_var(self.variables[&slot].0, value);',
        '''        if let Some((_, provenance)) = &self.provenance {
            provenance.borrow_mut().get_mut(&slot).expect("profiled SSA lane").writes += 1;
        }
        b.def_var(self.variables[&slot].0, value);''')
    source = replace(source, '    pub(in crate::native_artifact) fn spill_all(',
        '''    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn profile_words(
        &mut self,
        entry: Block,
        origin: impl Fn(usize) -> &'static str,
    ) {
        if std::env::var_os("MECH_PROFILE_SSA_CONSTRUCTION").is_some() {
            self.provenance = Some((
                entry,
                RefCell::new(self.variables.keys().map(|&slot| {
                    (slot, WordProfile { origin: origin(slot), ..WordProfile::default() })
                }).collect()),
            ));
        }
    }
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn report_words(&self) {
        if let Some((entry, profiles)) = &self.provenance {
            let profiles = profiles.borrow();
            eprintln!("profile-word-bank {} {}", entry.as_u32(), profiles.len());
            for (slot, profile) in profiles.iter() {
                eprintln!("profile-word {slot} {} {} {} {}",
                    profile.origin, profile.reads, profile.writes, profile.parameter_reads);
            }
        }
    }
    pub(in crate::native_artifact) fn spill_all(''')
    return source


def compact(source):
    source = replace(source, '        let words = Words::new_with_regions_and_initial(',
                             '        let mut words = Words::new_with_regions_and_initial(')
    source = replace(source, '''            |_| MemFlags::new(),
        )?;
        if let Some(arrays) = &arrays {''', '''            |_| MemFlags::new(),
        )?;
        words.profile_words(entry, |slot| {
            if inputs.contains_key(&slot) {
                "input"
            } else {
                match initial.get(&slot) {
                    Some(Initial::Bits(_)) => "constant",
                    Some(Initial::Table(_)) => "table",
                    None => "seed",
                }
            }
        });
        if let Some(arrays) = &arrays {''')
    return replace(source, '''        eprintln!("compact-stage: finalize SSA");
        capacity_growth''', '''        words.report_words();
        eprintln!("compact-stage: finalize SSA");
        capacity_growth''')


SOURCES = {
    SEM / 'frontend.rs': semantic_frontend,
    SEM / 'document_lowering.rs': document,
    Path('src/stdlib/src/catalog.rs'): catalog,
    Path('vendor/cranelift-frontend/src/ssa.rs'): ssa,
    GPU / 'codegen/common/ssa_words.rs': words,
    GPU / 'program/compact/emit.rs': compact,
}


def main():
    pins = {}
    patches = []
    for name, transform in SOURCES.items():
        source = git('show', f'{HEAD}:{name}')
        assert (CHECKOUT / name).read_bytes() == source, name
        output = HERE / 'source' / name
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(transform(source.decode()))
        subprocess.run(['rustfmt', '+nightly-2026-03-03', '--edition=2024',
                        '--config', 'skip_children=true', output], check=True)
        edited = output.read_bytes()
        pins[str(name)] = {'original': digest(source), 'attribution': digest(edited)}
        patches.extend(unified_diff(source.decode().splitlines(keepends=True),
            edited.decode().splitlines(keepends=True), f'a/{name}', f'b/{name}'))
    (HERE / 'instrumentation.patch').write_text(''.join(patches))
    (HERE / 'source.json').write_text(json.dumps({'base': HEAD, 'sources': pins}, indent=2) + '\n')
    print('Prepared', len(pins), 'exact-source probes.', flush=True)


if __name__ == '__main__':
    main()
