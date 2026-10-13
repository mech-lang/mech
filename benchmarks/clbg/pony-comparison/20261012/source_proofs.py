"""Draft bounded public-library and actor handoff contracts.

Actual production-source work and runtime GC/scheduler qualification remain
separate required gates. This module cannot approve a full study.
"""


def run(preparation, guard, harness, proposal, snapshot, build, compiler,
        regex, gmp_wrapper, sdk, environment, runtime_environment, pins, lock_fd, report):
    source = snapshot / 'support/standard_primitives/main.pony'
    destination = build / 'bin/standard-primitives'
    destination.mkdir()
    binary = destination / 'pony-standard-primitives'
    output = guard.OUT / 'source-proofs'
    output.mkdir()
    command = preparation.build_command(compiler, sdk, regex, gmp_wrapper, source,
                                        destination, binary.name)
    preparation.owned_command(guard, harness, command, environment, output / 'build',
                              lock_fd, 600, 'standard-primitives')
    pins.update(preparation.merge(pins, preparation.pin_tree(destination),
        preparation.native_images(guard, harness, binary, environment, output / 'images',
                                  lock_fd, [build / 'release', preparation.GMP, preparation.PCRE])))
    guard.verify(pins)
    row = dict(id='pony-standard-primitives', key='pidigits:pony-standard-primitives',
               benchmark='pidigits', language='pony', source=str(source),
               source_sha256=preparation.sha(source), binary_sha256=preparation.sha(binary),
               command=[str(binary), '--ponymaxthreads', '8'], cwd=str(preparation.HERE),
               runtime_environment=dict(runtime_environment), primitive_or_ownership_contract=True,
               preparation_only=True, never_chart_eligible=True)
    attempt = preparation.probe(guard, harness, row, 0, None, b'PONY_PRIMITIVES_OK\x00\n',
                                pins, output, lock_fd, report, 'ordinary-public-apis')
    return dict(status='passed', result=attempt['supervisor_result_path'],
                complete_seven_actor_map_handoffs=True, parent_value_reads_and_release=True,
                signed_floor_division=True, large_LP64_integer_argument=True,
                valid_decimal_constructor=True, immutable_operands_under_bounded_churn=True,
                regex_nonoverlap_and_ordered_replacement=True, binary_nul_stdout=True,
                observed_compute_thread_identity_cap=8, explicit_runtime_scheduler_cap=8,
                invalid_decimal_rejection_claimed=False, finalizer_counts_proven=False,
                production_source_work_traces_proven=False, full_performance_runs=0,
                preparation_only=True, never_chart_eligible=True)
