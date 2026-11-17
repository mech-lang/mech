"""Stage statement replay and the paired capture-only ablation."""
import difflib
import json
from pathlib import Path
import shutil
import sys

here = Path(__file__).resolve().parent
profile = Path('/private/tmp/mech-post-function-profile.FgQnHIs3')
sys.path.insert(0, str(profile))
from driver import base, checkout, digest, doc, env, git, input_source, pin, replace, run, syntax, write

prior = Path('/private/tmp/mech-definition-replay.sDi0bKpN')
env.update(MECH_DEFINITION_SOURCE=str(input_source))
old_tests = Path('/private/tmp/mech-dense-index.5noolrp2/validation')
files = [syntax / doc / name for name in ('continuation.rs', 'definition_replay.rs')]


def extend_continuation(text):
    anchor = """            rules::FUNCTION_DEFINE if parser.current_rule() == Some(rules::MECH_CODE_ALT) => {
                SyntaxKind::FunctionDefine
            }
"""
    return replace(text, anchor, anchor + """            rules::STATEMENT if parser.current_rule() == Some(rules::MECH_CODE_ALT) => {
                SyntaxKind::Statement
            }
""").replace('record_definition(', 'record_structural_choice(')


def extend_recorder(text):
    return replace(text, '//! Scoped recipes for winning variable and function definitions.\n',
                   '//! Scoped recipes for selected structural alternatives.\n')


def workspace(name):
    return here / name / 'workspace'


def fmt(paths, log):
    run(['rustfmt', '+nightly-2026-03-03', '--edition', '2024', '--config', 'skip_children=true', *paths], log)


def stage():
    for role in ('candidate', 'capture-only', 'validation'):
        (here / role).mkdir()
    candidate = workspace('candidate')
    assert git('rev-parse', 'refs/heads/codex/native-compact-source-handoff').decode().strip() == pin
    shutil.copytree(base / syntax, candidate / syntax)
    for file, change in zip(files, (extend_continuation, extend_recorder), strict=True):
        path = candidate / file
        path.write_text(change(path.read_text()))
    fmt([candidate / file for file in files], here / 'rustfmt.log')
    patch = []
    for file in files:
        assert (base / file).read_bytes() == git('show', f'{pin}:{file}')
        patch.extend(difflib.unified_diff((base / file).read_text().splitlines(keepends=True),
                                          (candidate / file).read_text().splitlines(keepends=True),
                                          f'a/{file}', f'b/{file}'))
    (here / 'statement-replay.patch').write_text(''.join(patch))
    ablation = workspace('capture-only')
    shutil.copytree(candidate, ablation)
    f = ablation / files[1]
    f.write_text(replace(f.read_text(), '        if !APPLY {\n',
                         '        if !APPLY || self.origin.kind == SyntaxKind::Statement {\n'))
    create_validation(candidate)
    write(here / 'source-bindings.json', {'pin': pin, 'input': digest(input_source),
                                          'patch': digest(here / 'statement-replay.patch'),
                                          'production_sources': {str(p): digest(candidate / p) for p in files},
                                          'ablation': digest(f)})


