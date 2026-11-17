"""Stage the sealed ASCII grapheme experiment from the current review tree."""
from difflib import unified_diff
from hashlib import sha256
from itertools import permutations
import json
import os
from pathlib import Path
import shutil
from statistics import median
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROOT = Q.parent
PIN = '7a0b3127ce2ec163d3989f706b7ab819901ec0c2'
CHECKOUT = Q / 'native-review-pr/checkout'
SAMPLE = Path((HERE.parent / 'post-statement-profile-path').read_text().strip())
RUST = Path('src/syntax/src')
GRAMMAR = RUST / 'document/parser/grapheme_scan.rs'
ENV = os.environ | {'CARGO_MANIFEST_DIR': str(CHECKOUT / 'src/syntax'), 'CARGO_PKG_VERSION': '0.3.5'}
EXPECTED = SAMPLE / 'observed/00-plain.json'
ROLES = ('control', 'expected', 'candidate')


def once(body, old, new):
    assert body.count(old) == 1, old
    return body.replace(old, new, 1)


def sha(path):
    return sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def run(argv, log):
    argv = list(map(str, argv))
    with log.open('wb') as out:
        subprocess.run(argv, env=ENV, cwd=CHECKOUT, stdout=out, stderr=subprocess.STDOUT, check=True)
    return argv


def patch(body, use_source_scanner):
    original_fast_entry = '''        if let Some(text) = source.contiguous_without_lookup_work() {
            return self.advance_in(text, end, final_input, allowance);
        }
        self.advance_in(source, end, final_input, allowance)
'''
    # The matched ablation uses the same fast operation on literals. Only
    # the TextSnapshot path stays on the ordinary source scanner in that arm.
    direct = '''
            if final_input && let Some(progress) = self.advance_sealed_ascii(text, end, allowance) {
                return progress;
            }'''
    body = once(body, original_fast_entry, original_fast_entry.replace(
        ' {\n            return self.advance_in(text', ' {' + direct + '\n            return self.advance_in(text'))
    body = once(body, 'impl ScanSource for str {\n',
                '''impl ScanSource for str {
    fn contiguous_without_lookup_work(&self) -> Option<&str> {
        Some(self)
    }
''')
    if not use_source_scanner:
        start = 'impl ScanSource for TextSnapshot {\n'
        old = '''    fn contiguous_without_lookup_work(&self) -> Option<&str> {
        TextSnapshot::contiguous_without_lookup_work(self)
    }
'''
        # Keep TextSnapshot's borrowed contiguous scan with the original logic.
        body = once(body, start + old, start + old + '''    fn allows_ascii_step(&self) -> bool {
        false
    }
''')
        body = once(body, 'pub(crate) trait ScanSource {\n', '''pub(crate) trait ScanSource {
    fn allows_ascii_step(&self) -> bool {
        true
    }
''')
        body = once(body, '            if final_input && let Some(progress)',
                    '            if source.allows_ascii_step() && final_input && let Some(progress)')
    body = once(body, '    fn advance_in<S: ScanSource + ?Sized>(\n', '''    fn advance_sealed_ascii(
        &mut self,
        source: &str,
        end: TextSize,
        allowance: &mut u64,
    ) -> Option<ScanProgress> {
        let start = self.start.to_usize();
        if self.final_end != Some(end) || self.context.is_some() || self.cursor.cur_cursor() != start {
            return None;
        }
        let text = source.get(..end.to_usize())?;
        let bytes = text.as_bytes();
        let first = *bytes.get(start)?;
        if !first.is_ascii() {
            return None;
        }
        // CRLF is the only pair of adjacent ASCII scalars in one grapheme.
        // An ASCII lookahead also gives the cursor a settled next origin.
        let width = if first == b'\\r' && bytes.get(start + 1) == Some(&b'\\n') {
            2
        } else {
            1
        };
        let boundary = start + width;
        let calls = match bytes.get(boundary) {
            None => width as u64,
            Some(byte) if byte.is_ascii() => (width + 1) as u64,
            Some(_) => return None,
        };
        if *allowance < calls {
            return None;
        }

        // The ordinary scanner charges a bounded Unicode call for each
        // forward scalar and for the lookahead at a non-final boundary.
        let preceding = text[..start].chars().next_back().map_or(0, char::len_utf8);
        *allowance -= calls;
        self.work.calls += calls;
        self.work.forward_bytes += calls;
        self.work.lookbehind_bytes += preceding as u64 + calls - 1;
        self.cursor.set_cursor(boundary);
        let next = TextSize(boundary as u32);
        let range = TextRange::new(self.start, next);
        self.start = next;
        Some(ScanProgress::Grapheme(range))
    }

    fn advance_in<S: ScanSource + ?Sized>(
''')
    if use_source_scanner:
        body = once(body, '    fn drain(\n', '''    #[test]
    fn sealed_ascii_preserves_poll_partitions_and_scanner_work() {
        for text in [
            "",
            "abc",
            "\\r\\n\\r\\t\\0",
            "a\\r\\nb",
            "éa\\r\\nβ",
            "abc\\wxyz",
            "ae\\u{301}b",
            "a👩\\u{200d}💻b",
            "a🇦🇧🇨z",
        ] {
            for start in (0..=text.len()).filter(|&i| text.is_char_boundary(i)) {
                for end in (start..=text.len()).filter(|&i| text.is_char_boundary(i)) {
                    for budget in 1..=5 {
                        let start = TextSize(start as u32);
                        let end = TextSize(end as u32);
                        let mut reference = GraphemeScan::new(start, Some(end));
                        let mut shortcut = reference.clone();
                        for poll in 0.. {
                            let mut a = if poll == 0 { 0 } else { budget };
                            let mut b = a;
                            let expected = reference.advance_in(text, end, true, &mut a);
                            let observed = shortcut.advance(text, end, true, &mut b);
                            assert_eq!((observed, b, shortcut.work), (expected, a, reference.work), "{text:?} {start:?} {end:?} {budget}");
                            if observed == ScanProgress::End {
                                break;
                            }
                            assert!(!matches!(observed, ScanProgress::InvalidSource | ScanProgress::NeedInput));
                        }
                    }
                }
            }
        }
    }

    fn drain(
'''.replace('            "abc\\wxyz",\n',''))
    return body


