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
 *
 * ported to Java by Piotr Tarsa
 */

import jextract_gmp.gmp_h;

import java.lang.foreign.Arena;
import java.lang.foreign.MemoryLayout;
import java.lang.foreign.MemorySegment;

public class pidigits {
    private static final Arena GLOBAL_ARENA = Arena.global();

    // Manually written as jextract didn't extract it.
    // Names and types taken from gmp.h
    private static final MemoryLayout mpz_t = MemoryLayout.structLayout(
            gmp_h.C_INT.withName("_mp_alloc"),
            gmp_h.C_INT.withName("_mp_size"),
            gmp_h.C_POINTER.withName("_mp_d")
    ).withName("__mpz_struct");

    @SuppressWarnings("SameParameterValue")
    private static MemorySegment alloc(MemoryLayout typeDescription) {
        return GLOBAL_ARENA.allocate(typeDescription);
    }

    public static void main(String[] args) {
        MemorySegment n1 = alloc(mpz_t),
                n2 = alloc(mpz_t),
                d = alloc(mpz_t),
                u = alloc(mpz_t),
                v = alloc(mpz_t),
                w = alloc(mpz_t);
        int k = 1, k2, i = 0;
        var n = Integer.parseInt(args[0]);

        gmp_h.__gmpz_init(u);
        gmp_h.__gmpz_init(v);

        gmp_h.__gmpz_init_set_si(w, 0);
        gmp_h.__gmpz_init_set_si(n1, 4);
        gmp_h.__gmpz_init_set_si(n2, 3);
        gmp_h.__gmpz_init_set_si(d, 1);

        var outBuf = new StringBuilder();

        for (; ; ) {
            gmp_h.__gmpz_tdiv_q(u, n1, d);
            gmp_h.__gmpz_tdiv_q(v, n2, d);

            if (gmp_h.__gmpz_cmp(u, v) == 0) {
                outBuf.append(gmp_h.__gmpz_get_si(u));
                i++;
                if (i % 10 == 0)
                    outBuf.append("\t:").append(i).append("\n");
                if (i == n)
                    break;

                // extract
                gmp_h.__gmpz_mul_si(u, u, -10);
                gmp_h.__gmpz_mul(u, d, u);
                gmp_h.__gmpz_mul_si(n1, n1, 10);
                gmp_h.__gmpz_add(n1, n1, u);
                gmp_h.__gmpz_mul_si(n2, n2, 10);
                gmp_h.__gmpz_add(n2, n2, u);
            } else {
                // produce
                k2 = k * 2;
                gmp_h.__gmpz_mul_si(u, n1, k2 - 1);
                gmp_h.__gmpz_add(v, n2, n2);
                gmp_h.__gmpz_mul_si(w, n1, k - 1);
                gmp_h.__gmpz_add(n1, u, v);
                gmp_h.__gmpz_mul_si(u, n2, k + 2);
                gmp_h.__gmpz_add(n2, w, u);
                gmp_h.__gmpz_mul_si(d, d, k2 + 1);
                k++;
            }
        }

        if (i % 10 != 0)
            outBuf.append(" ".repeat(10 - n % 10)).append("\t:").append(n).append("\n");

        System.out.print(outBuf);
    }
}
