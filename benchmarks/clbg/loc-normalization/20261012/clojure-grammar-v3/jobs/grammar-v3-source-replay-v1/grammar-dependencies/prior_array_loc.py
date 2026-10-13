"""Unhooked preservation/review for seventeen exact prior array source pins.

The caller must first verify source_text's UTF-8 bytes against sha256 and verify
the returned producer/manifest provenance pins. This module performs no hashes
and does not admit workloads or samples. It is not a general Uiua/APL lexer or
normalization certificate. No producer files or program records are changed.
"""
from copy import deepcopy
import json
from pathlib import Path


COHORT = 'r-apl-uiua-comparison/20261011'
UIUA_AUDIT = COHORT + '/source-audit-uiua.json'
APL_AUDIT = COHORT + '/source-audit-apl.json'
MANDEL_MANIFEST = COHORT + '/uiua-mandel-parallel-v4/full-study/manifest.json'
MANIFEST_PINS = {
    'uiua': 'c6cbef6595f2f5a8195b4edbe937ebe28165985245d73f34db17dd42a02dac32',
    'apl': 'b2da5804fa136b4e62990a90c49414a1b567346c06c3819b653c2678cd4eccdc',
    'mandelbrot-study': 'd286d36a47de8eda58a8999c6d4c00c498308eda09ad8de89a7ea616b495d1a6',
}
EVIDENCE_DIRECTORY = COHORT + '/evidence/explorer-mandelbrot-knucleotide-20261012'
RECEIPT = EVIDENCE_DIRECTORY + '/import-receipt.json'
RECEIPT_PIN = 'ad69ff7c5d6989a5efa9f7deeeb76cce8afef6b72d2950684737a1d14b4892dd'
RECEIPT_PIN_ORIGIN = EVIDENCE_DIRECTORY + '/push-plan.json'
PRODUCER_PINS = {
    APL_AUDIT: dict(sha256='9b26c06182bb97b4d3e26d70d9edd6e5ec78e57f1b59a77a91874869c9e83936',
        archive=EVIDENCE_DIRECTORY + '/timings-apl.json',
        archive_sha256='9ef8f3776ce6c7ddf65a29cec7378e782188862a271aea2826df486cac0f98f7',
        receipt_snapshot_key='benchmarks/clbg/' + COHORT + '/results/timings-apl.json'),
    UIUA_AUDIT: dict(sha256='82b15648d0c49c6393a0eec1efb441f1ee11237ceae4a8f41ca854c0b7ee0860',
        archive=EVIDENCE_DIRECTORY + '/timings-uiua.json',
        archive_sha256='004c830d5ffbf56e62809dcc65e3f6993b1599a31a4768ae2a522d3d6d74bcac',
        receipt_snapshot_key='benchmarks/clbg/' + COHORT + '/results/timings-uiua.json'),
    MANDEL_MANIFEST: dict(sha256=MANIFEST_PINS['mandelbrot-study'],
        archive=EVIDENCE_DIRECTORY + '/uiua-manifest.json',
        archive_sha256=MANIFEST_PINS['mandelbrot-study'],
        receipt_snapshot_key='benchmarks/clbg/' + MANDEL_MANIFEST),
}
APL_RULE = ('One independent statement per line; comments/blanks and standalone closing ∇ excluded; '
            'function headers, I/O and termination count. No diamonds or combined assignments.')
UIUA_BASE_RULE = ('Nonblank noncomment physical source lines; each independent binding expands to its own LOC. '
                  'Tacit pipelines, branches, long literals and one data declaration remain one expression. No source packing discount.')
UIUA_SPECTRAL_RULE = ('Each independent binding/data declaration is a separate nonblank noncomment line. '
                      'Ordinary tacit pipelines remain one expression; no source packing discount.')
UIUA_KNUC_RULE = ('Every independent binding and data declaration occupies one separate line. '
                  'The unchanged multiline formatter retains its physical lines; ordinary pipelines are single expressions.')
UIUA_FASTA_RULE = ('Every independent binding, datatype declaration and function-body line is counted; '
                   'all formatter and ALU chunk helpers are included. No packed-binding discount.')
