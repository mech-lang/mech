#!/usr/bin/env python3
"""Make an additive denominator placement revision from canonical-only v2."""
from pathlib import Path
import hashlib

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
PINS = {
    'compact': 'ce4a4d559ea8e67eb5feb1474f41caa7e06c196c16ad637ab0fe67b0d7542b41',
    'fast': '70017b830776a4299a715f189eabc3600385e7c729d7997d539c93f710ae509d',
}
OLD = '(≡≡(¤√ⁿ3)+Diagonal Norm)'
NEW = '(≡≡¤ √ⁿ3+Diagonal Norm)'


def main():
    (HERE / 'sources').mkdir(exist_ok=True)
    for variant, digest in PINS.items():
        original = SUITE / 'nbody-canonical-v2/sources/uiua' / (variant + '.ua')
        text = original.read_text()
        assert hashlib.sha256(original.read_bytes()).hexdigest() == digest
        assert text.count(OLD) == 1
        assert 'Norm         ← ≡≡/+˙×' in text
        source = HERE / 'sources' / (variant + '.ua')
        source.write_text(text.replace(OLD, NEW))
        print(source, hashlib.sha256(source.read_bytes()).hexdigest())


if __name__ == '__main__':
    main()
