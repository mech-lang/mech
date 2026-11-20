"""Keep every adaptive-block threshold; evaluate pure predicates conditionally."""
from hashlib import sha256
BASE_SHA = 'ecf98df0c4f0ea9bfd2b5ae548a76fdfba178703c224a6e33e987848711a4b22'
SOURCE_SHA = 'cb8f5acae98624c6d8ad37541cc70d6dad04914a324b637baa8c158e375f3489'
BEFORE = '  check := !at-limit && (number-fresh >= 512u64) && (bits/wrapping-sub(position, begin) >= 5000u64) && (bits/wrapping-sub(end, position) >= 5000u64)'
AFTER = '  check := (at-limit?\n    | true => false\n    | false => ((number-fresh >= 512u64)?\n      | false => false\n      | true => ((bits/wrapping-sub(position, begin) >= 5000u64)?\n        | false => false\n        | true => bits/wrapping-sub(end, position) >= 5000u64)))'

def generate(source):
    assert sha256(source.encode()).hexdigest() == BASE_SHA
    assert source.count(BEFORE) == 1
    result = source.replace(BEFORE, AFTER)
    assert sha256(result.encode()).hexdigest() == SOURCE_SHA
    return result
