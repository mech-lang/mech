/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * This benchmark uses a lookup table for the symbolic codes from the
 * output space of the random number generator, so in the case of an
 * random number generator with a modest number of outputs (the modulo
 * IM in this case), the table even fits in CPU cache.  For a
 * different random number generator with much larger number of output
 * states this strategy could still be used up to around 32 bits, but
 * then gets impractical.
 *
 * Using a table like this avoids using an iterative and branching
 * algorithm (whether linear or logarithmic tree-based).  This
 * eliminates several instructions in the innermost loops, as well as
 * elimnating pipeline busting decisions in those same loops and
 * better instructions per cycle.

 * The preprocessor random number generator allows the compiler to
 * optimize across both the memory address calculations for the output
 * buffer, the hash, and the random number generator.  In particular,
 * the modulo disappears and the constants are changed, but the
 * results are equivalent and faster.
 *
 * perf stat
 *  2,904,895,668      cycles                    #    4.306 GHz
 *  3,511,967,349      instructions              #    1.21  insn per cycle
 *
 * About 3/4 of time is spent in the table lookups and a little under
 * 1/4 in the random number generation. 2 sourcecode lines, 5 instructions.
 *
 * The main suboptimality in this code is that the memory operations
 * are single byte on 32 and 64 bit architectures, and the random
 * number generation is also just 32 bit on machines with at least 64
 * bit operations (and up to 512 bits). The instructions per cycle is
 * good, but not great (above 2.0), so 2-4x performance should be
 * possible with possibly explicit loop unwinding, larger data types,
 * and random number generator sequence skipping.  Attempts along this
 * line have not been fruitful yet, and they will be significantly more 
 * complex than this.
 *  Adding random number sequence fast skipping would also allow a threading
 * speedup, though it my be limited by memory bandwidth.
 *
 * by Drake Diedrich
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define IM 139968
#define IA   3877
#define IC  29573
#define SEED   42
static uint32_t seed = SEED;
#define uint32_rand() ( seed = (seed * IA + IC ) % IM )

/* tune up or down to adjust speed vs memory/cache tradeoffs */
#ifndef BUFLINES
#define BUFLINES 100
#endif

static const char *alu =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

static const char *iub = "acgtBDHKMNRSVWY";
static const float iub_p[] = {
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
static const float homosapiens_p[] = {
  0.3029549426680,
  0.1979883004921,
  0.1975473066391,
  0.3015094502008
};

#define LINELEN 60

static void repeat_fasta(const char *seq,
			 const int n) {
  const int len = strlen(seq);

  int buflen1 = len + LINELEN;
  char *buffer1 = malloc(buflen1);
  int i;
  if (LINELEN < len) {
    memcpy(buffer1,seq,len);
    memcpy(buffer1+len, seq, LINELEN);
  } else {
    for (i=0; i < LINELEN/len; i++) memcpy(buffer1+i*len, seq, len);
    memcpy(buffer1+i*len, seq, n - i*n);
  }

  int buflen2 = (LINELEN+1) * len;
  char *buffer2 = malloc(buflen2);
  for (i=0; i<len; i++) {
    memcpy(buffer2+i*(LINELEN+1), buffer1+((i*LINELEN)%len), LINELEN);
    buffer2[(i+1)*(LINELEN+1)-1] = '\n';
  }

  int whole_buffers = n / (len*LINELEN);
  for (i=0; i< whole_buffers; i++) write(1, buffer2, buflen2);

  int data_remaining = n - whole_buffers * len * LINELEN;
  int embedded_newlines = data_remaining / LINELEN;
  write(1, buffer2, data_remaining + embedded_newlines);

  free(buffer1);
  free(buffer2);
  if (n % LINELEN != 0) write(1, "\n", 1);
}

static char * build_hash(const char *symb,const float *probability) {
  int i,j;
  char *hash = malloc(IM);
  if (!hash) exit(-1);
  float sum = 0.0;
  const int len = strlen(symb);
  sum = probability[0];
  for (i=0,j=0;i<IM && j<len;i++) {
    float r = 1.0 * i / IM;
    if (r>=sum) {
      j++;
      sum += probability[j];
    }
    hash[i] = symb[j];
  }
  return hash;
}

static char * buffer_with_linebreaks(const int lines) {
  char *buffer = malloc((LINELEN+1)*lines);
  if (!buffer) exit(-1);
  for (int i=0; i<lines; i++) buffer[i*(LINELEN+1)+LINELEN] = '\n';
  return buffer;
}

static void random_fasta(const char *symb,
			 const float *probability,
			 const int n) {
  int i,j,k;

  char *hash = build_hash(symb,probability);

  char *buffer = buffer_with_linebreaks(BUFLINES);

  /* fill whole buffers for bulk of data
   * skips over linebreaks
   * minimizes decisions, just fixed length loops and computations */
  int buffers = n/LINELEN/BUFLINES;
  for (i=0; i<buffers; i++) {
    for (j=0;j<BUFLINES;j++) {
      for (k=0; k<LINELEN;k++) {
	uint32_t v = uint32_rand();
	buffer[j*(LINELEN+1)+k] = hash[v];
      }
    }
    write(1, buffer, (LINELEN+1)*BUFLINES);
  }

  /* handle remaining whole and partial lines as separate cases
     avoids putting conditionals in the bulk loop above */
  int lines = n/LINELEN - buffers*BUFLINES;
  for (j=0;j<lines;j++) {
    for (k=0; k<LINELEN;k++) {
      uint32_t v = uint32_rand();
      buffer[j*(LINELEN+1)+k] = hash[v];
    }
  }
  int partials = n - LINELEN*lines - buffers*BUFLINES*LINELEN;
  for (k=0; k<partials; k++) {
    uint32_t v = uint32_rand();
    buffer[lines*(LINELEN+1)+k] = hash[v];
  }
  write(1, buffer, lines*(LINELEN+1)+partials);

  /* unless entire output is an exact multiple of a linelength, add a newline */
  if (n % LINELEN != 0) write(1, "\n", 1);
  
  free(buffer);
  free(hash);
}

const char header1[] = ">ONE Homo sapiens alu\n";
const char header2[] = ">TWO IUB ambiguity codes\n";
const char header3[] = ">THREE Homo sapiens frequency\n";

int main(int argc, char **argv) {
  int n=1000;
  if (argc>1) n = atoi(argv[1]);

  write(1, header1, sizeof(header1)-1);
  repeat_fasta(alu, n*2);

  write(1, header2, sizeof(header2)-1);
  random_fasta(iub, iub_p, n*3);

  write(1, header3, sizeof(header3)-1);
  random_fasta(homosapiens, homosapiens_p, n*5);

  return 0;
}
