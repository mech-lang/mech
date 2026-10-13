"""Draft unscored generic ABI, ORC ownership and actual seven-map proofs.

The caller owns the common mutex. These are isolated audit copies and primitive
contracts; no production source is edited and no full process is measured.
"""
from collections import Counter
import hashlib
from pathlib import Path

import references

BIGINT_ABI = '''import std/options
import bigints
let a = initBigInt(17)
let b = initBigInt(5)
doAssert a div b == initBigInt(3)
doAssert a mod b == initBigInt(2)
# BigInts 1.1.0 defines floor division and divisor-signed modulo.
doAssert initBigInt(-17) div b == initBigInt(-4)
doAssert initBigInt(-17) mod b == initBigInt(3)
doAssert a div initBigInt(-5) == initBigInt(-4)
doAssert a mod initBigInt(-5) == initBigInt(-3)
doAssert initBigInt(-17) div initBigInt(-5) == initBigInt(3)
doAssert initBigInt(-17) mod initBigInt(-5) == initBigInt(-2)
doAssert (a * b) - a == initBigInt(68)
doAssert toInt[int](initBigInt(9)).get() == 9
echo "pure-bigint-contract-ok"
'''

GMP_ABI = '''import "support/gmp_ops"
doAssert sizeof(Mpz) == 16
doAssert alignof(Mpz) == 8
var a: Mpz
var b: Mpz
var q: Mpz
var zero: Mpz
var expected: Mpz
init(a)
init(b)
init(q)
init(zero)
init(expected)
setUi(a, 17)
setUi(b, 5)
divide(q, a, b)
doAssert getUi(q) == 3
divide(a, a, b)
doAssert getUi(a) == 3
setUi(a, 17)
sub(a, zero, a)
divide(q, a, b)
setUi(expected, 3)
sub(expected, zero, expected)
doAssert compare(q, expected) == 0
setUi(a, 7)
mulUi(a, a, 3)
doAssert getUi(a) == 21
add(a, a, a)
doAssert getUi(a) == 42
addmulUi(a, a, 2)
doAssert getUi(a) == 126
clear(a)
clear(b)
clear(q)
clear(zero)
clear(expected)
echo "gmp-lp64-alias-sign-contract-ok"
'''

PCRE_ABI = '''import std/[strutils, re]
import wrappers/pcre as api
doAssert ($api.version()).startsWith("8.45 ")
doAssert sizeof(api.ExtraData) == 64
doAssert alignof(api.ExtraData) == 8
doAssert findAll("aaaaa", re("aa")).len == 2
let first = replace("taNDHaD", re("aND|Ha[DS]"), "<3>")
doAssert replace(first, re("<[^>]*>"), "|") == "t||"
doAssert findAll("AGGGTAAA", re("agggtaaa")).len == 0
echo "pcre-8.45-dynamic-layout-regex-contract-ok"
'''

ORC_OWNERSHIP = '''import std/[tables, typedthreads, sets, atomics, os]
type Job = tuple[count: int, slot: int, output: ptr Table[uint64, string], tid: ptr int, ready: ptr Atomic[int], go: ptr Atomic[bool]]
proc worker(job: Job) {.thread.} =
  job.output[] = initTable[uint64, string]()
  job.tid[] = getThreadId()
  discard job.ready[].fetchAdd(1, moRelease)
  while not job.go[].load(moAcquire):
    sleep(1)
  for i in 0 ..< job.count:
    let key = uint64(i * 17 + job.slot)
    job.output[][key] = $key & ":" & $job.slot
for count in [0, 1, 2, 17, 65, 1025, 4096]:
  var tables: array[7, Table[uint64, string]]
  var tids: array[7, int]
  var threads: array[7, Thread[Job]]
  var ready: Atomic[int]
  var go: Atomic[bool]
  for slot in 0 ..< 7:
    createThread(threads[slot], worker, (count, slot, addr tables[slot], addr tids[slot], addr ready, addr go))
  while ready.load(moAcquire) != 7:
    sleep(1)
  go.store(true, moRelease)
  for slot in 0 ..< 7:
    joinThread(threads[slot])
  var threadIds = initHashSet[int]()
  for slot in 0 ..< 7:
    threadIds.incl(tids[slot])
    doAssert tables[slot].len == count
    for i in 0 ..< count:
      let key = uint64(i * 17 + slot)
      doAssert tables[slot][key] == $key & ":" & $slot
  doAssert threadIds.len == 7
  reset(tables)
  for slot in 0 ..< 7:
    doAssert tables[slot].len == 0
echo "orc-seven-worker-join-parent-destruction-ok"
'''


