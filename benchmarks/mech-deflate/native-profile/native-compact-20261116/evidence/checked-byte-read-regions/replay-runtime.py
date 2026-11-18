"""Replay the archived checked-read peer comparisons from their exact observations."""
import gzip
import json
from pathlib import Path
import sys

here = Path(__file__).resolve().parent
packet = here.parents[1]
sys.path.insert(0, str(packet))
from qualification_records import ACTORS, Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run


def replay(kind):
    part = here / kind
    plan = read_json(part / 'plan.json')
    policy = read_json(packet / 'qualification-policy-v2.json')
    assert plan['policy']['sha256'] == digest(packet / 'qualification-policy-v2.json')
    with gzip.open(part / 'replay.json.gz', 'rt') as raw:
        original = json.load(raw)
    findings = Findings()
    evidence = Evidence(packet, findings)
    fixtures = load_fixtures(evidence, policy)
    assert plan['policy_version'] == policy['version'] == 2
    assert digest(part / 'contract.json') == plan['identities']['contract_sha256']
    assert digest(part / 'environment.json') == plan['identities']['environment_sha256']

    def ref(path, prior=None):
        result = {'path': str(path.relative_to(packet)), 'sha256': digest(path)}
        if prior is not None:
            assert result['sha256'] == prior['sha256'], path
        return result

    runs = []
    for definition in plan['runs']:
        directory = part / definition['order']
        existing = original['receipts'][definition['id']]
        pins = ref(directory / 'canonical-pins.json', existing['pins'])
        assert digest(directory / 'pins.json') == existing['original_pins']['sha256']
        complete = read_json(directory / 'complete.json')
        assert complete['balanced_blocks_per_case'] == definition['blocks_per_case'] == 96
        receipt = definition | {'pins': pins,
            'summary': ref(directory / 'summary.json', existing['summary']),
            'complete': ref(directory / 'complete.json', existing['complete']),
            'cleanup': {actor: ref(directory / f'{actor}.stderr', existing['cleanup'][actor]) for actor in ACTORS},
            'observations': {case: ref(directory / case / 'observations.json.gz', expected)
                             for case, expected in existing['observations'].items()}}
        result = runtime_run(receipt, fixtures, evidence, plan['identities'], policy, definition)
        assert result is not None
        runs.append(result)
    replayed = decide_runtime(runs, [], policy, fixtures, findings)
    assert findings.decision == original['decision']
    assert findings.rejected == original['rejected']
    assert findings.unresolved == original['unresolved']
    assert replayed['runs'] == original['runs']
    print(kind, findings.decision, findings.unresolved)


if __name__ == '__main__':
    raise SystemExit("Superseded: both historical workers linked the reference object. See README.md.")
