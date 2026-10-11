/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Contributed by Michael Ganss
 * derived from PHP version that was
 * contributed by Oleksii Prudkyi
 * port from pidigits.lua-5.lua (Mike Pall, Wim Couwenberg)
 * modified by Craig Russell
 * 
 * Original C version by Mr Ledrug
*/

#include <stdio.h>
#include <stdlib.h>
#include <gmp.h>

mpz_t n1, n2, d, u, v, w;

int main(int argc, char **argv)
{
    int k = 1, k2, i = 0;
    int n = atoi(argv[1]);

    mpz_init(u);
    mpz_init(v);

    mpz_init_set_si(w, 0);
    mpz_init_set_si(n1, 4);
    mpz_init_set_si(n2, 3);
    mpz_init_set_si(d, 1);

    for (;;)
    {
        mpz_tdiv_q(u, n1, d);
        mpz_tdiv_q(v, n2, d);

        if (mpz_cmp(u, v) == 0)
        {
            putchar('0' + mpz_get_si(u));
            i++;
            if (i % 10 == 0)
                printf("\t:%d\n", i);
            if (i == n)
                break;

            // extract
            mpz_mul_si(u, u, -10);
            mpz_mul(u, d, u);
            mpz_mul_si(n1, n1, 10);
            mpz_add(n1, n1, u);
            mpz_mul_si(n2, n2, 10);
            mpz_add(n2, n2, u);
        }
        else 
        {
            // produce
            k2 = k * 2;
            mpz_mul_si(u, n1, k2 - 1);
            mpz_add(v, n2, n2);
            mpz_mul_si(w, n1, k - 1);
            mpz_add(n1, u, v);
            mpz_mul_si(u, n2, k + 2);
            mpz_add(n2, w, u);
            mpz_mul_si(d, d, k2 + 1);
            k++;
        }
    }

    if (i % 10 != 0)
        printf("%*s\t:%d\n", 10 - n % 10, "", n);
    return 0;
}
