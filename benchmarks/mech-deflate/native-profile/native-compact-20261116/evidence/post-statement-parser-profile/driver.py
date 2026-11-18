"""Measure remaining structural parser work after statement replay."""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile

here = Path(__file__).resolve().parent
repo = Path('/private/tmp/mech-shared-cold-epilogues.2L6obP')
q = repo / '.qualification'
checkout = q / 'native-review-pr/checkout'
prior = Path('/private/tmp/mech-best-choice-profile.Y0SGCETr')
prior_standalone = Path('/private/tmp/mech-definition-replay.sDi0bKpN')
sys.path.insert(0, str(prior))
from prepare import patch_document, replace, receipt, source as input_source

pin = 'e8b2bb4f2d8c44390e9a67dfe054c6ff40341249'
syntax = Path('src/syntax/src')
doc = Path('document/parser/canonical/document')
base = here / 'pinned'
overlay = here / 'diagnostic'
env = os.environ | {'CARGO_MANIFEST_DIR': str(checkout / 'src/syntax'), 'CARGO_PKG_VERSION': '0.3.5'}


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(*args):
    return subprocess.check_output(['git', *map(str, args)], cwd=repo)


def run(args, log):
    command = list(map(str, args))
    with log.open('wb') as output:
        subprocess.run(command, cwd=checkout, env=env, stdout=output, stderr=subprocess.STDOUT, check=True)
    return command


def patch_continuation(text):
    text = replace(text, '    definition_poll: u64,\n',
                   '    definition_poll: u64,\n    best_choice_probe: super::best_choice_probe::State,\n')
    text = replace(text, '            definition_poll: 0,\n',
                   '            definition_poll: 0,\n            best_choice_probe: super::best_choice_probe::State::default(),\n')
    text = replace(text, '''    ) -> Progress {
        self.definition_poll = self.definition_poll.saturating_add(1);''', '''    ) -> Progress {
        self.best_choice_probe.resume();
        let progress = self.advance_frames(parser, input_final, allowance);
        self.best_choice_probe.suspend();
        progress
    }

    fn advance_frames(
        &mut self,
        parser: &mut Parser<'_>,
        input_final: bool,
        allowance: &mut u64,
    ) -> Progress {
        self.definition_poll = self.definition_poll.saturating_add(1);''')
    completed = '''self.best_choice_probe.finish_ready(
                self.frames.len(), parser, self.state, self.steps + self.child_work, input_final,
                self.result,
            );'''
    text = replace(text, '            let final_input = input_final || !parser.state.cursor_frontier;\n',
                   f'            {completed}\n            let final_input = input_final || !parser.state.cursor_frontier;\n')
    text = replace(text, '        Progress::Complete(self.result)\n    }\n', f'        {completed}\n        Progress::Complete(self.result)\n    }}\n')
    text = replace(text, '                Frame::BestChoice(mut frame, mut selected, mut definition) => {\n',
                   '''                Frame::BestChoice(mut frame, mut selected, mut definition) => {
                    let work = self.steps + self.child_work;'''.rstrip() + '\n')
    text = replace(text, '                    if let Some(recording) = definition.take() {\n',
                   '''                    self.best_choice_probe.trial(parser, self.state, work - 1, input_final, self.result, winner);
                    if let Some(recording) = definition.take() {
''')
    text = replace(text, '''                    if halted {
                        self.result = Attempt::Committed;
                        continue;
                    }
                    if stop {
                        continue;
                    }
                    frame.index += 1;''', '''                    if halted {
                        self.best_choice_probe.stop_choice();
                        self.result = Attempt::Committed;
                        continue;
                    }
                    if stop {
                        self.best_choice_probe.stop_choice();
                        continue;
                    }
                    frame.index += 1;''')
    text = replace(text, '''                        self.push(Frame::BestChoice(frame, selected, definition));
                        self.push(Frame::Expression(item));
                    } else if let Some((index, _)) = selected {
                        if let Some((steps, child_work)) = definition.and_then(|recording| {''', '''                        self.best_choice_probe.begin_trial(parser, self.state, work, input_final);
                        self.push(Frame::BestChoice(frame, selected, definition));
                        self.push(Frame::Expression(item));
                    } else if let Some((index, _)) = selected {
                        self.best_choice_probe.replay(self.frames.len(), &frame.items[index], parser, self.state, work, input_final);
                        if let Some((steps, child_work)) = definition.and_then(|recording| {''')
    text = replace(text, '''                            self.child_work += child_work;
                            self.result = Attempt::Matched;
                        } else {
                            self.push(Frame::Expression(&frame.items[index]));
                        }
                    } else {
                        self.result = Attempt::NoMatch;''', '''                            self.child_work += child_work;
                            self.result = Attempt::Matched;
                            self.best_choice_probe.record_applied();
                        } else {
                            self.push(Frame::Expression(&frame.items[index]));
                        }
                    } else {
                        self.best_choice_probe.stop_choice();
                        self.result = Attempt::NoMatch;''')
    return replace(text, '''                    let definition =
                        self.record_structural_choice(parser, &items[0], input_final, allowance);
                    self.push(Frame::BestChoice(frame, None, definition));''', '''                    let definition =
                        self.record_structural_choice(parser, &items[0], input_final, allowance);
                    self.best_choice_probe.begin_choice(parser, self.state, self.steps + self.child_work, input_final);
                    self.push(Frame::BestChoice(frame, None, definition));''')


