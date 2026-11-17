"""Run the linked resource and checked-error probes for the address ABI."""

from study import HERE, driver, read, save, sha

ROLE = 'ord-abi'


def main():
    trial = HERE / ROLE
    bindings = {}
    output = trial / 'checks'
    for role, source in (('control', driver.TEMPLATE), (ROLE, trial / 'bound')):
        worker = output / 'worker-checks' / role
        receipt = driver.link.link_worker(driver.TEMPLATE, source, worker)
        assert receipt['worker_sha256'] == sha(source / 'mech-worker')
        driver.link.check_worker(worker, f'ordered-abi-stack-{role}')
        bindings[role] = receipt
    save(output / 'bindings.json', bindings)
    assert driver.link.check_contracts(output, tuple(bindings)) == 0
    reports = output / 'linked-proofs'
    result = read(reports / 'result.json')
    assert result['matches_success_observations'] == 162 and len(result['checked_failure_cases']) == 6
    assert result['peak_stack_bytes']['control'] == 5280
    print(result['peak_stack_bytes'], flush=True)


if __name__ == '__main__':
    main()