UIUA_MANDEL11_RULE = ('Independent bindings and datatype declaration each occupy separate lines; no packed-binding discount.')
UIUA_MANDEL16_RULE = ('Every independent binding, record declaration, helper and output statement is on its own counted line.')

# Exact previously displayed source identities, not language-wide exemptions.
# Each tuple is language, source SHA-256, producer LOC, producer policy, provenance.
SOURCES = {
    'spectralnorm:spectralnorm-apl-compact': ('apl', '138dac87e5b26e8fea7a1b637468d79225a89685ee98c08b3fc4f238f86c7d61', 27, APL_RULE, APL_AUDIT),
    'spectralnorm:spectralnorm-apl-optimized': ('apl', '34f1a097f3da19cf7b4435812c5f0256c21b6f0ae0c1982386361d114958f237', 29, APL_RULE, APL_AUDIT),
    'pidigits:pidigits-apl-compact': ('apl', '6a691668ec0835a4930cdc550dfceb5f04bbfbce08368e47834e26dd5be9ab76', 55, APL_RULE, APL_AUDIT),
    'regexredux:regexredux-apl-v3-optimized': ('apl', 'bfcfeb21c2753bb6217b26e2db1cff17eed762d9ade7d27077d523e2b27401d8', 33, APL_RULE, APL_AUDIT),
    'revcomp:revcomp-apl-v2-compact': ('apl', '611f8f040cd5893993d5d43830ff4c7e74c0c3bf1cc6b8f4832f616de7400d38', 51, APL_RULE, APL_AUDIT),
    'revcomp:revcomp-apl-v2-optimized': ('apl', 'a4fcaa32c69f215bc07a9d1794bd72ceb764bd47059dfd4ec9490b0a8ab4d991', 71, APL_RULE, APL_AUDIT),
    'fasta:fasta-uiua-compact': ('uiua', 'fba046bc5d20288547051f1fa56a80d52217edd614ed0efbbabe6ef16353a626', 22, UIUA_BASE_RULE, UIUA_AUDIT),
    'revcomp:revcomp-uiua-optimized': ('uiua', '62f7d39f115c255826b1a771ea68670903e639cf80699fff5aff5def4fd1cc2e', 11, UIUA_BASE_RULE, UIUA_AUDIT),
    'regexredux:regexredux-uiua-compact': ('uiua', 'dc57b95038cfb33cf10360eac4dde3259f2d045ac42b38e56ed3ea2afef6fb9e', 16, UIUA_BASE_RULE, UIUA_AUDIT),
    'pidigits:pidigits-uiua-compact': ('uiua', 'f62e41833adfb713529afca4eb8d60eee52c7df5a38e7f3858ac73e756585148', 46, UIUA_BASE_RULE, UIUA_AUDIT),
    'spectralnorm:spectralnorm-uiua-vectorized-compact9-v3': ('uiua', '6f715834b433fb85cb428e208a6140962d5e6ab770c424e4199da3063e4db9a3', 9, UIUA_SPECTRAL_RULE, UIUA_AUDIT),
    'spectralnorm:spectralnorm-uiua-vectorized-optimized-v3': ('uiua', '622f019150b668537f0cbcfb6dcd8322c3a32d9ce73fc648688231403c4d33a3', 17, UIUA_SPECTRAL_RULE, UIUA_AUDIT),
    'knucleotide:knucleotide-uiua-bounded-optimized-v3': ('uiua', 'be3f21d43c56a8b4de2eb661552ff5c13d9390939e34a205c48a878fcbb155ff', 27, UIUA_BASE_RULE, UIUA_AUDIT),
    'knucleotide:knucleotide-uiua-native-group-optimized-v3': ('uiua', '65afb1ed3299583c0ea1ea83224b5b9e0e954363c2d6bfde7bcf30e5f1b661c1', 29, UIUA_KNUC_RULE, UIUA_AUDIT),
    'fasta:fasta-uiua-batched-output-optimized-v2': ('uiua', '552af2b9a71bad5dd79a7f8bf4f6f74ae408ee9226415d084463a5950d3ba4e7', 34, UIUA_FASTA_RULE, UIUA_AUDIT),
    'mandelbrot:uiua-parallel-study:whole16-parallel': ('uiua', '183bc351c4428b3c1ff4c23e5d1a19eb6ba6658181142e9267f4b59f92f22569', 16, UIUA_MANDEL16_RULE, MANDEL_MANIFEST),
    'mandelbrot:uiua-parallel-study:whole11': ('uiua', '64f2913ec83009b70cc3ac8c4064438732bc1faf4448af198b7052e4cb0aaa4e', 11, UIUA_MANDEL11_RULE, MANDEL_MANIFEST),
}


