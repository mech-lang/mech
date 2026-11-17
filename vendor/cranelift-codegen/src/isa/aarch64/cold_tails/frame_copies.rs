//! Reuse exact frame copies preceding a common continuation.

use super::{B, Branch, Flow, Replacement};
use alloc::{vec, vec::Vec};
use core::cmp::Reverse;

// Bound discovery even when the function has a long run of frame copies.
const MIN_COPIES: usize = 3;
const MAX_COPIES: usize = 64;

fn frame_copy(word: u32, frame_size: u32) -> bool {
    // LDR/STR Xt, [SP, #imm12 * 8], with no writeback. Exclude FP and LR
    // destinations so an eligible sequence has no architectural frame action.
    matches!(word & 0xffc0_03e0, 0xf900_03e0 | 0xf940_03e0)
        && word & 31 < 29
        && (((word >> 10) & 0xfff) + 1) * 8 <= frame_size
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Candidate {
    copies: usize,
    begin: usize,
    jump: usize,
    retained: usize,
    continuation: usize,
    fallthrough_entry: bool,
}

pub(super) fn plan(
    words: &[u32],
    flows: &[Flow],
    heads: &[bool],
    protected: &[bool],
    first: usize,
    frame_size: u32,
) -> Vec<Option<Replacement>> {
    let mut branch_entries = vec![0u8; words.len()];
    for flow in flows {
        if let Flow::Branch(Branch { target, .. }) = *flow {
            branch_entries[target] = branch_entries[target].saturating_add(1);
        }
    }
    let mut candidates = Vec::new();
    for jump in first..words.len() {
        let Flow::Branch(Branch { target, .. }) = flows[jump] else {
            continue;
        };
        if words[jump] & 0xfc00_0000 != B || protected[jump] || heads[jump] {
            continue;
        }

        // The retained copy is the sequence immediately preceding its
        // continuation. It already falls through. A rewritten fallthrough
        // entry therefore moves one existing branch from after to before its
        // frame copies. Explicitly targeted entries can share the copy directly.
        let mut copies = 0;
        while copies < MAX_COPIES {
            let from = jump - copies;
            let retained = target - copies;
            if from <= first
                || retained <= first
                || heads[from]
                || (copies != 0 && heads[retained])
                || protected[from - 1]
                || protected[retained - 1]
                || words[from - 1] != words[retained - 1]
                || !frame_copy(words[from - 1], frame_size)
            {
                break;
            }
            copies += 1;
        }
        // Keep the leading instruction of a multi-entry dispatch block at
        // its own label. Its trailing copy sequence remains eligible.
        if copies != 0 && branch_entries[jump - copies] > 1 {
            copies -= 1;
        }
        if copies >= MIN_COPIES {
            let begin = jump - copies;
            let prior = begin - 1;
            let prior_unconditional =
                words[prior] & 0xfc00_0000 == B && matches!(flows[prior], Flow::Branch(_));
            candidates.push(Candidate {
                copies,
                begin,
                jump,
                retained: target - copies,
                continuation: target,
                fallthrough_entry: !prior_unconditional && !matches!(flows[prior], Flow::End),
            });
        }
    }

    let mut occupied = vec![false; words.len()];
    let mut removed = vec![false; words.len()];
    let mut replacements = vec![None; words.len()];
    candidates.sort_unstable_by_key(|site| {
        (
            Reverse(site.copies),
            Reverse(site.begin),
            Reverse(site.retained),
        )
    });
    for site in candidates {
        let Candidate {
            begin,
            jump,
            retained,
            continuation,
            fallthrough_entry,
            ..
        } = site;
        let touched = begin..=jump;
        let retained_code = retained..=continuation;
        if (begin <= continuation && retained <= jump)
            || occupied[touched.clone()].iter().any(|&entry| entry)
            || removed[retained_code.clone()].iter().any(|&entry| entry)
        {
            continue;
        }
        occupied[touched.clone()].fill(true);
        occupied[retained_code].fill(true);
        removed[touched].fill(true);
        replacements[begin] = Some(Replacement {
            target: retained,
            width: jump - begin + 1,
            emit_branch: fallthrough_entry,
            redirect_branches: true,
        });
    }
    replacements
}
