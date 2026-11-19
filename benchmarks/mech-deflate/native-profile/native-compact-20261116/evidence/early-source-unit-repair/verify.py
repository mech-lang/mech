"""Recalculate the archived source-unit and complete-build evidence."""
from pathlib import Path
from hashlib import sha256
from statistics import mean, median
import gzip
import json
import math
import struct
import subprocess

HERE = Path(__file__).resolve().parent


def data(path):
    if not path.exists():
        path = path.with_name(path.name + '.gz')
    raw = path.read_bytes()
    return gzip.decompress(raw) if path.suffix == '.gz' else raw


def read(path):
    return json.loads(data(path))


def digest(raw):
    return sha256(raw).hexdigest()


def close(a, b):
    assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12), (a, b)


def text_size(raw):
    offset = 32
    for _ in range(struct.unpack_from('<I', raw, 16)[0]):
        kind, size = struct.unpack_from('<II', raw, offset)
        if kind == 0x19:
            for i in range(struct.unpack_from('<I', raw, offset + 64)[0]):
                section = offset + 72 + 80 * i
                name, segment = struct.unpack_from('<16s16s', raw, section)
                if name.rstrip(b'\0') == b'__text' and segment.rstrip(b'\0') == b'__TEXT':
                    return struct.unpack_from('<Q', raw, section + 40)[0]
        offset += size
    raise AssertionError('No Mach-O text section')


def main():
    manifest = read(HERE / 'manifest.json')
    for name, pin in manifest.items():
        raw = (HERE / name).read_bytes()
        assert len(raw) == pin['bytes'] and digest(raw) == pin['sha256'], name
    study = read(HERE / 'study.json')
    repository = HERE.parents[5]
    for name, pin in study['source_inputs'].items():
        raw = data(HERE / 'committed-source' / name)
        assert digest(raw) == pin, name
        committed = subprocess.check_output(['git', 'show', study['source_commit'] + ':' + name],
                                            cwd=repository)
        assert committed == raw, name
    for role, row in study['cases'].items():
        for name in ['program.o', 'metadata.json']:
            raw = data(HERE / 'cases' / role / 'frontend' / (name + '.gz'))
            assert digest(raw) == row['artifacts'][name], (role, name)
            if name == 'program.o':
                assert text_size(raw) == row['encoder_text_bytes']
    replays = read(HERE / 'source-replay/result.json')
    assert len(replays) == 6
    for replay in replays:
        role = 'early-owner-output-root-preserved' if replay['label'] == 'experimental-unit' else 'selected-map'
        assert replay['artifacts'] == study['cases'][role]['artifacts']
    build = read(HERE / 'matched-builds/comparison.json')
    plan = read(HERE / 'matched-builds/plan.json')
    summary = read(HERE / 'matched-builds/summary.json')
    assert digest(data(HERE / 'matched-builds/plan.json')) == build['plan_sha256']
    assert digest(data(HERE / 'matched-builds/timing-driver.py')) == plan['timer_sha256']
    rows = build['observations']
    assert len(rows) == summary['trials'] == 18 and len(plan['actors']) == 9
    for actor in plan['actors']:
        for key, expected in summary['medians'][actor].items():
            close(median(row[actor]['seconds'][key] for row in rows), expected)
        for reference, expected in summary['paired_ratios'].get(actor, {}).items():
            close(median(row[actor]['seconds']['source_through_link'] /
                         row[reference]['seconds']['source_through_link'] for row in rows), expected)
        for row in rows:
            assert row[actor]['generated_artifacts'] == plan['actors'][actor]['artifact_sha256']
    for pair, expected in summary['paired_differences'].items():
        actor, reference = pair.split(' vs ')
        close(median(row[actor]['seconds']['source_through_link'] -
                     row[reference]['seconds']['source_through_link'] for row in rows),
              expected['paired_median_seconds'])
    for order in ['forward', 'reverse']:
        folder = HERE / 'focus-observations' / order
        rows = read(folder / 'summary.json')
        assert len(rows) == 17
        for row in rows:
            blocks = read(folder / row['case'] / 'observations.json.gz')
            assert len(blocks) == 96
            for actor in row['paired_x_libdeflate']:
                for block in blocks:
                    close(mean(x['seconds'] for x in block['observations'][actor]), block['means'][actor])
                    for observation in block['observations'][actor]:
                        for key in ['exact_reference', 'independent_inflation',
                                    'input_alias_unchanged', 'borrowed_output_alias_unchanged']:
                            if key in observation:
                                assert observation[key], (order, row['case'], actor, key)
                close(math.exp(mean(math.log(b['means'][actor] / b['means']['libdeflate']) for b in blocks)),
                      row['paired_x_libdeflate'][actor])
        overlap = next(row for row in rows if row['case'] == '320-overlap')
        assert overlap['paired_x_libdeflate']['mech-control'] < 1 < overlap['paired_x_libdeflate']['mech-experiment']
    assert not study['architecture_selected']
    print('Archive, source, emission, replay, matched builds and both focused orders verified.')


if __name__ == '__main__':
    main()
