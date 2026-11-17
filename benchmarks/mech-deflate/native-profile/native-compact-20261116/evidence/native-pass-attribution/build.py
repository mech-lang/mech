"""Build a diagnostic compiler from the pinned frame-copy source."""
from collections import Counter
from difflib import unified_diff
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROOT = Q.parent
SOURCE = Q / 'native-review-pr/checkout'
PROGRAM = Path('hosts/gpu/src/native_artifact/program')
BENCH = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')
BASE = '7a0b3127ce2ec163d3989f706b7ab819901ec0c2'
TARGET = Q / 'current-roadmap/definition-compiler/target'
REFERENCE = Q / 'current-roadmap/unused-error-shells/production/matched/plan.json'
ARMS = {
    'analysis.cfg': 'compute_cfg', 'analysis.dominators': 'compute_domtree',
    'analysis.flowgraph': 'flowgraph', 'analysis.verify': 'verify',
    'analysis.prune': 'eliminate_unreachable_code',
    'analysis.phi': 'remove_constant_phis',
    'analysis.prune-phi': 'prune_unreachable_and_remove_constant_phis',
}
HOOK = 'use crate::native_artifact::program::attribution::measure;\n'
FILE = PROGRAM / 'attribution.rs'


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)


def once(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


def before(text, old, extra):
    return once(text, old, extra + old)


def insertion(text, fn, group):
    signature = re.search(r'\bfn ' + fn + r'\b[\s\S]*?\{', text)
    assert signature is not None, fn
    at = signature.end()
    return text[:at] + '\n    let _pass = crate::native_artifact::program::attribution::span("' + group + '");' + text[at:]


def wrap(text, pattern, name):
    pattern = re.compile(pattern)
    matches = []
    for match in pattern.finditer(text):
        assert match.group()[-1] == '('
        level = 1
        at = match.end()
        while level:
            char = text[at]
            level += (char == '(') - (char == ')')
            at += 1
        matches.append((match.start(), at, name(match) if callable(name) else name))
    for start, end, tag in reversed(matches):
        text = text[:start] + f'measure!("{tag}", ' + text[start:end] + ')' + text[end:]
    return text, len(matches)


def calls(text, prefix, names):
    found = Counter()
    for tag, expression in names.items():
        text, count = wrap(text, r'(?<![\w:])' + expression + r'\(', prefix + tag)
        if count:
            found[tag] += count
    return text, dict(found)


def instrument(relative, text):
    profiled = {}
    if relative == Path('mod.rs'):
        line = '#[cfg(feature = "canonical-native-program")]\n'
        return text + '\n' + line + 'mod attribution;\n', {}
    if relative == Path('compiler.rs'):
        sig = '''fn emit_compact_closed_object(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    observe: bool,
    llvm_root: Option<&super::llvm_backend::NativeLlvmToolchain>,
) -> Result<(Vec<u8>, serde_json::Value), NativeCompileError> {
'''
        text = once(text, sig, sig + '    let _attribution = super::attribution::Session::start();\n')
        text = insertion(text, 'emit_compact_closed_object', 'native')
        # The session must outlive its outer native span.
        text = once(text, '''    let _pass = crate::native_artifact::program::attribution::span("native");
    let _attribution = super::attribution::Session::start();''',
            '''    let _attribution = super::attribution::Session::start();
    let _pass = crate::native_artifact::program::attribution::span("native");''')
        for name in ('lower_native_program_artifact_with_definition_table_limit', 'PreparedNativeProgram::prepare'):
            # Include calls in the ordinary entry points, where the session is absent.
            text, count = wrap(text, r'(?<![\w:])' + name + r'\(', f'prepare.{name}')
            assert count
        profiled['session'] = 1
    if relative == Path('compact/emit.rs'):
        for fn, detail in [('define_with_pool', 'compact.define'), ('define_entry', 'compact.entry')]:
            text = insertion(text, fn, detail)
        text = once(text, '    eprintln!("compact-stage: CFG cleanup");',
            '    let cleanup_pass = crate::native_artifact::program::attribution::span("root.cleanup");\n'
            '    eprintln!("compact-stage: CFG cleanup");')
        text = before(text, '    let calls = context\n', '    drop(cleanup_pass);\n')
        for tag, name in {
            'actions': 'Recipe::lower_all', 'flow': 'Flow::derive',
            'owners': 'OwnerFlow::derive', 'guards': 'Guards::derive',
            'word-packs': 'word_packs::derive', 'phases': 'phases::define',
            'layout': 'Layout::new', 'word-kernels': 'word_collections::define',
            'capacity-calls': r'growth\.replace', 'layouts': 'phase_storage::Geometry::derive',
        }.items():
            text, count = wrap(text, r'(?<![\w:])' + name + r'\(', f'compact.{tag}')
            profiled[tag] = count
        for prefix in ('cfg_cleanup', 'bit_counts', 'unsigned_guard_facts', 'vector_guard_facts',
                       'checked_traversals', 'checked_region_discovery', 'constant_folds',
                       'owner_counts', 'backing_owners', 'word_pairs', 'cold_status',
                       'owner_retirement', 'scalar_selects'):
            pattern = rf'\b{prefix}::([a-zA-Z_][a-zA-Z_0-9]*)\('
            # Long fully-qualified call paths are matched as one expression.
            text, count = wrap(text, r'(?<![\w:])(?:[a-zA-Z_][a-zA-Z_0-9]*::)*' + pattern,
                lambda m: f'compact.{prefix}.{m[1]}')
            profiled[prefix] = count
    if relative == Path('compact/flow.rs'):
        for fn, detail in [('derive_entry', 'flow.derive-entry'), ('prove_initialized', 'flow.initialization')]:
            text = insertion(text, fn, detail)
        return text, {'scopes': 2}
    if relative == Path('codegen/cfg_cleanup/mod.rs'):
        for fn, label in [('optimize_ordered', 'cleanup.ordered'), ('finish_imported_passes', 'cleanup.after-ordered')]:
            text = insertion(text, fn, label)
        parts = {
            'early-identities': r'fold_identities(?=\(&mut context\.func)',
            'constant-branches': r'fold(?=\(&mut context\.func)',
            'count-ir': r'counts(?=\(&context\.func)',
            'aliases': r'context\.func\.dfg\.resolve_all_aliases',
            'local-phis': 'local_phis::simplify',
            'preheaders': 'preheaders::normalize',
            'selected-ranges': 'selected_ranges::fold',
            'unsigned-intervals': 'unsigned_intervals::combine',
            'proved-widths': 'known_widths::fold',
            'demand-liveness': 'dead_values::eliminate',
            'identity-routes': 'identity_routes::optimize',
            'equal-arguments': 'local_phis::equal_arguments::simplify',
            'parallel-arguments': 'local_phis::coalesce_parallel',
            'literal-arguments': 'local_phis::literal_arguments::coalesce',
            'loop-edges': 'threaded_edges::optimize',
            'boolean-conditions': 'boolean_conditions::optimize',
        }
        text, profiled = calls(text, 'cleanup.', parts)
    if relative == Path('prepare/mod.rs'):
        # Each of these calls is a producer inside the prepared owner.
        start = text.index('    pub(in crate::native_artifact) fn prepare(')
        end = text.index('\n    }\n', start) + 6
        text_inside = text[start:end]
        for pattern in (r'((?:\w+::)+derive(?:_with_frontiers)?)\(',
                        r'((?:\w+::)+control_visits)\(',
                        r'(facts\.(?:scalar_aggregates|buffer_frame_owner_layouts|unsigned_buffer_calls|text_line_retention_frontiers))\(',
                        r'(plan\.(?:validate_current_visits|string_map_regions))\(',
                        r'(compose_work|prepare_frame_composition|prepare_native_program_library_call_with_descriptors)\('):
            text_inside, count = wrap(text_inside, r'(?<![\w:])' + pattern, lambda m: 'prepare.' + m[1])
            profiled[pattern] = count
        text = text[:start] + text_inside + text[end:]
    if relative in (Path('compact/emit.rs'), Path('codegen/cfg_cleanup/mod.rs')):
        text, counts = calls(text, '', {tag: r'context\s*\.\s*' + method for tag, method in ARMS.items()})
        profiled.update(counts)
    assert 'measure!(' in text, relative
    first = text.index('\nuse ') + 1
    return text[:first] + HOOK + text[first:], profiled


def stage():
    original = {}
    generated = {}
    applied = {}
    for name in ('mod.rs', 'compiler.rs', 'compact/emit.rs', 'compact/flow.rs',
                 'codegen/cfg_cleanup/mod.rs', 'prepare/mod.rs'):
        relative = Path(name)
        path = PROGRAM / relative
        old = git('show', f'{BASE}:{path}')
        assert (SOURCE / path).read_bytes() == old, path
        result, sites = instrument(relative, old.decode())
        original[path] = old
        generated[path] = result
        applied[str(relative)] = sites
    original[FILE] = None
    generated[FILE] = (HERE / 'attribution.rs').read_text()
    for relative, data in generated.items():
        path = HERE / 'patch-source' / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(data)
        subprocess.run(['rustfmt', '+nightly-2026-03-03', '--edition', '2024',
                        '--config', 'skip_children=true', path], check=True)
    patch = []
    for p, old in original.items():
        new = (HERE / 'patch-source' / p).read_text()
        patch += unified_diff(old.decode().splitlines(keepends=True) if old else [],
                              new.splitlines(keepends=True), f'a/{p}' if old else '/dev/null', f'b/{p}')
    (HERE / 'instrumentation.patch').write_text(''.join(patch))
    write(HERE / 'instrumentation.json', {
        'source': BASE, 'wrapped_sites': applied, 'patch_sha256': digest(HERE / 'instrumentation.patch'),
        'temporary_source': {str(f): digest(HERE / 'patch-source' / f) for f in generated},
    })
    return original


def build():
    assert git('rev-parse', 'refs/heads/codex/native-compact-source-handoff').decode().strip() == BASE
    files = stage()
    pins = read(REFERENCE)['actors']['mech-early-error-pruning']['artifact_sha256']
    dest = HERE / 'compiler'
    dest.mkdir()
    good = []
    try:
        for relative, old in files.items():
            path = SOURCE / relative
            assert (path.read_bytes() if path.exists() else None) == old
            shutil.copyfile(HERE / 'patch-source' / relative, path)
            good.append(relative)
        command = [sys.executable, '-B', SOURCE / BENCH / 'rebuild.py', dest / 'rebuild',
                   '--variant','frame-copies','--compiler-opt-level','3','--direct-source',
                   '--preselect-root-inlining','--skip-refused-general-ssa','--target-dir',TARGET]
        env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
        with (dest / 'build.log').open('wb') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE':'true'}, stdout=log,
                           stderr=subprocess.STDOUT, check=True)
        for path,key in [('frontend/program.mecb','bytecode'),('native/program.o','encoder'),
                         ('native/metadata.json','metadata')]:
            assert digest(dest / 'rebuild' / path) == pins[key], path
        executable = dest / 'compile_profile'
        shutil.copy2(TARGET / 'release/compile_profile', executable)
        write(dest / 'build.json', {
            'command':list(map(str,command)), 'reference_plan_sha256': digest(REFERENCE),
            'compiler_sha256': digest(executable), 'artifacts': pins,
        })
        print(executable, digest(executable), flush=True)
    finally:
        for relative in good:
            path = SOURCE / relative
            assert path.read_bytes() == (HERE / 'patch-source' / relative).read_bytes()
            previous = files[relative]
            if previous is None:
                path.unlink()
            else:
                path.write_bytes(previous)


if __name__ == '__main__':
    build()
