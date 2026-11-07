from pathlib import Path
from datetime import datetime, timezone
import hashlib, json

D = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
target = D / 'compiler-native-24-hour-progress-20261107.json'
history = D / 'ledger-history-20261107'
history.mkdir(exist_ok=True)
snapshot = history / 'compiler-native-24-hour-progress-before-actual-24h-correction.json'
if not snapshot.exists():
    snapshot.write_bytes(target.read_bytes())
old_name = 'comparison-native-frozen-uniform-layouts-confirmation-20261106-5000-m3.json'
new_name = 'comparison-native-movable-captures-randomized-20261107-5000-m3.json'
old_resources = 'comparison-native-frozen-uniform-layouts-resources-20261106-5000-m3.json'
new_resources = 'comparison-native-movable-captures-resources-20261107-5000-m3.json'
read = lambda name: json.loads((D / name).read_text())
a, b, c, d = map(read, [old_name, new_name, old_resources, new_resources])
assert a['input']['sha256'] == b['input']['sha256']
assert a['input']['posts'] == b['input']['posts'] == 5000
x, y = a['summary']['mech-after-4'], b['summary']['mech-ir242-reuse-4']
u, v = c['summary']['mech-after-4-instrumented'], d['summary']['mech-ir242-reuse-4-instrumented']
assert all(t['all_exact_rust'] for t in [x, y, u, v])
assert x['calculation_boundary'] == y['calculation_boundary'] == 'post_json_including_setup'
selection_receipt = D / 'compiler-native-frozen-uniform-layouts-candidate-20261106.json'
selection = read(selection_receipt.name)
assert selection['selected'] and selection['identity'] == 'IR208/work114/ABI40'
anchor = datetime(2026, 11, 7, 8, 31, 40, tzinfo=timezone.utc)
boundary = anchor.timestamp() - 86400
assert 0 <= boundary - selection_receipt.stat().st_mtime < 120

def sizes(name, variant):
    z = next(z for z in read(name)['implementations'] if z['variant'] == variant)
    return {k: z[k] for k in ['executable_file_bytes', 'executable_text_bytes', 'native_program']}

def entry(identity, compiler, variant, timing, resource, cohort, resource_cohort, size):
    return dict(identity=identity, compiler=compiler, workers=4, variant=variant,
                latency_cohort=cohort, resource_cohort=resource_cohort,
                timing=timing, processing_resources=resource['post_json_measurement_median'],
                restoration_resources=resource['stage_measurements_median']['restore'],
                whole_process_resources=resource['process_resources_median'], sizes=size)

then = entry(selection['identity'], selection['compiler'], 'mech-after-4', x, u, old_name, old_resources,
             sizes('size-native-frozen-uniform-layouts-20261106.json', 'mech-after-4'))
now = entry('IR242/work115/ABI41/image9', '4cc266ae0d05c69432a4c15ffe99d5e586685647',
            'mech-ir242-reuse-4', y, v, new_name, new_resources,
            sizes('size-native-movable-captures-20261107.json', 'mech-ir242-reuse-4'))
delta = lambda before, after: 100 * (after / before - 1)
changes = {k: delta(x[k], y[k]) for k in ['median_post_json_seconds', 'median_whole_process_seconds',
            'post_json_over_fastest_matched_native', 'whole_process_over_fastest']}
for name in ['processing_resources', 'restoration_resources']:
    changes[name] = {k: delta(then[name][k], now[name][k]) for k in
                     ['allocation_requests', 'requested_allocation_bytes', 'peak_live_requested_heap_bytes']}
    changes[name]['instructions_retired'] = delta(then[name]['hardware']['instructions_retired'], now[name]['hardware']['instructions_retired'])
    changes[name]['cycles'] = delta(then[name]['hardware']['cycles'], now[name]['hardware']['cycles'])
checkpoint = read('selected-speed-checkpoint-20261107.json')
assert checkpoint['identity'] == now['identity'] and checkpoint['compiler'] == now['compiler']
report = dict(
    schema='mech.related-post-gen.actual-24-hour-progress.v2',
    checked_at_utc=datetime.now(timezone.utc).isoformat(),
    comparison_anchor_utc=anchor.isoformat(),
    requested_24h_boundary_utc=datetime.fromtimestamp(boundary, timezone.utc).isoformat(),
    then_selection_receipt=selection_receipt.name,
    then_selection_receipt_mtime_utc=datetime.fromtimestamp(selection_receipt.stat().st_mtime, timezone.utc).isoformat(),
    then_selection_receipt_precedes_boundary_seconds=boundary-selection_receipt.stat().st_mtime,
    correction='The previous report used IR197 at approximately 30 hours before this question. The closer selected checkpoint is IR208; its qualification/selection receipt precedes the 24-hour boundary by 55 seconds. The original report is retained in ledger history.',
    previous_report=str(snapshot), previous_report_sha256=hashlib.sha256(snapshot.read_bytes()).hexdigest(),
    input=a['input'], then=then, now=now, percent_change=changes,
    then_matched_reference=a['matched_native_reference'], now_matched_reference=b['matched_native_reference'],
    then_whole_reference=a['whole_process_reference'], now_whole_reference=b['whole_process_reference'],
    current_original_promotion_timing=checkpoint['timing'],
    original_promotion_timing_unchanged=True,
    interpretation=[
        'Latest selected processing remeasurement is 13.16% shorter than the recorded IR208 confirmation; matched Rust ratio moves from 1.9321x to 1.6943x.',
        'Latest whole-process remeasurement is 2.69% longer; no end-to-end improvement over this day-ago checkpoint is established.',
        'Separate historical/current cohorts establish a trend, not a paired causal speedup. The faster remeasurement of unchanged IR242 is not a new compiler improvement.',
        'The current selected compiler remains IR242. IR248 is unselected: blanket movable capture loads add 8.15% processing instructions and fail the joint performance gate.',
        'Resource measurements use separate instrumented cohorts; their timings are excluded from latency claims.',
    ],
    goal_complete=False,
)
target.write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(dict(report=str(target), then_identity=then['identity'], now_identity=now['identity'],
                      post_ms=[x['median_post_json_seconds']*1000, y['median_post_json_seconds']*1000],
                      whole_ms=[x['median_whole_process_seconds']*1000, y['median_whole_process_seconds']*1000],
                      changes=changes)))
