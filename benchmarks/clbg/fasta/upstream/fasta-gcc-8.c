/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * This is intended to be a straightforward, simple, idiomatic C
 * implementation of the fasta benchmark, using stdio, all
 * computation, no caching, and a single thread.  No knowledge of the
 * random number generator is assumed.  It just relies on the C
 * compiler and libraries for performance.  It's a starting point to
 * further optimize C implementations.
 *
 * by Drake Diedrich
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define IM 139968
#define IA   3877
#define IC  29573
#define SEED   42
static uint32_t seed = SEED;
float fasta_rand(float max) {
  seed = (seed * IA + IC ) % IM;
  return max * seed / IM;
}

static const char *alu =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

static const char *iub = "acgtBDHKMNRSVWY";
static const double iub_p[] = {
  0.27,
  0.12,
  0.12,
  0.27,
  0.02,
  0.02,
  0.02,
  0.02,
  0.02,
  0.02,
  0.02,
  0.02,
  0.02,
  0.02,
  0.02 };

static const char *homosapiens = "acgt";
static const double homosapiens_p[] = {
  0.3029549426680,
  0.1979883004921,
  0.1975473066391,
  0.3015094502008
};

#define LINELEN 60

// slowest character-at-a-time output
static void repeat_fasta(const char *seq,
			 const int n) {
  const int len = strlen(seq);
  int i;
  for (i=0; i<n; i++) {
    putchar(seq[i % len]);
    if (i % LINELEN == LINELEN - 1) putchar('\n');
  }
  if (i % LINELEN != 0) putchar('\n');
}

static void random_fasta(const char *symb,
			 const double *probability,
			 const int n) {
  const int len = strlen(symb);
  int i,j;
  for (i=0; i<n; i++) {
    double v = fasta_rand(1.0);
    /* slowest idiomatic linear lookup.  Fast if len is short though. */
    for (j=0; j<len-1; j++) {
      v -= probability[j];
      if (v<0) break;
    }
    putchar(symb[j]);
    if (i % LINELEN == LINELEN - 1) putchar('\n');
  }
  if (i % LINELEN != 0) putchar('\n');
}

int main(int argc, char **argv) {
  int n=1000;
  if (argc>1) n = atoi(argv[1]);
  setlinebuf(stdout);
  printf(">ONE Homo sapiens alu\n");
  repeat_fasta(alu, n*2);
  
  printf(">TWO IUB ambiguity codes\n");
  random_fasta(iub, iub_p, n*3);
  
  printf(">THREE Homo sapiens frequency\n");
  random_fasta(homosapiens, homosapiens_p, n*5);
}