def _apl_without_comment(line):
    """Remove an APL line comment while protecting quoted ⍝ and doubled quotes."""
    quoted = False
    index = 0
    while index < len(line):
        character = line[index]
        if character == "'":
            if quoted and index + 1 < len(line) and line[index + 1] == "'":
                index += 2
                continue
            quoted = not quoted
        elif character == '⍝' and not quoted:
            return line[:index]
        index += 1
    return line


def preserve(source_text, language, sha256, current_program, root):
    """Return a prior pinned Uiua metric or an explicitly unadmitted APL review.

    Unknown identities or changed producer metadata return None. APL metrics
    have normalized_loc=None and require explicit review before any proposed
    closing-line increments can affect a chart. Producer records are read only.
    """
    key = current_program.get('key')
    identity = SOURCES.get(key)
    if identity is None:
        return None
    expected_language, expected_sha, expected_loc, expected_policy, source_report = identity
    if (language != expected_language or sha256 != expected_sha or
            current_program.get('language') != expected_language or
            current_program.get('source_report') != source_report or
            current_program.get('loc_metric') != 'statement-normalized' or
            current_program.get('loc') != expected_loc or
            sha256 not in current_program.get('files', [])):
        return None
    root = Path(root)

    def path_for(relative):
        path = Path(relative)
        return path if path.is_absolute() else root / path

    def load(relative):
        path = path_for(relative)
        if not path.is_file():
            return None
        try:
            return json.loads(path.read_text())
        except (ValueError, OSError):
            return None

    producer = load(source_report)
    if not producer:
        return None
    producer_pin = PRODUCER_PINS[source_report]
    archived = load(producer_pin['archive'])
    receipt = load(RECEIPT)
    if not archived or not receipt:
        return None
    archived_pin = receipt.get('snapshots', {}).get(producer_pin['receipt_snapshot_key'], {})
    if (archived_pin.get('sha256') != producer_pin['archive_sha256'] or
            archived_pin.get('snapshot') != 'benchmarks/clbg/' + producer_pin['archive']):
        return None
    study = source_report == MANDEL_MANIFEST
    if study:
        if (producer.get('status') != 'prepared-awaiting-root-review' or
                current_program.get('study_manifest_sha256') != MANIFEST_PINS['mandelbrot-study']):
            return None
        manifest, manifest_path, manifest_pin = producer, source_report, MANIFEST_PINS['mandelbrot-study']
        rows = producer.get('profiles', [])
    else:
        if (archived.get('source_audit_sha256') != producer_pin['sha256'] or
                not archived.get('source_audit') or
                str(path_for(archived['source_audit'])) != str(path_for(source_report))):
            return None
        if producer.get('status') != 'reviewed' or producer.get('language') != language:
            return None
        manifest_path = producer.get('manifest')
        manifest_pin = producer.get('manifest_sha256')
        if not manifest_path or manifest_pin != MANIFEST_PINS[language]:
            return None
        manifest = load(manifest_path)
        if not manifest or manifest.get('language') != language or manifest.get('status') != 'prepared':
            return None
        rows = producer.get('programs', [])
    matches = [row for row in rows if row.get('key') == key]
    if len(matches) != 1:
        return None
    row = matches[0]
    policy = row.get('loc_rule') if language == 'apl' else row.get('loc_review')
    if (row.get('language') != language or row.get('source_sha256') != sha256 or
            row.get('loc') != expected_loc or policy != expected_policy or not row.get('source')):
        return None
    if producer.get('pins', {}).get(row['source']) != sha256:
        return None
    if not study:
        manifest_matches = [item for item in manifest.get('programs', []) if item.get('key') == key]
        if len(manifest_matches) != 1:
            return None
        mirrored = manifest_matches[0]
        if any(mirrored.get(field) != row.get(field) for field in ('source', 'source_sha256', 'loc')):
            return None
        if mirrored.get('loc_rule' if language == 'apl' else 'loc_review') != expected_policy:
            return None
    else:
        archived_rows = [item for item in archived.get('profiles', []) if item.get('key') == key]
        if len(archived_rows) != 1:
            return None
        if any(archived_rows[0].get(field) != row.get(field)
               for field in ('source', 'source_sha256', 'loc', 'physical_loc', 'loc_expansions', 'loc_review')):
            return None
    expansions = deepcopy(row.get('loc_expansions', []))
    if current_program.get('loc_expansions', []) != expansions:
        return None
    physical = row.get('physical_loc', expected_loc)
    if current_program.get('physical_loc') != physical:
        return None
    provenance = dict(source_report=source_report, producer_status=producer['status'],
        expected_producer_sha256=producer_pin['sha256'],
        producer_key=key, source=str(path_for(row['source'])), source_sha256=sha256,
        producer_loc=expected_loc, producer_policy=expected_policy,
        control_snapshot=str(path_for(manifest_path)), expected_snapshot_sha256=manifest_pin,
        provenance_pins={str(path_for(source_report)):producer_pin['sha256'],
                         str(path_for(manifest_path)):manifest_pin,
                         str(path_for(producer_pin['archive'])):producer_pin['archive_sha256'],
                         str(path_for(RECEIPT)):RECEIPT_PIN},
        producer_pin_evidence=dict(archived_metadata=producer_pin['archive'],
            archived_metadata_expected_sha256=producer_pin['archive_sha256'],
            receipt=RECEIPT, receipt_expected_sha256=RECEIPT_PIN,
            receipt_snapshot_key=producer_pin['receipt_snapshot_key'],
            receipt_pin_origin=RECEIPT_PIN_ORIGIN,
            pin_origin_field='exact_changed_paths entry for import-receipt.json'),
        required_verification='Caller verifies source UTF-8 SHA-256 and every provenance_pins expected digest before preservation or review; no hashes are performed here.')
    common = dict(rule_version='prior-array-source-policy-v1', language=language, sha256=sha256,
        prior_normalized_loc=expected_loc, prior_physical_loc=physical,
        expansions=expansions, loc_expansions=deepcopy(expansions), canonical_events=[],
        provenance=provenance, scope='Exact registered prior source identity only; no general language certification or workload admission.')
    if language == 'uiua':
        return dict(common, physical_loc=physical, normalized_loc=expected_loc,
            review_status='prior-source-audit-preserved',
            diagnostics=[dict(kind='prior-source-audit-preserved',
                message='Retain this exact reviewed producer count and independent-binding policy; no generic Uiua lexer certificate is claimed.')])

    written = []
    code_lines = []
    for number, original in enumerate(source_text.splitlines(), 1):
        code = _apl_without_comment(original).strip()
        if not code:
            continue
        code_lines.append(number)
        if code == '∇':
            written.append(dict(line=number, original=original, extra_lines=1,
                reason='Written standalone closing ∇ was excluded by the prior producer rule.'))
    remaining = len(code_lines) - len(written)
    matches_prior = remaining == expected_loc
    return dict(common, physical_loc=len(code_lines), normalized_loc=None,
        proposed_normalized_loc=expected_loc + len(written) if matches_prior else None,
        closing_line_increments=written, extra_closing_lines=len(written),
        prior_nonclosing_count_matches=matches_prior, review_status='needs-explicit-closing-line-review',
        diagnostics=[dict(kind='apl-prior-closing-line-exclusion-review',
            message='Review the exact written closing ∇ lines and restore uniform closer counting before admitting a display count. The prior count and producer records remain unchanged.'),
            *([] if matches_prior else [dict(kind='apl-prior-source-count-mismatch',
                message='Noncomment/nonclosing written lines do not match this prior producer count; no proposed count is supplied.')])])
