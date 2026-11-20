"""Pinned libdeflate sorter plus a C expression of Mech's current Shell sort."""
from hashlib import sha256

PIN = "a46dae12f2020b68d20c965ae43f9710e3e792bf77f3ca4611582f8d8d0c9541"

PREFIX = r'''
#include <assert.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
typedef uint32_t u32;
typedef uint8_t u8;
#define DEFLATE_MAX_NUM_SYMS 288
#define MIN(a,b) ((a) < (b) ? (a) : (b))
#ifndef SORT_COUNTERS
#define SORT_COUNTERS 0
#endif
struct sort_work {
    uint64_t frequency_reads, counter_updates, key_comparisons, output_writes;
};
static struct sort_work work;
#define COUNT(field) do { if (SORT_COUNTERS) ++work.field; } while (0)
#define COMPARE(expression) (SORT_COUNTERS ? (++work.key_comparisons, (expression)) : (expression))
void replay_reset_work(void) { memset(&work, 0, sizeof(work)); }
void replay_read_work(struct sort_work *out) { *out = work; }
extern void *mi_zalloc(size_t);
extern void mi_free(void *);
'''

SUFFIX = r'''
static unsigned mech_shell(unsigned n, const u32 *freqs, u32 *words) {
    unsigned used = 0;
    for (unsigned sym = 0; sym < n; ++sym) {
        COUNT(frequency_reads);
        u32 frequency = freqs[sym];
        if (frequency) {
            COUNT(output_writes);
            words[used++] = sym | (frequency << 10);
        }
    }
    uint64_t gaps = UINT64_C(4362938489);
    while (gaps & 255) {
        unsigned gap = gaps & 255;
        for (unsigned at = gap; at < used; ++at) {
            u32 key = words[at];
            unsigned position = at;
            for (;;) {
                unsigned previous = position >= gap ? position - gap : position;
                u32 prior = position >= gap ? words[previous] : key;
                int moving = COMPARE(prior > key);
                COUNT(output_writes);
                words[position] = moving ? prior : key;
                if (!moving) break;
                position = previous;
            }
        }
        gaps >>= 8;
    }
    return used;
}

unsigned replay_c_native(unsigned n, const u32 *freqs, u32 *out) {
    assert(n >= 2 && n <= 288);
    u32 *scratch = mi_zalloc(n * sizeof(*scratch));
    assert(scratch);
    u8 lens[288];
    unsigned used = sort_symbols(n, freqs, lens, scratch);
    memcpy(out, scratch, used * sizeof(*out));
    mi_free(scratch);
    return used;
}

unsigned replay_c_mech(unsigned n, const u32 *freqs, u32 *out) {
    assert(n >= 2 && n <= 288);
    u32 *scratch = mi_zalloc(n * sizeof(*scratch));
    assert(scratch);
    unsigned used = mech_shell(n, freqs, scratch);
    memcpy(out, scratch, used * sizeof(*out));
    mi_free(scratch);
    return used;
}
'''


def generate(source):
    assert sha256(source.encode()).hexdigest() == PIN
    license = source[:source.index('#include')]
    start = source.index('static void\nheapify_subtree(')
    end = source.index('/*\n * Build a Huffman tree.', start)
    native = source[start:end]
    changes = {
        'A[child_idx + 1] > A[child_idx]': 'COMPARE(A[child_idx + 1] > A[child_idx])',
        'v >= A[child_idx]': 'COMPARE(v >= A[child_idx])',
        '\t\tA[parent_idx] = A[child_idx];': '\t\tCOUNT(output_writes);\n\t\tA[parent_idx] = A[child_idx];',
        '\tA[parent_idx] = v;': '\tCOUNT(output_writes);\n\tA[parent_idx] = v;',
        '\t\tA[length] = A[1];': '\t\tCOUNT(output_writes);\n\t\tA[length] = A[1];',
        '\t\tA[1] = tmp;': '\t\tCOUNT(output_writes);\n\t\tA[1] = tmp;',
        '\tfor (sym = 0; sym < num_syms; sym++)\n\t\tcounters[MIN(freqs[sym], num_counters - 1)]++;':
            '\tfor (sym = 0; sym < num_syms; sym++) {\n\t\tCOUNT(frequency_reads);\n\t\tCOUNT(counter_updates);\n\t\tcounters[MIN(freqs[sym], num_counters - 1)]++;\n\t}',
        '\t\tcounters[i] = num_used_syms;': '\t\tCOUNT(counter_updates);\n\t\tcounters[i] = num_used_syms;',
        '\t\tu32 freq = freqs[sym];': '\t\tCOUNT(frequency_reads);\n\t\tu32 freq = freqs[sym];',
        '\t\t\tsymout[counters[MIN(freq, num_counters - 1)]++] =':
            '\t\t\tCOUNT(counter_updates);\n\t\t\tCOUNT(output_writes);\n\t\t\tsymout[counters[MIN(freq, num_counters - 1)]++] =',
    }
    for old, new in changes.items():
        assert native.count(old) == 1, (old, native.count(old))
        native = native.replace(old, new)
    return license + PREFIX + native + SUFFIX
