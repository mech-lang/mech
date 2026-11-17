"""Stage an ASCII literal rejection shortcut with exact cache and work effects."""

from difflib import unified_diff
from hashlib import sha256
from importlib import import_module
import json
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROOT = Q.parent
CHECKOUT = Q / 'native-review-pr/checkout'
BASE = 'd03f097b06c8386822a8063de9826715a9c1e897'
SYNTAX = Path('src/syntax')
PARSER = SYNTAX / 'src/document/parser'
REFERENCE = HERE.parent / 'ascii-graphemes'
OUT = HERE / 'candidate'


def change(body, old, new):
    assert body.count(old) == 1, old
    return body.replace(old, new)


def literal_cache(body):
    body = change(body, "    fn slot(literal: &str, start: TextSize) -> usize {", '''    /// Complete a fresh, sealed single-grapheme ASCII mismatch. The normal
    /// scan charges one expected-scalar call, one prefix byte, two source
    /// calls including lookahead, and one compared byte.
    pub fn sealed_ascii_miss<S: ScanSource + ?Sized>(
        &mut self,
        source: &S,
        literal: &'static str,
        start: TextSize,
        consume_end: TextSize,
        context_end: TextSize,
        allowance: u64,
    ) -> Option<(Option<TextSize>, u64)> {
        const SCAN_WORK: u64 = 5;
        if allowance < SCAN_WORK + 3 {
            return None;
        }
        let [wanted] = literal.as_bytes() else {
            return None;
        };
        if !(b' '..=b'~').contains(wanted) {
            return None;
        }
        let text = source.contiguous_without_lookup_work()?.get(..context_end.to_usize())?;
        if !text.is_char_boundary(consume_end.to_usize()) {
            return None;
        }
        let bytes = text.as_bytes();
        let at = start.to_usize();
        let next = at.checked_add(1)?;
        if next > consume_end.to_usize() {
            return None;
        }
        let actual = *bytes.get(at)?;
        if actual == *wanted || !(b' '..=b'~').contains(&actual)
            || !bytes.get(next)?.is_ascii()
            || (at > 0 && !bytes[at - 1].is_ascii())
        {
            return None;
        }
        self.remember(Self::slot(literal, start), CacheEntry {
            literal, start, consume_end, context_end, result: None, work: SCAN_WORK,
        });
        Some((None, SCAN_WORK))
    }

    fn remember(&mut self, index: usize, entry: CacheEntry) {
        if self.slots.is_empty() {
            self.slots.resize(CACHE_CAPACITY, None);
        }
        self.slots[index] = Some(entry);
    }

    fn slot(literal: &str, start: TextSize) -> usize {''')
    # The scanner already computed the destination slot on its hit path.
    # Store through the same helper so both paths have the same cache effect.
    body = change(body, '''            if cache.slots.is_empty() {
                cache.slots.resize(CACHE_CAPACITY, None);
            }
            cache.slots[index] = Some(CacheEntry {''', '''            cache.remember(index, CacheEntry {''')
    return change(body, '    #[test]\n    fn cached_scans_preserve_unicode_results_and_every_allowance_boundary() {',
                  '''    #[test]
    fn sealed_ascii_miss_matches_the_scan_and_cache_work() {
        for (text, start, expected, accepts) in [
            ("a+b", 1, "-", true),
            ("a+b", 1, "+", false),
            ("a+b", 1, "--", false),
            ("a+\\u{0301}b", 1, "-", false),
            ("é+b", 2, "-", false),
            ("a+", 1, "-", false),
            ("x+", 0, "-", true),
        ] {
            let start = TextSize(start);
            let end = TextSize(text.len() as u32);
            for budget in 0..=10 {
                let mut candidate = LiteralCache::default();
                let admitted = candidate.sealed_ascii_miss(
                    text, expected, start, end, end, budget,
                );
                assert_eq!(admitted.is_some(), accepts && budget >= 8, "{text:?} {budget}");
                if let Some((result, charged)) = admitted {
                    let mut ordinary = LiteralCache::default();
                    let mut scan = LiteralScan::new(expected, start, Some(end)).unwrap();
                    let mut remaining = budget;
                    assert_eq!(
                        scan.advance_cached(text, end, end, true, &mut remaining, &mut ordinary),
                        LiteralProgress::Complete(result),
                    );
                    assert_eq!(budget - remaining, charged);
                    assert_eq!(
                        ordinary.replay(expected, start, end, end, budget),
                        candidate.replay(expected, start, end, end, budget),
                    );
                }
            }
        }
    }

    #[test]
    fn cached_scans_preserve_unicode_results_and_every_allowance_boundary() {''')


def base_call(body):
    return change(body, '''            && let Some((end, work)) = parser.state.literal_cache.replay(
                spec.literal,
                parser.offset(),
                parser.cursor().end(),
                parser.cursor().context_end(),
                *allowance,
            )''', '''            && let Some((end, work)) = {
                let start = parser.offset();
                let consume = parser.cursor().end();
                let context = parser.cursor().context_end();
                parser.state.literal_cache
                    .replay(spec.literal, start, consume, context, *allowance)
                    .or_else(|| {
                        if parser.is_halted() || parser.state.resource_finalizing {
                            return None;
                        }
                        parser.state.literal_cache.sealed_ascii_miss(
                            parser.source, spec.literal, start, consume, context, *allowance,
                        )
                    })
            }''')


def sha(path):
    return sha256(path.read_bytes()).hexdigest()


def stage():
    OUT.mkdir()
    shutil.copytree(CHECKOUT / SYNTAX / 'src', OUT / 'source' / SYNTAX / 'src')
    changes = {PARSER / 'literal_scan.rs': literal_cache,
               PARSER / 'canonical/base/continuation.rs': base_call}
    patch = []
    hashes = {}
    for path, edit in changes.items():
        base = subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT).decode()
        original = OUT / 'source' / path
        assert original.read_text() == base
        new = edit(base)
        original.write_text(new)
        hashes[str(path)] = sha(original)
        patch.extend(unified_diff(base.splitlines(keepends=True), new.splitlines(keepends=True),
                                  f'a/{path}', f'b/{path}'))
    (HERE / 'literal-miss.patch').write_text(''.join(patch))
    (HERE / 'source.json').write_text(json.dumps({'base': BASE, 'source_sha256': hashes}, indent=2) + '\n')


def build():
    sys.path.insert(0, str(REFERENCE))
    workflow = import_module('stage')
    workflow.HERE = HERE
    workflow.build('candidate')


if __name__ == '__main__':
    for step in sys.argv[1:]:
        {'stage': stage, 'build': build}[step]()
