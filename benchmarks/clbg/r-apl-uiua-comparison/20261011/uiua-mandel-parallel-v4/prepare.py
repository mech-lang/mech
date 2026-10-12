#!/usr/bin/env python3
"""Prepare isolated ordinary-thread variants, only while holding the common mutex."""
import fcntl
import hashlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
BASES = {
    'whole': (SUITE / 'uiua-mandelbrot-v2/sources/fast.ua',
              '64f2913ec83009b70cc3ac8c4064438732bc1faf4448af198b7052e4cb0aaa4e', 11),
    'active': (SUITE / 'uiua-mandel-active-v3/sources/fast.ua',
               '5a3397ac99a36bdff8bf834b5f1c81ed2d53ab969e4f80994ab4870172c9ca65', 14),
}
FOOTER = '''T ← ↧8N
Groups ← ⊕□⊸(◿T)⇡N
Worker ← □ ≡Row -1 ×2÷N °□
Handles ← ≡spawnWorker Groups
Order ← /◇⊂ Groups
&p "P4"
&p $"_ _" N N
&fwa "/dev/stdout" ⊏ (⍏Order) /◇⊂ wait Handles
'''


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def prepare():
    sources = HERE / 'sources'
    sources.mkdir(parents=True, exist_ok=True)
    for name, (base, digest, loc) in BASES.items():
        assert sha(base) == digest
        lines = base.read_text().splitlines()
        assert len(lines) == loc
        assert lines[-3:] == ['&p "P4"', '&p $"_ _" N N', '≡Row -1 ×2÷N⇡N']
        prefix = '\n'.join(lines[:-3]).replace('&fwa "/dev/stdout" Pack', 'Pack')+'\n'
        text = prefix + FOOTER
        assert len(text.splitlines()) == loc+5
        target = sources / (name+'.ua')
        target.write_text(text)
        print(name, loc+5, 'LOC', sha(target), flush=True)


if __name__ == '__main__':
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        prepare()