def create_validation(candidate):
    checked = workspace('validation')
    shutil.copytree(candidate, checked)
    shutil.copytree(checkout / 'src/syntax/tests/canonical_rules', checked / 'src/syntax/tests/canonical_rules')
    support = checked / 'src/core/tests/support'
    support.mkdir(parents=True)
    shutil.copyfile(checkout / 'src/core/tests/support/allocation_probe.rs', support / 'allocation_probe.rs')
    f = checked / syntax / 'document/parser/mod.rs'
    text = replace(f.read_text(), 'pub mod checkpoint;\n', 'pub mod checkpoint;\nmod definition_validation;\n')
    text = replace(text, '    definition_raw: DefinitionRawOrigin,\n',
                   '    definition_raw: DefinitionRawOrigin,\n    validation: definition_validation::Receipt,\n')
    text = replace(text, '            definition_raw: DefinitionRawOrigin::Off,\n',
                   '            definition_raw: DefinitionRawOrigin::Off,\n            validation: definition_validation::Receipt::default(),\n')
    f.write_text(text)
    validation = f.with_name('definition_validation.rs')
    tests = (prior / 'validation.rs').read_text().replace('fn scoped_definition_', 'fn scoped_statement_')
    tests = replace(tests, '            assert!(applied.applies > 0, "{applied:?}");\n',
                    '            assert!(applied.applies > 0, "{applied:?}");\n')
    tests = replace(tests, '    let fixtures = [\n', '''    let fixtures = [
        "add(a<u64>, b<u64>) => (out<u64>) :=\\n  sum := a + b\\n  out := sum.\\n",
        "é(a<u64>) => (out<u64>) :=\\n  name := \\"e\\u{301}\\"\\n  out := a.\\n",
        "add(a<u64>) => <u64> := a + 1u64.\\n",
        "same(n<u64>) => <u64>\\n  | (n) => n.\\n",
        "bad(a<u64>) => (out<u64>) :=\\n  val := a +\\n",
        "```mech\\nadd(a<u64>) => <u64> := a + 1u64.\\n```\\n",
'''.replace('\\"', '\\"'))
    validation.write_text(tests)
    f = checked / files[1]
    text = replace(f.read_text(), '''        if matches!(parser.state.definition_raw, DefinitionRawOrigin::Waiting)
            || !sealed(parser, position)''', '''        if (parser.state.validation.disabled && kind == SyntaxKind::Statement)
            || matches!(parser.state.definition_raw, DefinitionRawOrigin::Waiting)
            || !sealed(parser, position)''')
    text = replace(text, '''                origin
                    .finish(parser, position, raw)
                    .map(|summary| Box::new(Self(Phase::Selected(summary))))''', '''                origin.finish(parser, position, raw).map(|summary| {
                    if summary.origin.kind == SyntaxKind::Statement {
                        let report = &mut parser.state.validation;
                        report.captures += 1;
                        report.max_events = report.max_events.max(summary.events.len());
                        report.max_payload_bytes = report.max_payload_bytes.max(
                            summary.events.capacity() * core::mem::size_of::<Event>()
                        );
                        report.max_ordinals = report.max_ordinals.max(summary.nodes).max(summary.tokens);
                    }
                    Box::new(Self(Phase::Selected(summary)))
                })''')
    text = replace(text, '        if matches!(parser.state.definition_raw, DefinitionRawOrigin::Waiting)\n            || !sealed(parser, at)', '        if matches!(parser.state.definition_raw, DefinitionRawOrigin::Waiting)\n            || (parser.state.validation.disabled\n                && !matches!(parser.state.definition_raw, DefinitionRawOrigin::Off))\n            || !sealed(parser, at)')
    text = replace(text, '        let events = self.fresh_events(parser.ids)?;\n', '''        let events = self.fresh_events(parser.ids)?;
        if self.origin.kind == SyntaxKind::Statement {
            let first_grant = at.steps + at.child_work + self.steps + self.child_work
                + remaining_headroom(parser).expect("admitted stale-cache headroom");
            let report = &mut parser.state.validation;
            report.applies += 1;
            if at.poll == 1 && report.minimum_first_grant == 0 {
                report.minimum_first_grant = first_grant;
            }
        }
''')
    witness = (prior / 'busy-witness-test.rs').read_text().replace('scoped_definition_', 'scoped_statement_')
    witness = replace(witness, '        input_final: true,\n', '        input_final: true,\n        delimiter: None,\n')
    witness = witness.replace('Recording::begin(&mut parser, position)',
                               'Recording::begin(&mut parser, position, SyntaxKind::FunctionDefine)')
    start = witness.index('    parser.state.definition_raw = DefinitionRawOrigin::Fresh')
    witness = witness[:start] + """    parser.state.definition_raw = DefinitionRawOrigin::Waiting;
    assert!(Recording::begin(&mut parser, position, SyntaxKind::FunctionDefine).is_none());
    assert!(matches!(parser.state.definition_raw, DefinitionRawOrigin::Waiting));

    parser.state.definition_raw = DefinitionRawOrigin::Fresh {
        at: TextSize(1),
        final_end: Some(snapshot.byte_len()),
    };
    let trial = Recording::begin(&mut parser, position, SyntaxKind::FunctionDefine).unwrap();
    assert!(matches!(parser.state.definition_raw, DefinitionRawOrigin::Waiting));
    parser.observe_definition_raw_origin(Some(snapshot.byte_len()));
    assert!(matches!(parser.state.definition_raw, DefinitionRawOrigin::Fresh { at: TextSize(0), .. }));
    assert!(trial.observe(&mut parser, position, false).is_none());
    assert!(matches!(parser.state.definition_raw, DefinitionRawOrigin::Fresh { at: TextSize(1), final_end: Some(end) } if end == snapshot.byte_len()));

    parser.state.definition_raw = DefinitionRawOrigin::Inherited;
    let trial = Recording::begin(&mut parser, position, SyntaxKind::VariableDefine).unwrap();
    assert!(trial.observe(&mut parser, position, false).is_none());
    assert!(matches!(parser.state.definition_raw, DefinitionRawOrigin::Inherited));
}
"""
    f.write_text(text + '\n' + witness)
    fmt([f, validation], here / 'validation-rustfmt.log')


def build(name):
    out = here / name
    root = workspace(name)
    binding = json.loads((profile / 'plain/build.json').read_bytes())
    source = root / syntax / 'lib.rs'
    if name == 'validation':
        existing = json.loads((old_tests / 'receipt.json').read_text())['tests']['build']
        sha2 = next(arg for arg in binding['link'] if arg.startswith('sha2='))
        binary = out / 'test-syntax'
        command = [str(source) if arg.endswith('/source/lib.rs')
                   else str(binary) if arg == str(old_tests / 'syntax-tests')
                   else 'metadata=statement-replay-exact' if arg.startswith('metadata=')
                   else arg for arg in existing] + ['--extern', sha2]
        assert str(source) in command and str(binary) in command
        command = run(command, out / 'build.log')
        write(out / 'build.json', {'compile': command, 'binary': digest(binary)})
    else:
        library = out / 'libmech_syntax.rlib'
        binary = out / 'parser'
        command = [str(source) if arg == str(base / syntax / 'lib.rs')
                   else str(library) if arg == str(profile / 'plain/libmech_syntax.rlib')
                   else f'metadata=statement-replay-{name}' if arg.startswith('metadata=')
                   else arg for arg in binding['compile']]
        assert str(source) in command and str(library) in command
        command = run(command, out / 'build.log')
        link = [f'mech_syntax={library}' if arg.startswith('mech_syntax=')
                else str(binary) if arg == str(profile / 'plain/parser')
                else arg for arg in binding['link']]
        assert str(binary) in link
        link = run(link, out / 'link.log')
        write(out / 'build.json', {'compile': command, 'link': link, 'binary': digest(binary)})
    print(f'{name} {digest(binary)}', flush=True)


if __name__ == '__main__':
    job, *arguments = sys.argv[1:]
    if job == 'stage':
        stage()
    elif job == 'build':
        for argument in arguments:
            build(argument)
