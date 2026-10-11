"""Prepare additive copies with the exact canonical Jupiter z coordinate."""
import copy
import hashlib
import json
import struct
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
G = HERE.parent
CANONICAL = '0.103622044471123109'
OLD = {'apl': '0.103622044471123139', 'uiua': '0.10362204447112314'}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def main():
    for language in OLD:
        parent_path = G / (language + '-manifest.json')
        parent = json.loads(parent_path.read_text())
        rows = [r for r in parent['programs'] if r['benchmark'] == 'nbody' and not r.get('excluded')]
        assert len(rows) == 2
        programs = []
        copies = {}
        for original in rows:
            source = Path(original['source'])
            assert sha(source) == original['source_sha256']
            if source not in copies:
                text = source.read_text()
                assert text.count(OLD[language]) == 1
                fixed = text.replace(OLD[language], CANONICAL)
                assert fixed.replace(CANONICAL, OLD[language]) == text
                target = HERE / 'sources' / language / source.name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(fixed)
                copies[source] = target
            target = copies[source]
            row = copy.deepcopy(original)
            row['id'] += '-canonical-v2'
            row['key'] = 'nbody:' + row['id']
            row['source'] = str(target)
            row['source_sha256'] = sha(target)
            row['command'] = [s.replace(str(source), str(target)) for s in row['command']]
            if row.get('timing_alias_of'):
                declared = row['timing_alias_of'].split(':')[-1]
                row['timing_alias_of'] = 'nbody:' + declared + '-canonical-v2'
            row['supersedes_keys'] = [original['key']]
            row['supersedes_source_sha256'] = [original['source_sha256']]
            row['canonical_constant_correction'] = {
                'original_source': str(source), 'original_source_sha256': sha(source),
                'old_literal': '¯' + OLD[language], 'new_literal': '¯' + CANONICAL,
                'old_binary64_hex': struct.pack('>d', -float(OLD[language])).hex(),
                'canonical_binary64_hex': struct.pack('>d', -float(CANONICAL)).hex(),
                'only_literal_changed': True,
                'original_full_attempts': 0,
                'reason': 'Two-ULP transcription correction to published canonical initial data; '
                          'same application algorithm, work shape, runtime and LOC. '
                          'Original small validated sources remain retained; no full failure is claimed.',
            }
            row['adaptations'] = list(row.get('adaptations', [])) + [
                'Use exact canonical Jupiter z initial coordinate -0.103622044471123109.']
            row.update(measurement_status='not-measured', eligible_for_chart=False,
                       samples_seconds=[], sample_validation=[], attempts=[])
            row.pop('small_validation', None)
            programs.append(row)
        manifest = dict(status='prepared-pending-locked-validation', language=language,
                        created_at=datetime.now(timezone.utc).isoformat(), programs=programs,
                        runtime=parent['runtime'],
                        parent_manifest=str(parent_path), parent_manifest_sha256=sha(parent_path),
                        scope='Additive canonical nbody initial-data correction only; no full timings.',
                        preparation_command=['python3', str(Path(__file__).resolve())])
        save(HERE / (language + '-manifest.json'), manifest)
        print('PREPARED', language, len(programs), len(copies), 'distinct sources', flush=True)


if __name__ == '__main__':
    main()
