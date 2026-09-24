#!/usr/bin/env python3
"""Reproduce a source-specific, weighted lexical EKF size comparison.

The normal form is an audit stream, NOT executable source. The parser here is
only a lexer; explicit name classes are reviewed for these hash-pinned inputs.
"""
from __future__ import annotations
from collections import Counter
from pathlib import Path
import csv
import hashlib
import json
from lexer import tokens

ROOT = Path(__file__).resolve().parent
BLOBS = {
    'ekf.mec': '6658d76c331b52303b64d9e94fdf328e4ed2b709',
    'rust_simd.rs': 'b00fa105b195e8231a2efb8667d7adc4ca684d99',
    'rust_scalar.rs': '5a5fcd7d35786adee9766109ccb454de4f03aca6',
}
RUST_FIXED = set('''std slice wide f32x4 f32 usize u8 u32 bool inline always
derive Clone Copy Debug Eq PartialEq Default ops Add Sub Mul Div Neg Output
add sub mul div neg array from_fn new to_array into_iter enumerate map iter
copied chain flat_map fold abs all Vec Option Some None copy_from_slice
iter_mut fill len max min as_mut_ptr as_ptr from_raw_parts_mut from_raw_parts
then clone thread with_capacity push spawn position filter_map join expect
min_by_key is_some assert vec _ default sin cos sum zip to_vec from'''.split())
# Freely chosen application names, including custom fields, parameters, labels,
# constants, types, functions, and custom (not trait-required) methods.
RUST_LOCAL = set('''V4 LANES DT SYMMETRY_TOLERANCE Matrix ROWS COLS INNER Fault
value values base lanes load store sin cos s c other left right result rhs turn
instance constraint input row column sum index state covariance faults diagonal
finite step_group candidate_faults theta sin_theta cos_theta distance
predicted_state f g ft gt process_noise predicted_covariance first second delta_x
delta_y squared_range predicted_bearing raw_innovation innovation_sin
innovation_cos innovation h ph_t innovation_variance gain next_state a identity
at corrected_base next_covariance reset dispatch_parallel_fused velocity
angular_velocity bearing turns checked workers groups state_ptrs covariance_ptrs
velocity_ptr angular_velocity_ptr bearing_ptr checkpoints handles worker
start_group end_group group_count count offset state_slices covariance_slices
velocities angular_velocities bearings first_fault group packed_state
packed_covariance matrix lane handle fault checkpoint_state checkpoint_covariance
instances Scratch predicted_p process_left process_p pht ap corrected_p dispatch
CHECKED scratch step candidate_state valid_candidate rows inner columns out
candidate_fault checkpoint fault_code transpose matmul dt b'''.split())
# sin and cos are local arrays only in the SIMD file; scalar calls are methods.
AMBIGUOUS = {'ZERO', 'splat', 'sin_cos', 'atan2', 'is_finite', 'scope', 'sin', 'cos', 'sum'}
MECH_FIXED = {'math', 'sin', 'cos', 'atan2', 'abs', 'f32', 'compute'}


def classify_rust(ts, i):
    t = ts[i]
    if t.kind != 'identifier':
        return False, t.kind
    word = t.text
    prev = [x.text for x in ts[max(0, i - 5):i]]
    if word in AMBIGUOUS:
        # External associated items in f32 / f32x4 and std::thread.
        if len(prev) >= 3 and prev[-2:] == [':', ':'] and prev[-3] in {'f32', 'f32x4', 'thread'}:
            return False, 'library associated item'
        if word in {'sin', 'cos', 'sum'}:
            return (prev[-1:] != ['.']), 'local binding or scalar math method'
        if word in {'sin_cos', 'atan2'}:
            # Only scalar invocations in V4's adapter bodies keep library names.
            # Every such invocation is before the first arithmetic trait impl.
            trait_start = next((j for j, q in enumerate(ts) if q.text == 'impl' and j+1 < len(ts) and ts[j+1].text == 'std'), len(ts))
            simd_file = any(q.text == 'V4' for q in ts)
            if prev[-1:] == ['.'] and (not simd_file or i < trait_start):
                return False, 'scalar math method'
        if word == 'is_finite' and not any(q.text == 'V4' for q in ts):
            return False, 'scalar math method'
        return True, 'custom item or local binding'
    # Local variable spellings may collide with preserved external item names.
    if word in RUST_LOCAL and word not in {'sin', 'cos'}:
        return True, 'application identifier'
    if word in RUST_FIXED:
        return False, 'required or library identifier'
    raise ValueError(f'Unclassified Rust identifier {word!r} at {t.offset}')


def measure(text, lang):
    ts = [t for t in tokens(text, lang) if t.kind not in {'whitespace', 'comment'}]
    records = []
    skip = set()
    for i, t in enumerate(ts):
        if i in skip:
            continue
        raw = t.text
        if lang == 'mech' and raw == 'EKF' and ts[i+1].text == 'step':
            raw = 'EKFstep'  # Whitespace outside literals excluded.
            skip.add(i+1)
            local, reason = True, 'one semantic section name'
        elif lang == 'mech':
            local = t.kind == 'identifier' and raw not in MECH_FIXED
            reason = 'application identifier' if local else t.kind
        else:
            local, reason = classify_rust(ts, i)
        records.append({
            'offset': t.offset,
            'line': text.count('\n', 0, t.offset)+1,
            'token': raw,
            'class': reason,
            'normalizable': local,
            'original_width': len(raw),
            'width1': 1 if local else len(raw),
            'width4': 4 if local else len(raw),
        })
    return {
        'normalized_characters': sum(r['width1'] for r in records),
        'width4_characters': sum(r['width4'] for r in records),
        'original_name_characters': sum(r['original_width'] for r in records),
        'normalized_identifier_occurrences': sum(r['normalizable'] for r in records),
    }, records


def main():
    from prepare_sources import prepare
    prepare()
    for name, expected in BLOBS.items():
        data = (ROOT/'originals'/name).read_bytes()
        actual = hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
        if actual != expected:
            raise ValueError(f'Original changed: {name}: {actual}')
    results = {}
    for name, lang in [('ekf.mec', 'mech'), ('rust_simd.rs', 'rust'), ('rust_textbook.rs', 'rust')]:
        p = ROOT/'selected'/name
        if not p.exists():
            continue
        text = p.read_text(encoding='utf-8')
        metric, records = measure(text, lang)
        metric['sha256'] = hashlib.sha256(p.read_bytes()).hexdigest()
        results[name] = metric
        with (ROOT/'normalized'/f'{name}-tokens.tsv').open('w', encoding='utf-8', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=records[0].keys(), delimiter='\t')
            writer.writeheader()
            writer.writerows(records)
        (ROOT/'normalized'/f'{name}.txt').write_text(''.join('x' if r['normalizable'] else r['token'] for r in records)+'\n')
    print(json.dumps(results, indent=2))
    (ROOT/'counts.json').write_text(json.dumps(results, indent=2)+'\n')
    with (ROOT/'counts.csv').open('w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['language','strategy','normalized_source_characters','source'])
        for language,strategy,source in [
            ('Mech','Textbook','ekf.mec'),
            ('Rust','Textbook (publication-normalized)','rust_textbook.rs'),
            ('Mech','SIMD-4 / 8 workers','ekf.mec'),
            ('Rust','SIMD-4 / 8 workers','rust_simd.rs'),
        ]:
            writer.writerow([language,strategy,results[source]['normalized_characters'],'selected/'+source])

if __name__ == '__main__':
    main()