def map_copy(source, variant):
    marker = 'for index in 0 ..< 2:\n'
    assert source.count(marker) == 1, 'Actual source output anchor must be unique'
    assert 'MECH_NIM_WORK_TRACE' not in source
    addition = '''let audit = open(getEnv("MECH_NIM_WORK_TRACE"), fmWrite)
audit.writeLine("TRACE_NIM1|knucleotide|VARIANT|7|", sequence.len)
for index, width in lengths:
  var total = 0'i64
  for key, value in counts[index]:
    audit.writeLine("MAP|", width, "|", key, "|", value)
    total += value
  audit.writeLine("TABLE|", width, "|", counts[index].len, "|", total)
audit.writeLine("END_NIM1")
audit.close()
'''.replace('VARIANT', variant)
    # Imports/audit/reset are only in this unscored copy. All production
    # arithmetic, map updates, joins and stdout statements remain in order.
    return 'import std/os\n' + source.replace(marker, addition + marker) + '\nreset(counts)\n'


def reconcile(path, data, variant):
    sequence, expected = references.nucleotide_maps(data)
    lines = path.read_text().splitlines()
    assert lines[0] == f'TRACE_NIM1|knucleotide|{variant}|7|{len(sequence)}'
    assert lines[-1] == 'END_NIM1' and lines.count('END_NIM1') == 1
    observed = {width: {} for width in references.WIDTHS}
    position = 0
    summaries = []
    for line in lines[1:-1]:
        fields = line.split('|')
        assert position < 7 and len(fields) == 4
        width = references.WIDTHS[position]
        assert int(fields[1]) == width
        if fields[0] == 'MAP':
            if variant == 'compact':
                word = fields[2]
            else:
                key = int(fields[2])
                assert str(key) == fields[2] and 0 <= key < (1 << (2 * width))
                word = ''.join('ACGT'[(key >> (2 * shift)) & 3] for shift in range(width - 1, -1, -1))
            assert len(word) == width and set(word) <= set('ACGT')
            assert word not in observed[width]
            count = int(fields[3])
            assert count > 0 and str(count) == fields[3]
            observed[width][word] = count
        else:
            assert fields[0] == 'TABLE'
            assert int(fields[2]) == len(observed[width])
            assert int(fields[3]) == sum(observed[width].values()) == max(0, len(sequence) - width + 1)
            assert observed[width] == expected[position]
            summaries.append(dict(width=width, keys=len(observed[width]), windows=sum(observed[width].values())))
            position += 1
    assert position == 7
    return dict(status='passed', actual_source_maps=True, all_seven_maps=True,
                first_two_maps_included=True, all_overlapping_windows=True,
                sequence_length=len(sequence), tables=summaries,
                trace_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                post_join_parent_reset_in_audit_copy=True,
                preparation_only=True, never_chart_eligible=True)


