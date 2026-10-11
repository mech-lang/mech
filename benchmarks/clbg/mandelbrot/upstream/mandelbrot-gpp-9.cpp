// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Elam Kolenovic
//
// Changes (2013-05-07)
//   - changed omp schedule for more even distribution of work
//   - changed loop variables to signed integer because msvc was complaining
//     when omp was enabled
//   - replaced std::copy and std::fill by one loop. slightly faster.
//   - swapped order of tests in for-i-loop. slightly faster.
//
// Changes (2013-04-19)
//   - using omp
//   - use buffer and fwrite at end instead of putchar
//   - pre-calculate cr0[]
//   - rename variables and use underscore before the index part of the name
//   - inverted bit tests, better performance under MSVC
//   - optional argument for file output, usefull in windows shell
//
// Changes (2013-04-07):
//   - removed unnecessary arrays, faster especially on 32 bits
//   - using putchar instead of iostreams, slightly faster
//   - using namespace std for readability
//   - replaced size_t with unsigned
//   - removed some includes

#include <cstdio>
#include <cstdlib>
#include <vector>

typedef unsigned char Byte;

using namespace std;

int main(int argc, char* argv[])
{
    const int    N              = max(0, (argc > 1) ? atoi(argv[1]) : 0);
    const int    width          = N;
    const int    height         = N;
    const int    max_x          = (width + 7) / 8;
    const int    max_iterations = 50;
    const double limit          = 2.0;
    const double limit_sq       = limit * limit;

    vector<Byte> buffer(height * max_x);

    vector<double> cr0(8 * max_x);
#pragma omp parallel for
    for (int x = 0; x < max_x; ++x)
    {
        for (int k = 0; k < 8; ++k)
        {
            const int xk = 8 * x + k;
            cr0[xk] = (2.0 * xk) / width - 1.5;
        }
    }

#pragma omp parallel for schedule(guided)
    for (int y = 0; y < height; ++y)
    {
        Byte* line = &buffer[y * max_x];

        const double ci0 = 2.0 * y / height - 1.0;

        for (int x = 0; x < max_x; ++x)
        {
            const double* cr0_x = &cr0[8 * x];
            double cr[8];
            double ci[8];
            for (int k = 0; k < 8; ++k)
            {
                cr[k] = cr0_x[k];
                ci[k] = ci0;
            }

            Byte bits = 0xFF;
            for (int i = 0; bits && i < max_iterations; ++i)
            {
                Byte bit_k = 0x80;
                for (int k = 0; k < 8; ++k)
                {
                    if (bits & bit_k)
                    {
                        const double cr_k    = cr[k];
                        const double ci_k    = ci[k];
                        const double cr_k_sq = cr_k * cr_k;
                        const double ci_k_sq = ci_k * ci_k;

                        cr[k] = cr_k_sq - ci_k_sq + cr0_x[k];
                        ci[k] = 2.0 * cr_k * ci_k + ci0;

                        if (cr_k_sq + ci_k_sq > limit_sq)
                        {
                            bits ^= bit_k;
                        }
                    }
                    bit_k >>= 1;
                }
            }
            line[x] = bits;
        }
    }

    FILE* out = (argc == 3) ? fopen(argv[2], "wb") : stdout;
    fprintf(out, "P4\n%u %u\n", width, height);
    fwrite(&buffer[0], buffer.size(), 1, out);

    if (out != stdout)
    {
        fclose(out);
    }

    return 0;
}
