// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Elam Kolenovic
//
// Changes (2013-04-07):
//   - removed unnecessary arrays, faster especially on 32 bits
//   - using putchar instead of iostreams, slightly faster
//   - using namespace std for readability
//   - replaced size_t with unsigned
//   - removed some includes

#include <cstdio>
#include <cstdlib>
#include <limits>
#include <vector>

typedef unsigned char Byte;

using namespace std;

int main(int argc, char* argv[])
{
    const unsigned N             = max(0, (argc > 1) ? atoi(argv[1]) : 0);
    const unsigned width         = N;
    const unsigned height        = N;
    const unsigned maxX          = (width + 7) / 8;
    const unsigned maxIterations = 50;
    const double   limit         = 2.0;
    const double   limitSq       = limit * limit;

    vector<Byte> data(height * maxX);

    printf("P4\n%u %u\n", width, height);

    for (unsigned y = 0; y < height; ++y)
    {
        const double ci0 = 2.0 * y / height - 1.0;

        for (unsigned x = 0; x < maxX; ++x)
        {
            double cr0[8];
            for (unsigned k = 0; k < 8; ++k)
            {
                cr0[k] = 2.0 * (8 * x + k) / width - 1.5;
            }

            double cr[8];
            copy(cr0, cr0 + 8, &cr[0]);

            double ci[8];
            fill(ci, ci + 8, ci0);

            Byte bits = 0;
            for (unsigned i = 0; i < maxIterations && bits != 0xFF; ++i)
            {
                for (unsigned k = 0; k < 8; ++k)
                {
                    const Byte mask = (1 << (7 - k));
                    if ((bits & mask) == 0)
                    {
                        const double crk  = cr[k];
                        const double cik  = ci[k];
                        const double cr2k = crk * crk;
                        const double ci2k = cik * cik;

                        cr[k] = cr2k - ci2k + cr0[k];
                        ci[k] = 2.0 * crk * cik + ci0;

                        if (cr2k + ci2k > limitSq)
                        {
                            bits |= mask;
                        }
                    }
                }
            }
            putchar(~bits);
        }
    }

    return 0;
}
