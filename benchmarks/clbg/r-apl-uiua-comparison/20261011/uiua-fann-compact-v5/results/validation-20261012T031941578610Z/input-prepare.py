#!/usr/bin/env python3
"""Reproduce unqualified composed candidates without modifying retained inputs."""
import fcntl
import hashlib
from pathlib import Path
HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
BASE = SUITE / 'uiua-fannkuch-v2/sources/fast.ua'
BASE_SHA = '73d17269ba887b7eb5ab74c8cb56271c5b064460913bbcecc7d81e458297465f'
ENV = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1', RAYON_NUM_THREADS='1')
MASK = '(¤<⊃⊢(⋅(⇡N)))'
SHIFT = '(◿⊃⊢(⊞+⊃(↘1)(⋅(⇡N))))'
INDICES = '(+⊙(¤⇡N) ×⊃' + MASK + '(-⊃(⋅(¤⇡N))' + SHIFT + '))'
ROTATE = 'Rotate      ← ↯⊃(⋅△)(⊏⊃(♭+⊃(⋅(×N⇡⧻))' + INDICES + ')(⋅♭))'
PERMUTATION = 'Permutation ← ∧Rotate ⊃(≡⊂ Widths ◿(≡¤ Widths)⌊⊞÷ Divisors)(↯⊃(⊂⊃⧻(⋅N))(⋅(⇡N)))'
WORKER = '□ ⍢(Work|<⊃F.End F.Index) F ⊃(⊢|⊣|⋅0|⋅0)'
RANGES = '⍉⊟ ⊃(↙↧8Total)(↘1) ⌊÷↧8Total ×Total⇡+1↧8Total'
RESULT = 'Result       ← ≡°□ wait ≡spawn(' + WORKER + ') ' + RANGES


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def source_texts():
    result = {}
    for family, loc in [('compact', 20), ('fast', 21)]:
        lines = (SUITE / 'uiua-fannkuch-v2/sources' / (family + '.ua')).read_text().splitlines()
        assert len(lines) == loc
        out = []
        for line in lines:
            if line.startswith('Rotate '): line = ROTATE
            elif line.startswith('Permutation '): line = PERMUTATION
            elif line.startswith('Result '): line = RESULT
            elif line.startswith('&p F.Checksum'): line = '&p /+ ≡F.Checksum Result'
            elif line.startswith('&p $'): line = '&p $"Pfannkuchen(_) = _" N /↥ ≡F.Maximum Result'
            line = line.replace('~F {Index Checksum Maximum}', '~F {Index End Checksum Maximum}')
            line = line.replace('(F.Index|⋅Total)', '(F.Index|F.End)')
            line = line.replace('|+⊃(F.Checksum', '|F.End Block.State|+⊃(F.Checksum')
            line = line.replace('Flips≡Permutation Ranks.Indices', 'Flips Permutation Ranks.Indices')
            out.append(line)
        assert len(out) == loc
        result[family] = '\n'.join(out) + '\n'
    return result


def prepare():
    assert sha(BASE) == BASE_SHA
    assert sha(SUITE / 'uiua-fannkuch-v2/sources/compact.ua') == '410c84084a8edfafe84a5a11456f63215b36710e7844adaf9ca916372aa3e9f1'
    for family, text in source_texts().items():
        path = HERE / 'sources' / (family + '.ua')
        assert path.read_text() == text, ('Authored candidate differs from derivation', path)
        print(family, len(text.splitlines()), 'LOC', sha(path), flush=True)


if __name__ == '__main__':
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        prepare()
