/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * This benchmark uses ~line_length * sequence_lenth bytes from the
 * random number generator, and approaches the I/O limits of simple
 * write() calls from one thread.  It does this by exploiting the
 * periodicity of the random number generator.  Any language could pay
 * a similar startup cost and then issue these large system write, so
 * it's probably not terribly interesting as a language comparison
 * benchmark.  The calculations are simple, so approaching this number
 * should be a goal for any high performance implementation.  This
 * shows how close each other language and implementation gets to the
 * limit.
 *
 * perf stat
 *    452,094,945      cycles                    #    4.119 GHz
 *    512,589,831      instructions              #    1.13  insn per cycle
 *
 * Nearly all of that is system time.  Writing to /dev/null it almost
 * vanishes.  Parallel writes or making use of filesystem block
 * duplication could potentially let the kernel portion finish faster,
 * but the benchmark is to write to stdout, so this seems done.
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
#define rand_next() ( (seed * IA + IC ) % IM )
#define rand_advance() ( seed = rand_next() )
#define rand_seed(x) seed=x

/* per
 * https://en.wikipedia.org/wiki/Modular_exponentiation#Pseudocode */
uint32_t modular_pow(uint64_t base, uint64_t exponent, const uint32_t modulus) {
  if (modulus == 1) {
    return 0;
  }
  uint64_t result = 1;
  base = base % modulus;
  while (exponent > 0) {
    if ((exponent & 1) == 1) {
      result = (result * base) % modulus;
    }
    exponent = exponent >> 1;
    base = (base * base) % modulus;
  }
  return result;
}


/* per
 * https://www.nayuki.io/page/fast-skipping-in-a-linear-congruential-generator
 */
#define IA1 (IA-1)
#define IMA (IA1 * IM)

static uint32_t rand_skip(int n) {
  uint64_t y = ((modular_pow(IA, n, IMA) - 1) / IA1) * IC;
  uint64_t z = modular_pow(IA, n, IM) * seed;
  seed = (y + z) % IM;
  return seed;
}


#define BUFLINES 100

static const char alu[] =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

static const char iub[] = "acgtBDHKMNRSVWY";
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

static const char homosapiens[] = "acgt";
static const float homosapiens_p[] = {
  0.3029549426680,
  0.1979883004921,
  0.1975473066391,
  0.3015094502008
};

#define LINELEN 60
static int buffer_lines_size(const int seqlen) {
  return (LINELEN+1) * seqlen;
}

static char * build_buffer_lines(const char *seq, const int len) {
  int buflen1 = len + LINELEN;
  char *buffer1 = malloc(buflen1);
  int i;
  int whole_seqs = buflen1/len;
  for (i=0; i < whole_seqs; i++) memcpy(buffer1+i*len, seq, len);
  memcpy(buffer1 + whole_seqs*len, seq, buflen1 - whole_seqs*len);

  int buflen2 = buffer_lines_size(len);
  char *buffer2 = malloc(buflen2);
  for (i=0; i<len; i++) {
    memcpy(buffer2+i*(LINELEN+1), buffer1+((i*LINELEN)%len), LINELEN);
    buffer2[(i+1)*(LINELEN+1)-1] = '\n';
  }

  free(buffer1);
  return buffer2;
}

static void repeat_fasta(const char *seq,
			 const int len,
			 const int n) {
  ssize_t wrote = 0;

  char *buffer2 = build_buffer_lines(seq, len);
  const int buflen2 = buffer_lines_size(len);

  int whole_buffers = n / (len*LINELEN);
  for (int i=0; i< whole_buffers; i++) wrote += write(1, buffer2, buflen2);

  int data_remaining = n - whole_buffers * len * LINELEN;
  int embedded_newlines = data_remaining / LINELEN;
  wrote += write(1, buffer2, data_remaining + embedded_newlines);

  free(buffer2);
  if (n % LINELEN != 0) wrote += write(1, "\n", 1);
}

static char * build_hash(const char *symb,const float *probability) {
  int i,j;
  float sum = 0.0;
  const int len = strlen(symb);

  char *hash = malloc(IM);
  if (!hash) exit(-1);

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

static char * build_periodic_sequence(const char *hash, const int len) {
  int i;
  char *sequence = malloc(len);
  if (!sequence) exit(-1);
  for (i=0; i<len; i++) {
    uint32_t v = rand_advance();
    sequence[i] = hash[v];
  }
  return sequence;
}

static void random_fasta(const char *symb,
			 const float *probability,
			 const int n) {
  char *hash = build_hash(symb,probability);
  if (n>IM) {
    char *sequence = build_periodic_sequence(hash, IM);
    free(hash);
    repeat_fasta(sequence, IM, n);
    free(sequence);
  } else {
    char *sequence = build_periodic_sequence(hash, n);
    free(hash);
    repeat_fasta(sequence, n, n);
    free(sequence);
  }

  /* we might not have pulled enough random numbers to stay in the
   * same sequence as other benchmarks, so catch up */
  int skip = n - IM;
  if (skip > 0) {
    rand_skip(skip);
  } else if (skip < 0) {
    rand_seed(SEED);
    rand_skip(n);
  }
}

const char header1[] = ">ONE Homo sapiens alu\n";
const char header2[] = ">TWO IUB ambiguity codes\n";
const char header3[] = ">THREE Homo sapiens frequency\n";

int main(int argc, char **argv) {
  int n=1000;
  if (argc>1) n = atoi(argv[1]);

  write(1, header1, sizeof(header1)-1);
  repeat_fasta(alu, sizeof(alu)-1, n*2);

  write(1, header2, sizeof(header2)-1);
  random_fasta(iub, iub_p, n*3);
  
  write(1, header3, sizeof(header3)-1);
  random_fasta(homosapiens, homosapiens_p, n*5);

  return 0;
// }  
// not accepted, different algorithm, 0.016 seconds
