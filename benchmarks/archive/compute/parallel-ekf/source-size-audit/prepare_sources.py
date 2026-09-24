#!/usr/bin/env python3
"""Extract audited sources and apply the bounded scalar contract correction."""
from pathlib import Path
import difflib
import hashlib

ROOT = Path(__file__).resolve().parent
BLOBS = {
    'ekf.mec': '6658d76c331b52303b64d9e94fdf328e4ed2b709',
    'rust_simd.rs': 'b00fa105b195e8231a2efb8667d7adc4ca684d99',
    'rust_scalar.rs': '5a5fcd7d35786adee9766109ccb454de4f03aca6',
}
SPANS = {
    'ekf.mec': [(1,1), (16,16), (19,103)],
    'rust_simd.rs': [(3,245), (259,272), (325,459), (476,479), (481,483)],
    'rust_textbook.before.rs': [(2,17), (96,248), (23,26)],
}

DISPATCH = '''fn dispatch<const CHECKED: bool>(
    state: &mut [f32],
    covariance: &mut [f32],
    velocity: &[f32],
    angular_velocity: &[f32],
    bearing: &[f32],
    turns: u32,
    scratch: &mut Scratch,
) -> Option<Fault> {
    // The caller observes only the completed block. Rejecting any candidate
    // restores the entire population to its state before the block began.
    let checkpoint = CHECKED.then(|| (state.to_vec(), covariance.to_vec()));
    for turn in 0..turns {
        for lane in 0..velocity.len() {
            let constraint = step::<CHECKED>(
                &mut state[lane * 3..lane * 3 + 3],
                &mut covariance[lane * 9..lane * 9 + 9],
                velocity[lane],
                angular_velocity[lane],
                bearing[lane],
                scratch,
            );
            if constraint != 0 {
                if let Some((checkpoint_state, checkpoint_covariance)) = checkpoint {
                    state.copy_from_slice(&checkpoint_state);
                    covariance.copy_from_slice(&checkpoint_covariance);
                }
                return Some(Fault { turn, instance: lane, constraint });
            }
        }
    }
    None
}
'''
VALIDATION = '''#[inline(always)]
fn candidate_fault(state: &[f32; 3], covariance: &[f32; 9]) -> u8 {
    if !state.iter().all(|value| value.is_finite())
        || !covariance.iter().all(|value| value.is_finite())
    {
        return 1;
    }
    if !(covariance[0] > 0.0 && covariance[4] > 0.0 && covariance[8] > 0.0) {
        return 2;
    }
    if !((covariance[1] - covariance[3]).abs() <= 1.0e-4
        && (covariance[2] - covariance[6]).abs() <= 1.0e-4
        && (covariance[5] - covariance[7]).abs() <= 1.0e-4)
    {
        return 3;
    }
    0
}
'''
FAULT = '''#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Fault {
    turn: u32,
    instance: usize,
    constraint: u8,
}
'''


def prepare():
    for directory in ('selected', 'evidence', 'normalized'):
        (ROOT/directory).mkdir(parents=True, exist_ok=True)
    originals = {}
    for name, expected in BLOBS.items():
        data = (ROOT/'originals'/name).read_bytes()
        actual = hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
        assert actual == expected, (name, actual)
        originals[name] = data.decode('utf-8')

    def pick(name, spans):
        lines = originals[name].splitlines(True)
        return ''.join(''.join(lines[a-1:b]) for a,b in spans)

    (ROOT/'selected/ekf.mec').write_text(pick('ekf.mec', SPANS['ekf.mec']))
    simd = 'use std::slice;\nuse wide::f32x4;\n'+pick('rust_simd.rs', SPANS['rust_simd.rs'])
    (ROOT/'selected/rust_simd.rs').write_text(simd)
    before = pick('rust_scalar.rs', SPANS['rust_textbook.before.rs'])
    (ROOT/'evidence/rust_textbook.before.rs').write_text(before)
    after = before[:before.index('fn dispatch<')] + DISPATCH + before[before.index('#[inline(always)]\nfn step<'):]
    after = after.replace(') -> bool {\n    let dt =', ') -> u8 {\n    let dt =', 1)
    old = '''    if CHECKED && !valid_candidate(&candidate_state, &s.corrected_p) {
        return false;
    }
    state.copy_from_slice(&candidate_state);
    covariance.copy_from_slice(&s.corrected_p);
    true
}'''
    new = '''    if CHECKED {
        let constraint = candidate_fault(&candidate_state, &s.corrected_p);
        if constraint != 0 {
            return constraint;
        }
    }
    state.copy_from_slice(&candidate_state);
    covariance.copy_from_slice(&s.corrected_p);
    0
}'''
    assert after.count(old) == 1
    after = after.replace(old, new)
    start = after.index('#[inline(always)]\nfn valid_candidate(')
    end = after.index('#[inline(always)]\nfn matmul(')
    after = FAULT + after[:start] + VALIDATION + after[end:]
    (ROOT/'selected/rust_textbook.rs').write_text(after)
    (ROOT/'evidence/textbook-contract.patch').write_text(''.join(difflib.unified_diff(
        before.splitlines(True), after.splitlines(True),
        fromfile='archived-textbook-extract.rs', tofile='block-atomic-textbook-extract.rs')))
    # The numeric body and the hand-written matrix helpers stay verbatim.
    math_start = '    let dt = 0.1_f32;'
    math_end = '    if CHECKED'
    assert before[before.index(math_start):before.index(math_end)] == after[after.index(math_start):after.index(math_end)]
    assert before[before.index('#[inline(always)]\nfn matmul('):] == after[after.index('#[inline(always)]\nfn matmul('):]
    return before, after

if __name__ == '__main__':
    prepare()
