module clbg::pidigits;
import libc;
import clbg::gmp;

fn int main(int argc, char** argv)
{
    long n = libc::atoll(argv[1]);
    gmp::Mpz q;
    gmp::Mpz r;
    gmp::Mpz t;
    gmp::Mpz numerator3;
    gmp::Mpz numerator4;
    gmp::Mpz digit3;
    gmp::Mpz digit4;
    gmp::Mpz temporary;
    gmp::init(&q);
    gmp::init(&r);
    gmp::init(&t);
    gmp::init(&numerator3);
    gmp::init(&numerator4);
    gmp::init(&digit3);
    gmp::init(&digit4);
    gmp::init(&temporary);
    gmp::set_ui(&q, 1);
    gmp::set_ui(&t, 1);
    ulong k = 1;
    long printed = 0;
    while (printed < n)
    {
        gmp::mul_ui(&numerator3, &q, 3);
        gmp::add(&numerator3, &numerator3, &r);
        gmp::mul_ui(&numerator4, &q, 4);
        gmp::add(&numerator4, &numerator4, &r);
        gmp::divide(&digit3, &numerator3, &t);
        gmp::divide(&digit4, &numerator4, &t);
        if (gmp::compare(&digit3, &digit4) == 0)
        {
            ulong digit = gmp::get_ui(&digit3);
            libc::putchar((int)(48 + digit));
            printed++;
            if (printed % 10 == 0 || printed == n)
            {
                for (long padding = 0; padding < (10 - printed % 10) % 10; padding++)
                {
                    libc::putchar(32);
                }
                libc::printf("\t:%lld\n", printed);
            }
            gmp::mul_ui(&temporary, &t, digit);
            gmp::sub(&r, &r, &temporary);
            gmp::mul_ui(&r, &r, 10);
            gmp::mul_ui(&q, &q, 10);
        }
        else
        {
            ulong factor = 2 * k + 1;
            gmp::addmul_ui(&r, &q, 2);
            gmp::mul_ui(&r, &r, factor);
            gmp::mul_ui(&q, &q, k);
            gmp::mul_ui(&t, &t, factor);
            k++;
        }
    }
    gmp::clear(&q);
    gmp::clear(&r);
    gmp::clear(&t);
    gmp::clear(&numerator3);
    gmp::clear(&numerator4);
    gmp::clear(&digit3);
    gmp::clear(&digit4);
    gmp::clear(&temporary);
    return 0;
}
