#!/usr/bin/env python3
"""Author additive Uiua sources; hashing/preparation runs under the common mutex."""
import fcntl
import hashlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
BASE = SUITE / 'uiua-fannkuch-v2/sources/fast.ua'
BASE_SHA = '73d17269ba887b7eb5ab74c8cb56271c5b064460913bbcecc7d81e458297465f'
ENV = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1',
           VECLIB_MAXIMUM_THREADS='1', RAYON_NUM_THREADS='1')
FOOTER = '''Run     ← ⍢(Work|<⊃F.End F.Index)
T       ← ↧8 Total
Bounds  ← ⌊÷T ×Total⇡+1T
Ranges  ← ⍉⊟ ⊃(↙T|↘1) Bounds
Worker  ← □ Run F ⊃(⊢|⊣|⋅0|⋅0)
Handles ← ≡spawnWorker Ranges
Joined  ← ≡°□ wait Handles
&p /+ ≡F.Checksum Joined
&p $"Pfannkuchen(_) = _" N /↥ ≡F.Maximum Joined
'''
BLOCK_GENERATOR = '''~Rotation {Width Amounts}
ColumnMask         ← ¤ <⊃Rotation.Width(⋅(⇡N))
Shifted            ← ◿⊃Rotation.Width(⊞+⊃Rotation.Amounts(⋅(⇡N)))
RotationIndices    ← +⊙(¤⇡N) ×⊃ColumnMask(-⊃(⋅(¤⇡N))Shifted)
~RotateWork {Stage Permutations}
RotateGather       ← ↯⊃(△RotateWork.Permutations)(⊏⊃(♭+⊃(×N⇡⧻ RotateWork.Permutations)(RotationIndices RotateWork.Stage))(♭RotateWork.Permutations))
RotateRows         ← RotateGather RotateWork
Rotations          ← ≡Rotation Widths ◿(≡¤ Widths)⌊⊞÷ Divisors
InitialPermutations ← ↯⊃(⊂⊃⧻(⋅N))(⋅(⇡N))
PermutationBlock   ← ∧RotateRows ⊃Rotations InitialPermutations
'''


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def source_texts():
    lines = BASE.read_text().splitlines()
    assert len(lines) == 21
    assert lines[-3:] == ['Result       ← ⍢(Work|<Total F.Index) F 0 0 0',
                         '&p F.Checksum Result', '&p $"Pfannkuchen(_) = _" N F.Maximum Result']
    prefix = '\n'.join(lines[:-3]) + '\n'
    prefix = prefix.replace('~F {Index Checksum Maximum}', '~F {Index End Checksum Maximum}')
    prefix = prefix.replace('(F.Index|⋅Total)', '(F.Index|F.End)')
    prefix = prefix.replace('|+⊃(F.Checksum', '|F.End Block.State|+⊃(F.Checksum')
    parallel = prefix + FOOTER
    block_lines = []
    for line in prefix.splitlines():
        if line.startswith('Permutation '):
            block_lines.extend(BLOCK_GENERATOR.rstrip().splitlines())
        elif line.startswith('Rotate '):
            continue
        else:
            block_lines.append(line.replace('Flips≡Permutation Ranks.Indices',
                                           'Flips PermutationBlock Ranks.Indices'))
    blocked = '\n'.join(block_lines) + '\n' + FOOTER
    assert len(parallel.splitlines()) == 27
    assert len(blocked.splitlines()) == 35
    return {'parallel': parallel, 'block': blocked}


def prepare():
    assert sha(BASE) == BASE_SHA, 'Retained fast-v2 source changed'
    (HERE / 'sources').mkdir(parents=True, exist_ok=True)
    for name, text in source_texts().items():
        target = HERE / 'sources' / (name + '.ua')
        if target.exists():
            assert target.read_text() == text, target
        else:
            target.write_text(text)
        print(name, len(text.splitlines()), 'LOC', sha(target), flush=True)


if __name__ == '__main__':
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        prepare()
