"""Partition the unchanged, counted EKF extracts using the original counter."""
from pathlib import Path
import csv, json, sys, hashlib
ROOT = Path(__file__).resolve().parent
BASE = ROOT
OUTPUT = ROOT / 'proof'
OUTPUT.mkdir(parents=True, exist_ok=True)
sys.path.insert(0, str(BASE))
from measure import measure
PARTITIONS = {
    'ekf.mec': {
        'Setup, constants and declarations': [(1,27)],
        'EKF update and local candidate handling': [(28,63),(84,87)],
        'Validation and component extraction': [(64,83)],
        'Matrix helpers': [],
        'SIMD wrapper and adapters': [],
        'Batch dispatch, workers and rollback': [],
    },
    'rust_textbook.rs': {
        'Setup, constants and declarations': [(1,29),(193,196)],
        'EKF update and local candidate handling': [(63,156)],
        'Validation and component extraction': [(157,174)],
        'Matrix helpers': [(175,192)],
        'SIMD wrapper and adapters': [],
        'Batch dispatch, workers and rollback': [(30,62)],
    },
    'rust_simd.rs': {
        'Setup, constants and declarations': [(1,5),(92,99),(246,259),(395,401)],
        'EKF update and local candidate handling': [(159,245)],
        'Validation and component extraction': [(119,158)],
        'Matrix helpers': [(100,118)],
        'SIMD wrapper and adapters': [(6,91)],
        'Batch dispatch, workers and rollback': [(260,394)],
    },
}
ALL = {}
for name, parts in PARTITIONS.items():
    text = (BASE/'selected'/name).read_text()
    totals, records = measure(text, 'mech' if name.endswith('.mec') else 'rust')
    pinned = json.loads((BASE/'counts.json').read_text())[name]
    assert hashlib.sha256(text.encode()).hexdigest() == pinned['sha256']
    assert totals == {k: v for k, v in pinned.items() if k != 'sha256'}
    out=[]
    for r in records:
        buckets=[k for k,rs in parts.items() if any(a<=r['line']<=b for a,b in rs)]
        assert len(buckets)==1, (name,r,buckets)
        r['category']=buckets[0]
    for category, ranges in parts.items():
        selected=[r for r in records if r['category']==category]
        out.append({'category':category,'line_ranges':ranges,
                    'normalized_characters':sum(r['width1'] for r in selected),
                    'original_name_characters':sum(r['original_width'] for r in selected)})
    assert sum(r['normalized_characters'] for r in out)==totals['normalized_characters']
    ALL[name]={'sha256':hashlib.sha256(text.encode()).hexdigest(),'total':totals, 'parts':out}
    with (OUTPUT/f'{name}-categorized-tokens.tsv').open('w',newline='') as f:
        w=csv.DictWriter(f, fieldnames=records[0].keys(), delimiter='\t')
        w.writeheader(); w.writerows(records)
    print(name, '\n', '\n'.join(f"{x['normalized_characters']:5} {x['category']} {x['line_ranges']}" for x in out))
    print('TOTAL',totals['normalized_characters'])
(OUTPUT/'breakdown.json').write_text(json.dumps(ALL,indent=2)+'\n')
with (OUTPUT/'breakdown.csv').open('w',newline='') as f:
    w=csv.writer(f); w.writerow(['category',*ALL.keys()])
    for i,cat in enumerate(next(iter(PARTITIONS.values()))):
        w.writerow([cat,*[x['parts'][i]['normalized_characters'] for x in ALL.values()]])
    w.writerow(['TOTAL',*[x['total']['normalized_characters'] for x in ALL.values()]])

# Reproduce the exact excerpt subtotals alongside the full partition.
EXCERPTS = {
    'mech_correction': ('ekf.mec', 60, 62, 38),
    'rust_textbook_correction': ('rust_textbook.rs', 128, 146, 240),
    'rust_simd_arithmetic_trait_wrappers': ('rust_simd.rs', 57, 91, 480),
}
examples = {}
for label, (name, start, end, expected) in EXCERPTS.items():
    text = (BASE/'selected'/name).read_text(encoding='utf-8')
    excerpt = ''.join(text.splitlines(True)[start-1:end])
    metrics, _ = measure(excerpt, 'mech' if name.endswith('.mec') else 'rust')
    assert metrics['normalized_characters'] == expected, (label, metrics)
    examples[label] = {'source': name, 'line_ranges': [[start, end]],
                       'normalized_characters': metrics['normalized_characters']}
    (OUTPUT/f'{label}.txt').write_text(excerpt, encoding='utf-8')
(OUTPUT/'examples.json').write_text(json.dumps(examples, indent=2)+'\n', encoding='utf-8')
