/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by idzeta
*/


#define BOOST_DISABLE_THREADS 1
#include <future>
#include <re2/re2.h>
#include <boost/xpressive/xpressive.hpp>
#include <cassert>
#include <iostream>
#include <vector>

using namespace re2;
using namespace boost::xpressive;
using namespace std;
namespace rc = regex_constants;

int main()
{
    const string pattern1[] = {
        "agggtaaa|tttaccct",
        "[cgt]gggtaaa|tttaccc[acg]",
        "a[act]ggtaaa|tttacc[agt]t",
        "ag[act]gtaaa|tttac[agt]ct",
        "agg[act]taaa|ttta[agt]cct",
        "aggg[acg]aaa|ttt[cgt]ccct",
        "agggt[cgt]aa|tt[acg]accct",
        "agggta[cgt]a|t[acg]taccct",
        "agggtaa[cgt]|[acg]ttaccct"
    };

    const string pattern2[][2] = {
        "tHa[Nt]", "<4>",
        "aND|caN|Ha[DS]|WaS", "<3>",
        "a[NSt]|BY", "<2>",
        "<[^>]*>", "|",
        "\\|[^|][^|]*\\|", "-"
    };

    cout.sync_with_stdio(false);

    cin.seekg(0, ios_base::end);
    size_t read_size = cin.tellg();
    assert(read_size > 0);
    cin.seekg(0, ios_base::beg);

    string str(read_size, '\0');
    cin.read(&str[0], read_size);
    size_t len1 = cin.gcount();
    assert(len1);
    if (len1 < read_size) {
        str.resize(len1);
    }

    str = regex_replace(str, sregex::compile(">[^\n]*\n|\n"s, rc::optimize), "");
    size_t len2 = str.length();

    auto handle = async(launch::async, [&, out{str}]() mutable {
        for (auto *pattern : pattern2) {
            out = regex_replace(out, sregex::compile(pattern[0], rc::optimize), pattern[1]);
        }
        return out.length();
    });

    vector<future<int>> tasks;
    for (auto &&pattern : pattern1) {
        auto f = [&, count{0}, piece{StringPiece{str}}]() mutable {
            RE2 pat{pattern};
            while (RE2::FindAndConsume(&piece, pat)) {
                ++count;
            }
            return count;
        };
        tasks.push_back(async(launch::async, f));
    }

    for (size_t i = 0; i < tasks.size(); ++i) {
        cout << pattern1[i] << " ";
        cout << tasks[i].get() << endl;
    }

    cout << "\n" << len1 << "\n" << len2 << "\n";
    cout << handle.get() << endl;
}
