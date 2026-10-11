/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   converted to C++ from D by Rafal Rusin
   modified by Vaclav Haisman
   modified by The Anh to compile with g++ 4.3.2
   modified by Branimir Maksimovic
   modified by Kim Walisch
   modified by Tavis Bohne

   compiles with gcc fasta.cpp -std=c++11 -O2
*/

#include <algorithm>
#include <array>
#include <iostream>
#include <numeric>

struct IUB
{
    float p;
    char c;
};

std::array<char, 288> alu = 
{
    "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"
    "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"
    "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"
    "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"
    "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"
    "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
};

std::array<IUB,15> iub =
{{
    { 0.27f, 'a' },
    { 0.12f, 'c' },
    { 0.12f, 'g' },
    { 0.27f, 't' },
    { 0.02f, 'B' },
    { 0.02f, 'D' },
    { 0.02f, 'H' },
    { 0.02f, 'K' },
    { 0.02f, 'M' },
    { 0.02f, 'N' },
    { 0.02f, 'R' },
    { 0.02f, 'S' },
    { 0.02f, 'V' },
    { 0.02f, 'W' },
    { 0.02f, 'Y' }
}};

std::array<IUB, 4> homosapiens =
{{
    { 0.3029549426680f, 'a' },
    { 0.1979883004921f, 'c' },
    { 0.1975473066391f, 'g' },
    { 0.3015094502008f, 't' }
}};

float gen_random(float max = 1.0f)
{
    static const int IM = 139968, IA = 3877, IC = 29573;
    static int last = 42;
    last = (last * IA + IC) % IM;
    return max * last * (1.0f / IM);
}

template<class iterator_type>
class repeat_functor_type {
public:
    repeat_functor_type(iterator_type first, iterator_type last)
      : first(first), current(first), last(last)
    { }
    char operator()()
    {
        if (current == last)
            current = first;
        iterator_type p = current;
        ++current;
        return *p;
    }
private:
    iterator_type first;
    iterator_type current;
    iterator_type last;
};
template<class iterator_type>
repeat_functor_type<iterator_type> 
    make_repeat_functor(iterator_type first, iterator_type last)
{return repeat_functor_type<iterator_type>(first, last);}

template<class iterator_type>
class random_functor_type {
public:
    random_functor_type(iterator_type first, iterator_type last)
      : first(first), last(last)
    { }
    char operator()()
    {
        const float p = gen_random(1.0f);
        auto result = std::find_if(first, last, [p] (IUB i) { return p <= i.p; });
        return result->c;
    }
private:
    iterator_type first;
    iterator_type last;
};
template<class iterator_type>
random_functor_type<iterator_type> 
    make_random_functor(iterator_type first, iterator_type last)
{return random_functor_type<iterator_type>(first, last);}

template<class iterator_type>
void make_cumulative(iterator_type first, iterator_type last)
{
    std::partial_sum(first, last, first,
        [] (IUB l, IUB r) -> IUB { r.p += l.p; return r; });
}

template <class F>
void make(const char* desc, int n, F functor)
{
    std::cout << '>' << desc << '\n';
    const int MAXLINE = 60;
    char line[MAXLINE + 1];
    while (n > 0)
    {
        int thisline = n;
        if (thisline > MAXLINE) 
            thisline = MAXLINE;

        for(int i=0; i<thisline; ++i)
            line[i] = functor();

        line[thisline] = '\n';
        std::cout.write(line, thisline+1);
        n -= thisline;
    }
}

int main(int argc, char *argv[])
{
    int n = 1000;
    if (argc < 2 || (n = std::atoi(argv[1])) <= 0) {
        std::cerr << "usage: " << argv[0] << " length\n";
        return 1;
    }

    std::cout.sync_with_stdio(false);
    make_cumulative(iub.begin(), iub.end());
    make_cumulative(homosapiens.begin(), homosapiens.end());

    //alu must drop the trailing zero stuck on by string literals :(
    make("ONE Homo sapiens alu"      , n * 2, 
        make_repeat_functor(alu.begin(), alu.end()-1)); 
    make("TWO IUB ambiguity codes"   , n * 3, 
        make_random_functor(iub.begin(), iub.end()));
    make("THREE Homo sapiens frequency", n * 5, 
        make_random_functor(homosapiens.begin(), homosapiens.end()));
    return 0;
}
