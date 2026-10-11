/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   converted to C++ from D by Rafal Rusin
   modified by Vaclav Haisman
   modified by The Anh to compile with g++ 4.3.2
   modified by Branimir Maksimovic
   modified by Kim Walisch
*/

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <algorithm>
#include <vector>
#include <numeric>

namespace {

const char alu[] =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

const int LENGTH = 60;

struct IUB
{
  float p;
  char c;
};

std::vector<IUB> iub =
{
  { 0.27, 'a' },
  { 0.12, 'c' },
  { 0.12, 'g' },
  { 0.27, 't' },
  { 0.02, 'B' },
  { 0.02, 'D' },
  { 0.02, 'H' },
  { 0.02, 'K' },
  { 0.02, 'M' },
  { 0.02, 'N' },
  { 0.02, 'R' },
  { 0.02, 'S' },
  { 0.02, 'V' },
  { 0.02, 'W' },
  { 0.02, 'Y' }
};

std::vector<IUB> homosapiens =
{
  { 0.3029549426680, 'a' },
  { 0.1979883004921, 'c' },
  { 0.1975473066391, 'g' },
  { 0.3015094502008, 't' }
};

inline float gen_random(float max = 1.0f)
{
  static const int IM = 139968, IA = 3877, IC = 29573;
  static int last = 42;
  last = (last * IA + IC) % IM;
  return max * last * (1.0f / IM);
}

class Repeat {
public:
  Repeat(const char* alu)
    : alu(alu), size(std::strlen(alu)), i(0)
  { }
  char operator()()
  {
    if (i >= size)
      i = 0;
    return alu[i++];
  }
private:
  const char* alu;
  const std::size_t size;
  std::size_t i;
};

class Random {
public:
  Random(const std::vector<IUB>& i)
    : i(i)
  { }
  char operator()()
  {
    const float p = gen_random(1.0f);
    const std::size_t count = std::count_if(i.begin(), i.end(),
        [p] (IUB i) { return p >= i.p; });
    return i[count].c;
  }
private:
  const std::vector<IUB>& i;
};

void make_cumulative(std::vector<IUB>& i)
{
  std::partial_sum(i.begin(), i.end(), i.begin(),
      [] (IUB l, IUB r) -> IUB { r.p += l.p; return r; });
}

template <class F>
void make(const char* id, const char* desc, int n, F functor)
{
  std::printf(">%s %s\n", id, desc);
  char line[LENGTH + 1] = { 0 };
  int i = 0;
  while (n-- > 0)
  {
    line[i++] = functor();
    if (i >= LENGTH)
    {
      std::puts(line);
      i = 0;
    }
  }
  line[i] = 0;
  if (std::strlen(line) != 0)
    std::puts(line);
}

} // end namespace

int main(int argc, char *argv[])
{
   const int n = argc > 1 ? atoi(argv[1]) : 1;

   make_cumulative(iub);
   make_cumulative(homosapiens);

   make("ONE"  , "Homo sapiens alu"      , n * 2, Repeat(alu));
   make("TWO"  , "IUB ambiguity codes"   , n * 3, Random(iub));
   make("THREE", "Homo sapiens frequency", n * 5, Random(homosapiens));
}
