"""Account for constant literal-side grapheme work and cold cache collisions."""
from hashlib import file_digest
import json
from pathlib import Path
import shutil
import subprocess
import sys

from driver import checkout, env, here, input_source, replace, syntax, write

AREA = here / 'literal-census'
PARSER = syntax / 'document/parser'
FIX = PARSER / 'literal_scan.rs'
MAIN = PARSER / 'mod.rs'
PAST = here / 'plain'


def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def stage():
    root = here / 'pinned'
    overlay = AREA / 'overlay'
    shutil.copytree(root / syntax, overlay / syntax)
    for path in (FIX, MAIN):
        assert sha(root / path) == sha(checkout / path)
    literal = overlay / FIX
    original = literal.read_text()
    counters = '''
#[derive(Default)]
struct LiteralCensus {
    hits: u64,
    cold_scans: u64,
    expected_calls: u64,
    actual_calls: u64,
    other_work: u64,
    ascii_literal_cold_scans: u64,
    ascii_literal_expected_calls: u64,
    seen: std::collections::BTreeSet<(&'static str, TextSize, TextSize, TextSize)>,
    repeated_misses: u64,
    repeated_miss_calls: u64,
    literal_phases: std::collections::BTreeMap<&'static str, [u64; 3]>,
}

std::thread_local! {
    static CENSUS: std::cell::RefCell<LiteralCensus> = std::cell::RefCell::default();
}

pub(crate) fn print_cold_census() {
    CENSUS.with_borrow(|counts| {
        eprintln!("literal-census {} {} {} {} {} {} {} {} {} {} {}",
            counts.hits, counts.cold_scans, counts.expected_calls, counts.actual_calls,
            counts.other_work, counts.ascii_literal_cold_scans, counts.ascii_literal_expected_calls,
            counts.seen.len(), counts.repeated_misses, counts.repeated_miss_calls,
            counts.literal_phases.len());
        for (literal, work) in &counts.literal_phases {
            eprintln!("literal-row {} {} {} {} {:?}", literal.len(), work[0], work[1], work[2], literal);
        }
    });
}

'''
    text = replace(original, 'const CACHE_CAPACITY: usize = 4096;',
                   counters + 'const CACHE_CAPACITY: usize = 4096;')
    text = replace(text, '        Some((result, work))\n',
                   '        CENSUS.with_borrow_mut(|counts| counts.hits += 1);\n        Some((result, work))\n')
    text = replace(text, '''            *allowance -= work;
            return self.complete(result);
''', '''            *allowance -= work;
            CENSUS.with_borrow_mut(|counts| counts.hits += 1);
            return self.complete(result);
''')
    text = replace(text, '''        if let LiteralProgress::Complete(result) = result {
            if cache.slots.is_empty() {''', '''        if let LiteralProgress::Complete(result) = result {
            let expected = self.expected.work.calls;
            let actual = self.actual.work.calls;
            CENSUS.with_borrow_mut(|counts| {
                counts.cold_scans += 1;
                counts.expected_calls += expected;
                counts.actual_calls += actual;
                counts.other_work += before - *allowance - expected - actual;
                if self.literal.is_ascii() {
                    counts.ascii_literal_cold_scans += 1;
                    counts.ascii_literal_expected_calls += expected;
                }
                if !counts.seen.insert((self.literal, start, consume_end, context_end)) {
                    counts.repeated_misses += 1;
                    counts.repeated_miss_calls += expected + actual;
                }
                let row = counts.literal_phases.entry(self.literal).or_default();
                row[0] += 1;
                row[1] += expected;
                row[2] += actual;
            });
            if cache.slots.is_empty() {''')
    literal.write_text(text)
    mod_rs = overlay / MAIN
    body = mod_rs.read_text()
    body = replace(body, '''    canonical::document::parse_document_root(&mut parser);
    let output = parser.finish();''',
                   '''    canonical::document::parse_document_root(&mut parser);
    literal_scan::print_cold_census();
    let output = parser.finish();''')
    mod_rs.write_text(body)
    write(AREA / 'overlay.json', {'original': {str(path):sha(root / path) for path in (FIX,MAIN)},
                                  'diagnostic': {str(path):sha(overlay / path) for path in (FIX,MAIN)}})


def run():
    binary = AREA / 'parser'
    library = AREA / 'libmech_syntax.rlib'
    build = json.loads((PAST / 'build.json').read_bytes())
    args = [str(AREA / 'overlay' / syntax / 'lib.rs') if p == str(here / 'pinned' / syntax / 'lib.rs')
            else str(library) if p == str(PAST / library.name)
            else 'metadata=post-statement-literal-census' if p == 'metadata=post-structural-replay-plain'
            else p for p in build['compile']]
    assert str(library) in args and str(AREA / 'overlay' / syntax / 'lib.rs') in args
    link = [f'mech_syntax={library}' if p.startswith('mech_syntax=')
            else str(binary) if p == str(PAST / 'parser') else p for p in build['link']]
    assert str(binary) in link
    for name,cmd in [('build',args), ('link',link)]:
        with (AREA / f'{name}.log').open('wb') as log:
            subprocess.run(cmd, env=env, cwd=checkout, stdout=log, stderr=subprocess.STDOUT, check=True)
    observed = AREA / 'parse.json'
    with (AREA / 'counts.log').open('wb') as log:
        subprocess.run([binary, input_source, observed], env=env, cwd=checkout,
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    actual = json.loads(observed.read_bytes())
    original = json.loads((here / 'observed/00-plain.json').read_bytes())
    assert actual.pop('parse_seconds') > 0 and original.pop('parse_seconds') > 0
    assert actual == original
    totals, *rows = (AREA / 'counts.log').read_text().splitlines()
    tag, *values = totals.split()
    assert tag == 'literal-census'
    keys = ('cache_hits', 'cold_scans', 'expected_unicode_calls', 'actual_unicode_calls',
            'prefix_and_byte_compare_work', 'ascii_literal_cold_scans', 'ascii_expected_unicode_calls',
            'distinct_completed_keys', 'repeated_cold_misses', 'unicode_calls_at_repeated_misses',
            'distinct_static_literals')
    counts = dict(zip(keys,map(int,values),strict=True))
    assert counts['distinct_static_literals'] == len(rows)
    receipt = {'counts': counts, 'observed_exact_parse':actual,'build':args,'link':link,
               'library_sha256':sha(library),'binary_sha256':sha(binary)}
    write(AREA / 'report.json', receipt)
    print(counts, flush=True)


if __name__ == '__main__':
    {'stage':stage, 'run':run}[sys.argv[1]]()