def patch_census(text):
    text = replace(text, '    replays: u64,\n', '    replays: u64,\n    applied: u64,\n    raised_peaks_and_envelope_fits: u64,\n')
    text = replace(text, '            || previous.peaks_raised\n            || replay.peaks_raised\n','')
    text = replace(text, '        self.provisional_candidates += 1;\n', '''        if previous.peaks_raised || replay.peaks_raised {
            self.raised_peaks_and_envelope_fits += 1;
        }
        if (replay_end.cursor.offset - start.cursor.offset).0 > 8_192
            || replay_end.events.saturating_sub(start.events) > 24_000
            || replay.effect.nodes > 65_536
            || replay.effect.tokens > 65_536
        {
            return;
        }
        self.provisional_candidates += 1;
''')
    text = replace(text, '''    pub(super) fn finish_ready(
''', '''    pub(super) fn record_applied(&self) {
        if !TIME {
            let family = self.replays.last().expect("existing replay scope").family;
            COUNTERS.with_borrow_mut(|counts| counts.replays.entry(family).or_default().applied += 1);
        }
    }

    pub(super) fn finish_ready(
''')
    return replace(text, '                    t.different_effects,\n',
                   '                    t.different_effects,\n                    t.applied,\n                    t.raised_peaks_and_envelope_fits,\n')


