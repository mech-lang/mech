"""Actual candidate operands and both divisions, with independent recurrence reconciliation."""
import hashlib


def copy(source, variant):
    anchor = 'while printed < n:\n'
    assert source.count(anchor) == 1 and 'MECH_NIM_WORK_TRACE' not in source
    helper = ''
    if variant == 'optimized':
        before = 'import "../../support/gmp_ops"'
        assert source.count(before) == 1
        source = source.replace(before, 'import "support/gmp_ops"')
        helper = '''proc auditDecimal(value: var Mpz): string =
  var buffer = newString(int(mpz_sizeinbase(value, 10)) + 2)
  let written = mpz_get_str(cast[cstring](addr buffer[0]), 10, value)
  result = $written
'''
    setup = helper + '''let audit = open(getEnv("MECH_NIM_WORK_TRACE"), fmWrite)
audit.writeLine("PI_NIM1|VARIANT|", n)
'''.replace('VARIANT', variant)
    state = '  audit.writeLine("STATE|", q, "|", r, "|", t, "|", k, "|", printed)\n'
    divisions = '  audit.writeLine("DIV|", numerator3, "|", digit3, "|", numerator4, "|", digit4)\n'
    if variant == 'optimized':
        for name in ('q', 'r', 't'):
            state = state.replace(', ' + name + ',', ', auditDecimal(' + name + '),')
        for name in ('numerator3', 'digit3', 'numerator4', 'digit4'):
            divisions = divisions.replace(', ' + name, ', auditDecimal(' + name + ')')
        second = '  divide(digit4, numerator4, t)\n'
    else:
        second = '  let digit4 = numerator4 div t\n'
    assert source.count(second) == 1
    source = source.replace(anchor, setup + anchor + state).replace(second, second + divisions)
    return source + '\naudit.writeLine("END_PI_NIM1|", printed)\naudit.close()\n'


def reconcile(path, n, variant):
    lines = path.read_text().splitlines()
    assert lines[0] == f'PI_NIM1|{variant}|{n}'
    assert lines[-1].startswith('END_PI_NIM1|')
    assert int(lines[-1].split('|')[-1]) == n
    assert (len(lines) - 2) % 2 == 0
    q, r, t, k, printed = 1, 0, 1, 1, 0
    accepted = rejected = 0
    for offset in range(1, len(lines) - 1, 2):
        assert lines[offset].split('|')[0] == 'STATE'
        state = [int(v) for v in lines[offset].split('|')[1:]]
        assert state == [q, r, t, k, printed] and printed < n
        assert q > 0 and t > 0 and k >= 1
        lower, upper = 3 * q + r, 4 * q + r
        assert lower >= 0 and upper == lower + q
        fields = lines[offset + 1].split('|')
        assert fields[0] == 'DIV'
        a, d3, b, d4 = [int(v) for v in fields[1:]]
        assert [a, d3, b, d4] == [lower, lower // t, upper, upper // t]
        if d3 == d4:
            assert 0 <= d3 <= 9
            r, q = 10 * (r - d3 * t), 10 * q
            printed += 1
            accepted += 1
        else:
            factor = 2 * k + 1
            r, q, t, k = (2 * q + r) * factor, q * k, t * factor, k + 1
            rejected += 1
        assert 3 * q + r >= 0
    assert accepted == n and printed == n
    return dict(status='passed', actual_candidate_state=True, both_actual_integer_divisions=True,
        all_candidates_including_rejections=True, nonnegative_extraction_numerators=True,
        accepted_candidates=accepted, rejected_candidates=rejected,
        trace_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        preparation_only=True, never_chart_eligible=True)
