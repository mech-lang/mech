"""Verify all frozen raw streams for the three deployment CLIs and both links."""
from argparse import ArgumentParser
from hashlib import file_digest, sha256
import json
from pathlib import Path
import subprocess
import zlib

INVENTORY_SHA256 = 'b6de359f4e8e5fa47c9ebed9474954f6e7b809e08d4f3b36193412492c4d9490'
C_ORACLE_SHA256 = 'ab77258e776c24641f0a6b09c3f485d54f9e951e4bb2ad29d70a5a61e7cc7c2c'
CODECS = ('mech', 'roc', 'libdeflate')
LINKS = ('unstripped', 'stripped')


def digest(path):
    with path.open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def pinned(path, reference):
    data = path.read_bytes()
    assert sha256(data).hexdigest() == reference, path
    return data


def raw_inflates_to(output, expected):
    try:
        decoder = zlib.decompressobj(-zlib.MAX_WBITS)
        data = decoder.decompress(output, len(expected) + 1)
    except zlib.error:
        return False
    return data == expected and decoder.eof and not decoder.unused_data and not decoder.unconsumed_tail


def verify(executable, input_file, input_bytes, reference_size, reference_digest, reference_bytes):
    command = [str(executable), str(input_file)]
    try:
        run = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30, check=False)
    except subprocess.TimeoutExpired:
        return {'command': command, 'validated': False, 'failure': 'timeout'}
    output_sha256 = sha256(run.stdout).hexdigest()
    exact = len(run.stdout) == reference_size and output_sha256 == reference_digest
    if reference_bytes is not None:
        exact = exact and run.stdout == reference_bytes
    independent = raw_inflates_to(run.stdout, input_bytes)
    return {
        'command': command,
        'exit_code': run.returncode,
        'raw_bytes': len(run.stdout),
        'raw_sha256': output_sha256,
        'raw_exact': exact,
        'independent_full_stream_inflate': independent,
        'validated': run.returncode == 0 and exact and independent,
        **({'stderr': run.stderr.decode(errors='replace')} if run.stderr else {}),
    }


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path, help='New output receipt path')
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--libdeflate-oracle', type=Path, required=True)
    for codec in CODECS:
        for link in LINKS:
            parser.add_argument(f'--{codec}-{link}', type=Path, required=True)
    args = parser.parse_args()
    assert not args.report.exists(), args.report
    fixtures = json.loads(pinned(args.inventory, INVENTORY_SHA256))
    c_outputs = json.loads(pinned(args.libdeflate_oracle, C_ORACLE_SHA256))
    c_oracle = {row['case']: row for row in c_outputs}
    assert len(fixtures) == len({row['name'] for row in fixtures}) == len(c_outputs) == len(c_oracle) == 81
    assert {row['name'] for row in fixtures} == c_oracle.keys()

    executables = {
        codec: {link: getattr(args, f'{codec}_{link}').resolve() for link in LINKS}
        for codec in CODECS
    }
    hashes = {
        codec: {link: digest(executables[codec][link]) for link in LINKS}
        for codec in CODECS
    }
    observations = []
    summary = {codec: dict.fromkeys(LINKS, 0) for codec in CODECS}
    fixture_root = args.inventory.resolve().parent
    for case in fixtures:
        name = case['name']
        input_file = (fixture_root / case['input']).resolve()
        input_bytes = pinned(input_file, case['input_sha256'])
        expected = pinned(fixture_root / case['expected'], case['expected_sha256'])
        prefix_path = fixture_root / case['prefix']
        prefix = pinned(prefix_path, prefix_path.name)
        assert len(input_bytes) == case['input_bytes'] and len(prefix) == case['prefix_bytes'], name
        assert expected.startswith(prefix), name
        raw = expected[len(prefix):]
        assert len(raw) == case['raw_bytes'], name
        raw_sha = sha256(raw).hexdigest()
        c_expected = c_oracle[name]
        outputs = {}
        for codec in CODECS:
            if codec == 'libdeflate':
                expected_size = c_expected['raw_bytes']
                expected_sha = c_expected['output_sha256']
                expected_data = None
            else:
                expected_size = len(raw)
                expected_sha = raw_sha
                expected_data = raw
            outputs[codec] = {}
            for link in LINKS:
                result = verify(executables[codec][link], input_file, input_bytes, expected_size,
                                expected_sha, expected_data)
                outputs[codec][link] = result
                summary[codec][link] += int(result['validated'])
        assert digest(input_file) == case['input_sha256'], (name, 'input file changed')
        observations.append({'case': name, 'input_sha256': case['input_sha256'],
                             'mech_roc_raw_sha256': raw_sha,
                             'libdeflate_raw_sha256': c_expected['output_sha256'], 'results': outputs})
    assert all(digest(path) == hashes[codec][link]
               for codec, rows in executables.items() for link, path in rows.items())
    verdict = all(count == 81 for rows in summary.values() for count in rows.values())
    report = {
        'scope': 'Separate untimed one-shot raw stream deployment checks, level 12, no prefix',
        'fixture_inventory_sha256': INVENTORY_SHA256,
        'libdeflate_raw_oracle_sha256': C_ORACLE_SHA256,
        'binary_sha256': hashes,
        'decoder': {'name': 'Python zlib raw DEFLATE', 'runtime_version': zlib.ZLIB_RUNTIME_VERSION},
        'all_exact_and_fully_consumed': verdict,
        'successful_named_cases': summary,
        'observations': observations,
    }
    with args.report.open('x') as file:
        json.dump(report, file, indent=2)
        file.write('\n')
    print(json.dumps(summary, indent=2))
    raise SystemExit(0 if verdict else 1)


if __name__ == '__main__':
    main()
