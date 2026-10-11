/* The Computer Language Benchmarks Game
https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

converted to C++ from D by Rafal Rusin
modified by Vaclav Haisman
modified by The Anh to compile with g++ 4.3.2

compile: g++ -O3 -Wall -march=native -msse2 -o fasta fasta.cpp
*/

#include <stdio.h>
#include <stdlib.h>
#include <algorithm>
#include <memory>
#include <vector>
#include <numeric>
#include <functional>

using namespace std;

static int const IM = 139968, IA = 3877, IC = 29573;
static int last = 42;

static inline
double
genRandom(double max)
{
   return(max * (last = (last * IA + IC) % IM) / IM);
}

struct IUB
{
   char c;
   double p;
};

struct IUB_accum : public std::binary_function<IUB const &, IUB, IUB>
{
   result_type
      operator () (first_argument_type a, second_argument_type b) const
   {
      b.p += a.p;
      return b;
   }
};

template <typename Iter>
static inline
void
makeCumulative(Iter const start, Iter const end)
{
   partial_sum (start, end, start, IUB_accum ());
}

static const char alu[] =
"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
"GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
"CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
"ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
"GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
"AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
"AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

static const unsigned length = 60, alu_size = sizeof(alu) - 1;

static inline
void
makeRepeatFasta(char const * const id, char const * const desc, unsigned n)
{
   printf(">%s %s\n", id, desc);
   char line[length+1];
   unsigned alu_index = 0;
   while(n > 0)
   {
      unsigned const m = min (n, length);
      line[m] = '\n';
      n -= m;
      unsigned line_index = 0;
      while (m - line_index != 0)
      {
         unsigned const copy_count = min (m - line_index, alu_size - alu_index);
         copy (alu + alu_index, alu + alu_index + copy_count, line + line_index);
         alu_index += copy_count;
         line_index += copy_count;
         if (alu_index == alu_size)
            alu_index = 0;
      }
      fwrite_unlocked (line, 1, m + 1, stdout);
   }
}

template <typename Iter>
static inline
void
makeRandomFasta(char const * const id, char const * const desc, unsigned n, Iter start, Iter end)
{
   printf(">%s %s\n", id, desc);
   makeCumulative(start, end);
   char line[length + 1];
   while(n > 0)
   {
      unsigned const m = min (n, length);
      n -= m;
      for(size_t j = 0; j < m; ++j)
      {
         double const rval = genRandom(1);
         Iter it;
         for (it = start; it != end; ++it)
         {
            if (rval < it->p)
               break;
         }
         line[j] = it->c;
      }
      line[m] = '\n';
      fwrite_unlocked (line, 1, m + 1, stdout);
   }
}

static IUB iub[] =
{
   { 'a', 0.27 },
   { 'c', 0.12 },
   { 'g', 0.12 },
   { 't', 0.27 },

   { 'B', 0.02 },
   { 'D', 0.02 },
   { 'H', 0.02 },
   { 'K', 0.02 },
   { 'M', 0.02 },
   { 'N', 0.02 },
   { 'R', 0.02 },
   { 'S', 0.02 },
   { 'V', 0.02 },
   { 'W', 0.02 },
   { 'Y', 0.02 }
};

static IUB homosapiens[] =
{
   { 'a', 0.3029549426680 },
   { 'c', 0.1979883004921 },
   { 'g', 0.1975473066391 },
   { 't', 0.3015094502008 }
};

int main(int argc, char *argv[])
{
   unsigned const n = argc > 1 ? atoi(argv[1]) : 1;

   makeRepeatFasta("ONE", "Homo sapiens alu", n*2);
   makeRandomFasta("TWO", "IUB ambiguity codes", n*3, iub,
      iub + sizeof (iub) / sizeof (IUB));
   makeRandomFasta("THREE", "Homo sapiens frequency", n*5, homosapiens,
      homosapiens + sizeof (homosapiens) / sizeof (IUB));
}
