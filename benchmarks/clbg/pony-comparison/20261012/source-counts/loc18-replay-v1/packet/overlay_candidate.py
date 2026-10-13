"""Unexecuted exact-source overlay checker; raw compiler inventory stays separate."""

EXPECTED = {(benchmark, variant) for benchmark in (
    'binarytrees', 'fannkuchredux', 'nbody', 'spectralnorm', 'mandelbrot',
    'pidigits', 'fasta', 'revcomp', 'knucleotide', 'regexredux')
    for variant in ('compact', 'optimized')}


def actual_count(value):
    assert type(value) is int and value >= 0, 'Canonical count must be an actual nonnegative integer'
    return value


def reviewed_count(metric, review, source, digest, actual_source_characters):
    assert review['status'] == 'passed' and review['review_identity']
    assert review['source'] == source and review['source_sha256'] == digest
    assert review['physical_loc'] == actual_count(metric['physical_loc'])
    assert review['normalized_loc'] == actual_count(metric['normalized_loc'])
    assert review['complete_selected_source_read'] is True
    assert review['all_written_owners_retained'] is True
    assert review['literal_data_and_callback_ownership_reviewed'] is True
    assert review['actual_owner_events'] == metric['canonical_events']
    assert review['actual_expansions'] == metric['expansions']
    source_length = actual_count(review['source_character_length'])
    assert source_length == actual_count(actual_source_characters)
    diagnostics = metric['diagnostics']
    dispositions = review['diagnostic_dispositions']
    assert len(dispositions) == len(diagnostics)
    for diagnostic, disposition in zip(diagnostics, dispositions):
        assert disposition['actual_diagnostic'] == diagnostic
        assert disposition['status'] == 'accepted-exact-existing-owner-events'
        span = disposition['actual_source_span']
        assert type(span['start']) is type(span['end']) is int and 0 <= span['start'] < span['end'] <= source_length
        if 'offset' in diagnostic: assert span['start'] <= diagnostic['offset'] < span['end']
        assert disposition['owner_event_evidence']
        assert all(event in metric['canonical_events'] for event in disposition['owner_event_evidence'])
        for event in disposition['owner_event_evidence']:
            assert type(event['start']) is type(event['end']) is int
            assert 0 <= event['start'] < event['end'] <= source_length
            assert event['start'] < span['end'] and span['start'] < event['end'], 'Cited owner does not intersect the reviewed diagnostic span'
        if 'offset' in diagnostic:
            assert any(event['start'] <= diagnostic['offset'] < event['end']
                       for event in disposition['owner_event_evidence']), 'No cited written owner covers the actual diagnostic offset'
        assert disposition['review_identity'] and disposition['count_override'] is False
    assert review['count_override'] is False and not review.get('unresolved')
    # Every diagnostic is preserved in the actual replay. A semantic review may
    # certify the existing exact owners, but cannot invent/rewrite a number here.
    return dict(status='passed', review_identity=review['review_identity'],
                evidence_path=review['evidence_path'], evidence_sha256=review['evidence_sha256'])


def construct(raw_inventory, replay, review, inventory_evidence, review_evidence):
    """Return qualified input evidence + overlay only after complete validation.

    The caller must have checked actual source/result/review/grammar pins under
    the held mutex. Root's selected-source semantic review is a real external
    prerequisite; this function never supplies or guesses that review itself.
    """
    assert raw_inventory['status'] == 'compiler-input-proposals-awaiting-role-review'
    assert replay['status'] == 'actual-source-count-proposals-awaiting-selected-source-review'
    assert review['status'] == 'passed-exact-compiler-role-and-selected-source-review'
    assert review['inventory'] == inventory_evidence and review['replay'] == replay['self_evidence']
    rows = raw_inventory['profiles']
    assert len(rows) == len({row['id'] for row in rows}) == 20
    assert {(row['benchmark'], row['variant']) for row in rows} == EXPECTED
    assert set(review['profiles']) == set(replay['profiles']) == {row['id'] for row in rows}
    qualified_inputs, output_profiles = {}, {}
    for row in rows:
        identifier = row['id']; selected = review['profiles'][identifier]
        assert not row['unresolved']
        assert selected['status'] == 'passed' and selected['complete_compiler_packages_reviewed'] is True
        assert selected['source_sha256'] == row['source_sha256']
        assert selected['binary_sha256'] == row['binary_sha256']
        assert selected['actual_original_compile_command'] == row['actual_original_compile_command']
        assert selected['actual_package_inventory'] == row['packages']
        assert selected['actual_authored_files'] == [{key: item[key] for key in ('path', 'sha256')} for item in row['files']]
        counted = replay['profiles'][identifier]
        assert counted['id'] == identifier and counted['source_sha256'] == row['source_sha256']
        assert counted['binary_sha256'] == row['binary_sha256']
        assert [(item['path'], item['sha256']) for item in counted['source_bundle']] == [(item['path'], item['sha256']) for item in row['files']]
        bundle = []
        for item in counted['source_bundle']:
            original = next(value for value in row['files'] if value['path'] == item['path'])
            assert item['role'] == original['role'] and original['source_language'] == 'pony'
            metric = replay['source_counts'][item['path']]['metric']
            specific = dict(review['sources'][item['path']], **review_evidence)
            ownership = reviewed_count(metric, specific, item['path'], item['sha256'],
                                       replay['source_counts'][item['path']]['source_character_length'])
            bundle.append(dict(path=item['path'], sha256=item['sha256'], role=item['role'],
                physical_loc=actual_count(metric['physical_loc']), normalized_loc=actual_count(metric['normalized_loc']),
                count_receipt=dict(replay['self_evidence'], source_row_identity=item['path']),
                ownership_review=ownership, unresolved_diagnostics=[]))
        semantic = dict(status='passed', review_identity=selected['review_identity'], **review_evidence)
        qualified_inputs[identifier] = dict(status='qualified-exact-authored-compiler-inputs',
            source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
            evidence_path=inventory_evidence['path'], evidence_sha256=inventory_evidence['sha256'],
            files=[{key: item[key] for key in ('path', 'sha256')} for item in row['files']],
            source_role_review=semantic)
        output_profiles[identifier] = dict(benchmark=row['benchmark'], variant=row['variant'],
            source=row['source'], source_sha256=row['source_sha256'], producer_loc=counted['producer_loc'],
            physical_loc=sum(item['physical_loc'] for item in bundle),
            canonical_loc=sum(item['normalized_loc'] for item in bundle),
            expansions=[dict(source=item['path'], expansions=replay['source_counts'][item['path']]['metric']['expansions']) for item in bundle],
            source_bundle=bundle, semantic_review=semantic)
    return dict(qualified_inputs=qualified_inputs, profiles=output_profiles,
                status='complete-validated-overlay-payload-awaiting-evidence-write',
                canonical_LOC_admitted=False, chart_admission=False)