def stage():
    assert git('rev-parse', 'refs/heads/codex/native-compact-source-handoff').decode().strip() == pin
    base.mkdir()
    with tarfile.open(fileobj=io.BytesIO(git('archive', pin, syntax)), mode='r:') as stream:
        stream.extractall(base, filter='data')
    shutil.copytree(base / syntax, overlay / syntax)
    files = {}
    for path in (base / syntax).rglob('*'):
        if path.is_file():
            name = path.relative_to(base)
            assert path.read_bytes() == (checkout / name).read_bytes(), name
            files[str(name)] = digest(path)
    assert digest(input_source) == receipt['exact_tree_and_work']['source_sha256']
    check = []
    for name, change in [('continuation.rs', patch_continuation), ('mod.rs', patch_document)]:
        path = overlay / syntax / doc / name
        path.write_text(change(path.read_text()))
        check.append(path)
    phases = overlay / syntax / 'document/parser/mod.rs'
    body = phases.read_text()
    begin = '    let mut parser = Parser::new(&source, LexicalMode::CanonicalSourceFragment, config, ids);\n    // One-shot parsing'
    setup = """    #[cfg(best_choice_probe = "time")]
    let start = std::time::Instant::now();
"""
    body = replace(body, begin, setup + begin)
    old = """    canonical::document::parse_document_root(&mut parser);
    let output = parser.finish();
    finish_snapshot(source, output, ids, SyntaxKind::Document)
"""
    new = """    #[cfg(best_choice_probe = "time")]
    let started_grammar = std::time::Instant::now();
    canonical::document::parse_document_root(&mut parser);
    #[cfg(best_choice_probe = "time")]
    let ended_grammar = std::time::Instant::now();
    let output = parser.finish();
    #[cfg(best_choice_probe = "time")]
    let finished_events = std::time::Instant::now();
    let snapshot = finish_snapshot(source, output, ids, SyntaxKind::Document);
    #[cfg(best_choice_probe = "time")]
    eprintln!(
        "post-statement-profile\troot\t{}\t{}\t{}\t{}",
        (started_grammar - start).as_nanos(),
        (ended_grammar - started_grammar).as_nanos(),
        (finished_events - ended_grammar).as_nanos(),
        finished_events.elapsed().as_nanos(),
    );
    snapshot
"""
    body = replace(body, old, new)
    old = """    let sink_result = sink(&output.events, &source, ids)
        .unwrap_or_else(|_| fallback_tree(&source, ids, fallback_kind));
"""
    body = replace(body, old, setup + old + """    #[cfg(best_choice_probe = "time")]
    let finished_sink = std::time::Instant::now();
""")
    old = """    let mut snapshot = SyntaxSnapshot::new(source, sink_result.root, diagnostics);
    snapshot.stats = output.stats;
    snapshot.stats.new_node_count = snapshot.nodes.node_count() as u64;
    snapshot.restarts = build_restart_index(&snapshot);
    snapshot
"""
    new = """    #[cfg(best_choice_probe = "time")]
    let finished_diagnostics = std::time::Instant::now();
    let mut snapshot = SyntaxSnapshot::new(source, sink_result.root, diagnostics);
    snapshot.stats = output.stats;
    snapshot.stats.new_node_count = snapshot.nodes.node_count() as u64;
    #[cfg(best_choice_probe = "time")]
    let finished_tree_index = std::time::Instant::now();
    snapshot.restarts = build_restart_index(&snapshot);
    #[cfg(best_choice_probe = "time")]
    if fallback_kind == SyntaxKind::Document {
        eprintln!(
            "post-statement-profile\tfinish\t{}\t{}\t{}\t{}",
            (finished_sink - start).as_nanos(),
            (finished_diagnostics - finished_sink).as_nanos(),
            (finished_tree_index - finished_diagnostics).as_nanos(),
            finished_tree_index.elapsed().as_nanos(),
        );
    }
    snapshot
"""
    phases.write_text(replace(body, old, new))
    check.append(phases)
    path = overlay / syntax / doc / 'best_choice_probe.rs'
    path.write_text(patch_census((prior / 'best_choice_probe.rs').read_text()))
    check.append(path)
    command = ['rustfmt', '+nightly-2026-03-03', '--edition', '2024', '--config', 'skip_children=true', *check]
    run(command, here / 'rustfmt.log')
    write(here / 'source-bindings.json', {'pin': pin, 'syntax': files, 'input': digest(input_source),
                                        'diagnostic': {str(p.relative_to(overlay)): digest(p) for p in check}})


def build(mode):
    out = here / mode
    out.mkdir()
    old = json.loads((prior_standalone / 'optimized/candidate/build.json').read_text())
    root = base if mode == 'plain' else overlay
    library = out / 'libmech_syntax.rlib'
    binary = out / 'parser'
    args = [str(root / syntax / 'lib.rs') if a == str(prior_standalone / 'source/lib.rs')
            else str(library) if a.endswith('/libmech_syntax.rlib')
            else f'metadata=post-structural-replay-{mode}' if a.startswith('metadata=definition-replay')
            else a for a in old['build']]
    assert str(root / syntax / 'lib.rs') in args and str(library) in args
    if mode != 'plain':
        args += ['--cfg', f'best_choice_probe="{mode}"']
    link = [f'mech_syntax={library}' if a.startswith('mech_syntax=')
            else str(binary) if a == str(prior_standalone / 'optimized/candidate/parser')
            else a for a in old['link']]
    assert str(binary) in link
    run(args, out / 'compile.log')
    run(link, out / 'link.log')
    write(out / 'build.json', {'compile': args, 'link': link, 'library': digest(library), 'binary': digest(binary)})
    print(f'{mode} {digest(binary)}', flush=True)


if __name__ == '__main__':
    task, *args = sys.argv[1:]
    if task == 'stage':
        stage()
    elif task == 'build':
        for mode in args:
            assert mode in ('plain', 'time', 'census')
            build(mode)
