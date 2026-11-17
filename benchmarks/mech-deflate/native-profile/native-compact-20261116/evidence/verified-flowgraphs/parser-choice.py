"""Count settled ASCII rejection batches in the real base continuation."""
from difflib import unified_diff
from shutil import copytree
import sys

from production import CHECKOUT, HERE, PIN, ROADMAP, read, save, sha

sys.path.insert(0, str(ROADMAP / 'ascii-graphemes'))
from stage import GRAMMAR, RUST, run

OUT = HERE / 'parser-choice-census'
ASCII = ROADMAP / 'ascii-graphemes'
MAIN = RUST / 'document/parser/mod.rs'
BASE = RUST / 'document/parser/canonical/base/continuation.rs'

PROBE = r'''
#[derive(Default)]
struct ChoiceCounts {
    attempts: u64,
    failures: u64,
    failed_work: u64,
    settled_terminal_batches: u64,
    settled_terminal_arms: u64,
    settled_terminal_work: u64,
}

std::thread_local! {
    static CHOICES: std::cell::RefCell<std::collections::BTreeMap<&'static [RuleId], ChoiceCounts>> =
        std::cell::RefCell::default();
}

fn settled_terminal_run(parser: &Parser<'_>, rules: &[RuleId], final_input: bool) -> usize {
    if !final_input || parser.is_halted() {
        return 0;
    }
    let at = parser.offset();
    let Some(byte) = parser.cursor().byte().filter(|byte| (b' '..=b'~').contains(byte)) else {
        return 0;
    };
    let next = TextSize(at.0 + 1);
    if next >= parser.cursor().context_end() || next > parser.cursor().end()
        || !parser.source.byte_at(next).is_some_and(|byte| byte.is_ascii())
        || (at.0 > 0 && !parser.source.byte_at(TextSize(at.0 - 1)).is_some_and(|byte| byte.is_ascii()))
    {
        return 0;
    }
    rules.iter().take_while(|rule| {
        fixed_terminal_spec(**rule).is_some_and(|spec|
            spec.spacing == TerminalSpacing::Exact && spec.literal.as_bytes()[0] != byte
        )
    }).count()
}

pub(crate) fn print_choice_census() {
    CHOICES.with_borrow(|choices| {
        for (rules, row) in choices {
            eprint!("choice-census {} {} {} {} {} {}",
                row.attempts, row.failures, row.failed_work,
                row.settled_terminal_batches, row.settled_terminal_arms, row.settled_terminal_work);
            for rule in *rules {
                eprint!(" {}", rule.0);
            }
            eprintln!();
        }
    });
}

'''


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


def stage():
    control = ASCII / 'candidate/source'
    source = OUT / 'source'
    copytree(control / RUST, source / RUST)
    assert sha(source / GRAMMAR) == read(ASCII / 'production/build.json')['source_sha256']
    changes = []
    for path in (BASE, MAIN):
        old = (source / path).read_text()
        assert sha(source / path) == sha(CHECKOUT / path)
        if path == MAIN:
            before = '    canonical::document::parse_document_root(&mut parser);\n    let output = parser.finish();'
            text = replace(old, before, before.replace('\n    let output', (
                '\n    canonical::base::continuation::print_choice_census();\n    let output')))
        else:
            text = replace(old, "    Choice(&'static [RuleId], usize),",
                           "    Choice(&'static [RuleId], usize, u64, usize),")
            text = replace(text, 'pub(crate) enum Progress {', PROBE + 'pub(crate) enum Progress {')
            text = replace(text, 'self.push(Frame::Choice(rules, 0));',
                           'self.push(Frame::Choice(rules, 0, self.work, 0));')
            before = '''                Frame::Choice(rules, index) => {
                    if !self.result
                        && !parser.is_halted()
                        && let Some(rule) = rules.get(index + 1)
                    {
                        self.push(Frame::Choice(rules, index + 1));
                        self.push(Frame::Call(*rule));
                    }
                }'''
            after = '''                Frame::Choice(rules, index, start_work, mut batch_end) => {
                    let next = index + 1;
                    let batch = if index >= batch_end && !self.result {
                        settled_terminal_run(parser, &rules[next..], final_input)
                    } else {
                        0
                    };
                    CHOICES.with_borrow_mut(|counts| {
                        let row = counts.entry(rules).or_default();
                        row.attempts += 1;
                        if !self.result {
                            row.failures += 1;
                            let work = self.work - start_work;
                            row.failed_work += work;
                            if index < batch_end {
                                row.settled_terminal_arms += 1;
                                row.settled_terminal_work += work;
                            }
                        } else {
                            assert!(index >= batch_end, "a nominated terminal succeeded");
                        }
                        if batch >= 2 {
                            row.settled_terminal_batches += 1;
                            batch_end = next + batch;
                        }
                    });
                    if !self.result && !parser.is_halted() && let Some(rule) = rules.get(next) {
                        self.push(Frame::Choice(rules, next, self.work, batch_end));
                        self.push(Frame::Call(*rule));
                    }
                }'''
            text = replace(text, before, after)
        (source / path).write_text(text)
        changes.extend(unified_diff(old.splitlines(keepends=True), text.splitlines(keepends=True),
                                    f'a/{path}', f'b/{path}'))
    (OUT / 'instrumentation.patch').write_text(''.join(changes))


def main():
    OUT.mkdir()
    stage()
    control = ASCII / 'candidate'
    build = read(control / 'build.json')
    parser = OUT / 'parser'
    library = OUT / 'libmech_syntax.rlib'
    before = control / 'source' / RUST / 'lib.rs'
    after = OUT / 'source' / RUST / 'lib.rs'
    compile_args = [str(after) if arg == str(before) else str(library)
                    if arg == str(control / library.name) else 'metadata=base-choice-census'
                    if arg.startswith('metadata=') else arg for arg in build['compile']]
    link_args = [f'mech_syntax={library}' if arg.startswith('mech_syntax=') else str(parser)
                 if arg == str(control / 'parser') else arg for arg in build['link']]
    assert str(after) in compile_args and str(library) in compile_args and str(parser) in link_args
    run(compile_args, OUT / 'build.log')
    run(link_args, OUT / 'link.log')
    source = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec'
    parsed = OUT / 'parse.json'
    receipt = read(ASCII / 'measured/comparison.json')['exact_parse_report']
    assert sha(source) == receipt['source_sha256']
    run([parser, source, parsed], OUT / 'observations.log')
    observed = read(parsed)
    observed.pop('parse_seconds')
    assert observed == receipt
    keys = ('attempts', 'failures', 'failed_work', 'batches', 'eligible_arms', 'eligible_work')
    rows = []
    for line in (OUT / 'observations.log').read_text().splitlines():
        tag, *params = line.split()
        assert tag == 'choice-census', line
        stats = dict(zip(keys, map(int, params[:len(keys)]), strict=True))
        rows.append({'rules': list(map(int, params[len(keys):])), **stats})
    assert rows
    totals = {key: sum(row[key] for row in rows) for key in keys}
    families = sorted(rows, key=lambda row: -row['eligible_work'])
    report = {'source_revision': PIN, 'source': str(source), 'exact_report': observed,
              'totals': totals, 'families': families, 'compiler_sha256': sha(parser),
              'patch_sha256': sha(OUT / 'instrumentation.patch'), 'compile': compile_args, 'link': link_args,
              'scope': 'Logical counts for successive settled exact ASCII mismatches after a losing choice arm. Nested failed-work counts may overlap. Timing excluded.'}
    save(OUT / 'report.json', report)
    print(totals, families[:3], flush=True)


if __name__ == '__main__':
    main()
