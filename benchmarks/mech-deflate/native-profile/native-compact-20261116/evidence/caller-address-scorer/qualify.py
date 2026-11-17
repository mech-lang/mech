"""Link the selected scorer worker and check exact failures and stack high-water."""
from emit import HERE, TEMPLATE, link, read, save, sha
import sys


def main():
    label, = sys.argv[1:]
    root = HERE / label
    screen = read(root / 'focus/screen.json')
    assert screen['preserves_peer_wins'] and not screen['rejected']
    assert read(root / 'receipt.json')['text_change'] < 0
    folder = root / 'checks'
    checks = folder / 'worker-checks'
    participants = {'control': TEMPLATE, label: root / 'bound'}
    bindings = {}
    for name, original in participants.items():
        worker = checks / name
        binding = link.link_worker(TEMPLATE, original, worker)
        assert binding['worker_sha256'] == sha(original / 'mech-worker')
        link.check_worker(worker, f'scorer-qualification-{label}-{name}')
        bindings[name] = binding
    save(folder / 'bindings.json', bindings)
    result = link.check_contracts(folder, tuple(participants))
    if result:
        print((folder / 'verify-linked.log').read_text()[-6000:], flush=True)
        raise SystemExit(result)
    proof = read(folder / 'linked-proofs/result.json')
    assert proof['matches_success_observations'] == 162 and len(proof['checked_failure_cases']) == 6
    assert proof['peak_stack_bytes']['control'] == 5280
    assert proof['peak_stack_bytes'][label] <= 5280
    print(proof['peak_stack_bytes'], flush=True)


if __name__ == '__main__':
    main()