def stage():
    latest = subprocess.check_output(
        ['git','rev-parse','refs/heads/codex/native-compact-source-handoff'], cwd=ROOT).decode().strip()
    assert latest == PIN
    old = subprocess.check_output(['git','show',f'{PIN}:{GRAMMAR}'], cwd=ROOT)
    assert (CHECKOUT / GRAMMAR).read_bytes() == old == (SAMPLE / 'pinned' / GRAMMAR).read_bytes()
    for role in ROLES[1:]:
        output = HERE / role
        shutil.copytree(SAMPLE / 'pinned' / RUST, output / 'source' / RUST)
        path = output / 'source' / GRAMMAR
        path.write_text(patch(old.decode(), role != 'expected'))
        run(['rustfmt','+nightly-2026-03-03','--edition','2024',
             '--config','skip_children=true',path], output / 'fmt.log')
        (output / 'change.patch').write_text(''.join(unified_diff(
            old.decode().splitlines(keepends=True), path.read_text().splitlines(keepends=True),
            f'a/{GRAMMAR}',f'b/{GRAMMAR}')))
        save(output / 'source.json', {
            'base': PIN, 'source_before': sha(CHECKOUT / GRAMMAR), 'source_after': sha(path),
            'patch_sha256': sha(output / 'change.patch'),
        })


