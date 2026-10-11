/* The Computer Language Benchmarks Game
https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

contributed by Seth Heeren
*/
#include <iostream>
#include <string>

using std::string;
using std::cin;
using std::cout;
using std::getline;
using std::endl;

template <typename Ch> inline Ch complement(Ch c)
{
    switch (c)
    {
        // IDEA: Reorder branches after profiling?
        // IDEA: (gcc probably compiles the switch into a jump table)
        case 't': case 'T':             //  7707842
        case 'u': case 'U': return 'A'; //
        case 'a': case 'A': return 'T'; //  7592592
        case 'g': case 'G': return 'C'; //  5552804
        case 'c': case 'C': return 'G'; //  5442702
        case 'v': case 'V': return 'B'; //   205714
        case 's': case 'S': return 'S'; //   200078
        case 'h': case 'H': return 'D'; //   197260
        case 'w': case 'W': return 'W'; //   194442
        case 'r': case 'R': return 'Y'; //   194442
        case 'm': case 'M': return 'K'; //   174716
        case 'y': case 'Y': return 'R'; //   157808
        case 'k': case 'K': return 'M'; //   154990
        case 'b': case 'B': return 'V'; //   146536
        case 'd': case 'D': return 'H'; //   132446
        case 'n': case 'N': return 'N'; //   129628
    }
    throw "parse error"; // TODO proper exception
}

template <typename Out>
inline static void print_reverse(std::string const& sequence, Out& out)
{
    auto const rend = sequence.rend();
    size_t count = 0;
    for (auto i = sequence.rbegin(); i != rend; ++i)
    {
        *out++ = *i; // TODO: buffer writes and append line by line?
        if (0 == ++count % 60)
            *out++ = '\n';
    }
    if (count % 60)
        *out++ = '\n';
}

int main()
{
    string sequence, line;
    sequence.reserve(12000); // arbitrary heuristic preallocation

    cin.unsetf(std::ios::skipws);
    std::cin.tie(nullptr);
    std::cout.tie(nullptr);

    auto out = std::ostreambuf_iterator<char>(std::cout);
    while (getline(cin, line))
    {
        const bool is_header = (line[0] == '>');

        if (is_header)
        {
            if (!sequence.empty())
            {
                for (auto& c : sequence)
                    c = complement(c);
                print_reverse(sequence, out);
            }
            // clear, (retain allocated capacity)
            sequence.resize(0);

            // print header line
            cout << line << endl;
        }
        else
        {
            sequence.append(line);
        }
    }

    if (!sequence.empty())
    {
        for (auto& c : sequence)
            c = complement(c);
        print_reverse(sequence, out);
    }
}