def run(guard, harness, proposal, snapshot, build, compiler, toolchain,
        bigint, gmp_wrapper, sdk, environment, pins, lock_fd, report):
    import guarded_prepare_v7 as preparation
    output = guard.OUT / 'source-proofs'
    output.mkdir()
    import json
    primitive = json.loads(Path(proposal['prior_primitive_reuse']).read_text())
    summaries = [dict(p) for p in primitive['proofs']]

    def build_proof(name, text, asan=False):
        source = snapshot / (name.replace('-', '_') + '.nim')
        with source.open('x') as stream:
            stream.write(text)
        binary = build / 'bin' / name
        cache = build / 'cache' / name
        pins[str(source)] = preparation.sha(source)
        command = preparation.build_command(compiler, toolchain, bigint, gmp_wrapper, sdk, source, binary, cache)
        if asan:
            command[-1:-1] = ['--passC:-fsanitize=address', '--passL:-fsanitize=address', '--passC:-fno-omit-frame-pointer']
        preparation.owned_command(guard, harness, command, environment,
                                  output / name, lock_fd, 180, 'build')
        pins.update(preparation.merge(pins, preparation.pin_tree(cache), {str(binary): preparation.sha(binary)}))
        pins.update(preparation.merge(pins, preparation.native_images(guard, harness, binary, environment,
            output / name / 'images', lock_fd,
            [toolchain, preparation.GMP, preparation.PCRE, preparation.CLANG.parent.parent])))
        guard.verify(pins)
        return dict(id=name, key='pidigits:' + name, benchmark='pidigits', language='nim',
                    source=str(source), source_sha256=preparation.sha(source), binary_sha256=preparation.sha(binary),
                    command=[str(binary), '{n}'], cwd=str(preparation.HERE), runtime_environment={
                        'LC_ALL': 'C', 'DYLD_LIBRARY_PATH': preparation.LIBRARY_PATH},
                    primitive_or_ownership_contract=True, preparation_only=True, never_chart_eligible=True)

    ownership = json.loads(Path(proposal['prior_ownership_reuse']).read_text())
    guard.verify(ownership['pins'])
    assert ownership['status'] == 'verified-one-exact-passed-v6-ORC-contract'
    summaries.extend(dict(proof) for proof in ownership['proofs'])

    cases = (('empty', b'>THREE'),
             ('tail-comments-crlf', b'>ONE\r\nACGT\r\n>THREEBAD\r\nAAAA\r\n;>THREE\r\n>THREE\tselected\r\nggtattttaatttatagta\r\n;ignored\r\ncgt'),
             ('growing', b'>THREE\n' + b'ACGTGGTATT' * 4096))
    for variant in ('compact', 'optimized'):
        original = snapshot / 'sources/knucleotide' / (variant + '.nim')
        row = build_proof('actual-seven-map-' + variant, map_copy(original.read_text(), variant), True)
        row['benchmark'] = 'knucleotide'
        row['key'] = 'knucleotide:' + row['id']
        for case, data in cases:
            trace_path = output / row['id'] / case / 'actual-maps.trace'
            assert not trace_path.exists()
            row['runtime_environment']['MECH_NIM_WORK_TRACE'] = str(trace_path)
            attempt = preparation.probe(guard, harness, row, harness.WORK['knucleotide'][0], data,
                    references.knucleotide(data), pins, output, lock_fd, report, case,
                    trace=lambda directory: reconcile(trace_path, data, variant))
            pins[str(trace_path)] = preparation.sha(trace_path)
            summaries.append(dict(proof='actual-seven-map-' + variant, case=case, status='passed',
                                  result=attempt['supervisor_result_path'],
                                  reconciliation=attempt['source_work_proof'], asan=True))
    guard.verify(pins)
    import pi_work_v5
    for variant in ('compact', 'optimized'):
        original = snapshot / 'sources/pidigits' / (variant + '.nim')
        row = build_proof('actual-pi-' + variant, pi_work_v5.copy(original.read_text(), variant))
        for n in (0, 1, 9, 10, 11, 27, 97):
            case = 'n-' + str(n)
            trace_path = output / row['id'] / case / 'actual-divisions.trace'
            assert not trace_path.exists()
            row['runtime_environment']['MECH_NIM_WORK_TRACE'] = str(trace_path)
            attempt = preparation.probe(guard, harness, row, n, None, references.pidigits(n), pins,
                output, lock_fd, report, case, trace=lambda directory, n=n, variant=variant, path=trace_path:
                pi_work_v5.reconcile(path, n, variant))
            pins[str(trace_path)] = preparation.sha(trace_path)
            summaries.append(dict(proof='actual-pi-' + variant, case=case, status='passed',
                result=attempt['supervisor_result_path'], reconciliation=attempt['source_work_proof']))
    guard.verify(pins)
    return summaries
