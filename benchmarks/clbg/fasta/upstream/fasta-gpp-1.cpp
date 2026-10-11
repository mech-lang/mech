/* The Computer Language Benchmarks Game
https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

converted to C++ from D by Rafal Rusin
modified by Vaclav Haisman
modified by The Anh to compile with g++ 4.3.2
modified by Branimir Maksimovic

*/

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <algorithm>
#include <vector>
#include <numeric>

static int const IM = 139968, IA = 3877, IC = 29573;
static int last = 42;

static inline
float
genRandom(float max)
{
   return(max * (last = (last * IA + IC) % IM) / IM);
}

struct IUB
{
   char c;
   float p;
};

static inline
void
makeCumulative(std::vector<IUB>& i)
{
   std::partial_sum (i.begin(), i.end(), i.begin(), 
   [](IUB l,IUB r)->IUB{r.p+=l.p;return r;});
}

static const char alu[] =
"GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
"GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
"CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
"ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
"GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
"AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
"AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

static const unsigned length = 60;

template <class F>
static inline
void
make(char const * id, char const * desc, unsigned n, F f)
{
   printf(">%s %s\n", id, desc);
   char line[length+1]={0};
   unsigned i = 0;
   while(n-- > 0)
   {
      line[i++]=f();
      if(i >= length)
      {
         puts(line);
         i = 0;
      }
   }
   line[i] = 0;
   if(strlen(line))puts(line);
}

struct Repeat{
   Repeat(const char* alu):i(0),size(strlen(alu)),alu(alu){}
   char operator()()
   { 
      if(i >= size)i=0;
      return alu[i++];
   }
   unsigned i,size;
   const char* alu;
};

struct Random{
   Random(std::vector<IUB>& i):i(i){}
   char operator()()
   {
      float p = genRandom(1.0);
      return i[std::count_if(i.begin(),i.end(),
            [p](IUB i){ return p>=i.p;})].c;
   }
   std::vector<IUB>& i;
};

static std::vector<IUB> iub =
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

static std::vector<IUB> homosapiens =
{
   { 'a', 0.3029549426680 },
   { 'c', 0.1979883004921 },
   { 'g', 0.1975473066391 },
   { 't', 0.3015094502008 }
};

int main(int argc, char *argv[])
{
   unsigned const n = argc > 1 ? atoi(argv[1]) : 1;

   makeCumulative(iub);
   makeCumulative(homosapiens);
   
   make("ONE", "Homo sapiens alu", n*2,Repeat(alu));
   make("TWO", "IUB ambiguity codes", n*3, Random(iub));
   make("THREE", "Homo sapiens frequency", n*5, Random(homosapiens));
}