def build(role):
    output = HERE / role
    work = output / 'source' / RUST / 'lib.rs'
    library = output / 'libmech_syntax.rlib'
    binary = output / 'parser'
    reference = read(SAMPLE / 'plain/build.json')
    baseline_src = SAMPLE / 'pinned' / RUST / 'lib.rs'
    baseline_lib = SAMPLE / 'plain/libmech_syntax.rlib'
    assert sha(baseline_lib) == reference['library']
    reference_parser = SAMPLE / 'plain/parser'
    assert sha(reference_parser) == reference['binary']
    assert str(baseline_src) in reference['compile']
    assert str(baseline_lib) in reference['compile']
    assert str(reference_parser) in reference['link']
    argv = [str(work) if arg == str(baseline_src) else str(library) if arg == str(baseline_lib)
            else 'metadata=ascii-graphemes-' + role if arg.startswith('metadata=')
            else arg for arg in reference['compile']]
    link = [f'mech_syntax={library}' if arg.startswith('mech_syntax=')
            else str(binary) if arg == str(reference_parser) else arg for arg in reference['link']]
    save(output / 'build.json', {
        'compile': run(argv, output / 'build.log'), 'link': run(link, output / 'link.log'),
        'library_sha256': sha(library), 'binary_sha256': sha(binary),
    })
    if role == 'candidate':
        test()


def test():
    output = HERE / 'candidate'
    work = output / 'source' / RUST / 'lib.rs'
    support = Path('src/core/tests/support/allocation_probe.rs')
    staged = output / 'source' / support
    staged.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(CHECKOUT / support, staged)
    fixtures = Path('src/syntax/tests')
    shutil.copytree(CHECKOUT / fixtures, output / 'source' / fixtures, dirs_exist_ok=True)
    receipt = Path('/private/tmp/mech-dense-index.5noolrp2/validation/receipt.json')
    test_args = read(receipt)['tests']['build']
    path = receipt.parent / 'syntax-tests'
    assert str(path) in test_args
    test_binary = output / 'syntax-tests'
    command = [str(work) if arg.endswith('/source/lib.rs') else str(test_binary) if arg == str(path)
               else 'metadata=ascii-graphemes-tests' if arg.startswith('metadata=')
               else arg for arg in test_args]
    assert str(work) in command and str(test_binary) in command
    run(command, output / 'test-build.log')
    run([test_binary,'--test-threads=1'], output / 'tests.log')
    lines = (output / 'tests.log').read_text().splitlines()
    summary = [line for line in lines if line.startswith('test result:')]
    assert len(summary) == 1 and '0 failed' in summary[0]
    save(output / 'tests.json', {'build':command,'binary_sha256':sha(test_binary),'summary':summary[0]})
    print(summary[0], flush=True)


def measure():
    output = HERE / 'measured'
    output.mkdir()
    source = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec'
    control = SAMPLE / 'plain/parser'
    expect = read(EXPECTED)
    expect.pop('parse_seconds')
    assert sha(source) == expect['source_sha256']
    binaries = {'control': control, **{role: HERE / role / 'parser' for role in ROLES[1:]}}
    orders = list(permutations(ROLES))
    hashes = {name:sha(binary) for name,binary in binaries.items()}
    assert len(set(hashes.values())) == len(ROLES)
    save(output / 'plan.json', {'base': PIN, 'source_sha256': sha(source), 'orders':orders, 'binaries':hashes})

    def trial(name, label):
        path = output / f'{label}-{name}.json'
        run([binaries[name], source, path], output / f'{label}-{name}.log')
        measured = read(path)
        seconds = measured.pop('parse_seconds')
        assert measured == expect, name
        return seconds

    warmup = {name:trial(name,'warmup') for name in ROLES}
    rows = [{name:trial(name,f'order-{i}') for name in order} for i,order in enumerate(orders,1)]
    result = {
        'plan_sha256':sha(output / 'plan.json'), 'warmup_s': warmup, 'trials_s': rows,
        'median_s': {role:median(row[role] for row in rows) for role in ROLES},
        'pair_ratio':{role:median(row[role]/row['control'] for row in rows) for role in ROLES[1:]},
        'exact_parse_report': expect,
    }
    save(output / 'comparison.json', result)
    print(result, flush=True)


if __name__ == '__main__':
    for task in sys.argv[1:]:
        {'stage':stage,'build-expected':lambda:build('expected'),
         'build-candidate':lambda:build('candidate'),'test':test,'measure':measure}[task]()
