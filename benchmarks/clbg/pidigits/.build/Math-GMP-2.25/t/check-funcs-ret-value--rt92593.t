#!/usr/bin/perl

use strict;
use warnings;

# See:
# https://rt.cpan.org/Ticket/Display.html?id=92593

use Test::More tests => 47;

use Math::GMP;

{
    my $x = Math::GMP->new(5);
    my $val = $x->bfac();      # 1*2*3*4*5 = 120

    # TEST
    is ($x.'', "5", "x->bfac did not change x");

    # TEST
    is ($val.'', '120', 'val=x->bfac is correct.');
}

{
    my $x = Math::GMP->new(0b1100);
    my $ret = $x->band(0b1010, 0);

    # TEST
    is ($x.'', 0b1100, "x->band did not change");

    # TEST
    is ($ret.'', 0b1000, "ret = x->band is correct.");
}

{
    my $x = Math::GMP->new(0b1100);
    my $ret = $x->bxor(0b1010, 0);

    # TEST
    is ($x.'', 0b1100, "x did not change after x->bxor");

    # TEST
    is ($ret.'', 0b110, "ret = x->bxor is correct.");
}

{
    my $x = Math::GMP->new(0b1100);
    my $ret = $x->bior(0b1010, 0);

    # TEST
    is ($x.'', 0b1100, "x did not change after x->bior");

    # TEST
    is ($ret.'', 0b1110, "ret = x->bior is correct.");
}

{
    my $x = Math::GMP->new(1000 * 3);
    my $ret = $x->bgcd(1000 * 7);

    # TEST
    is ($x.'', 1000 * 3, "x did not change after x->bgcd");

    # TEST
    is ($ret.'', 1000, "ret = x->bgcd(y) is correct.");
}

{
    my $x = Math::GMP->new(1000 * 3 * 3);
    my $ret = $x->blcm(1000 * 3 * 7);

    # TEST
    is ($x.'', 1000 * 3 * 3, "x did not change after x->blcm");

    # TEST
    is ($ret.'', 1000 * 3 * 3 * 7, "ret = x->blcm(y) is correct.");
}

{
    my $x = Math::GMP->new(5);
    my $ret = $x->bmodinv(7);

    # TEST
    is ($x.'', 5, "x did not change after x->bmodinv");

    # TEST
    is ($ret.'', 3, "ret = x->bmodinv(y) is correct.");
}

{
    my $x = Math::GMP->new(6);
    my $ret = $x->bsqrt();

    # TEST
    is ($x.'', 6, "x did not change after x->bsqrt");

    # TEST
    is ($ret.'', 2, "ret = x->bsqrt() is correct.");
}

{
    my $x = Math::GMP->new(200);
    my $ret = $x->legendre(3);

    # TEST
    is ($x.'', 200, "x did not change after x->legendre");

    # TEST
    is ($ret, -1, "ret = x->legendre(y) is correct.");
}

{
    my $x = Math::GMP->new(200);
    my $ret = $x->jacobi(5);

    # TEST
    is ($x.'', 200, "x did not change after x->jacobi");

    # TEST
    is ($ret, 0, "ret = x->jacobi(y) is correct.");
}

{
    my $x = Math::GMP::fibonacci(200);

    # TEST
    is ($x.'', '280571172992510140037611932413038677189525', "Math::GMP::fibonacci() works fine");
}

{
    my $x = Math::GMP->new(7);
    my $is_prime_verdict = $x->probab_prime(10);

    # TEST
    is ($x.'', '7', "x did not change after x->probab_prime");

    # TEST
    is ($is_prime_verdict, '2', 'probab_prime works.');
}

{
    my $x = Math::GMP->new('1'. ('0' x 100));
    $x->add_ui_gmp(500);

    # TEST
    is ($x.'', '1' . ('0' x (100-3)) . '500', "x was mutated after add_ui_gmp");
}

{
    my $x = Math::GMP->new(7);
    my ($quo, $rem) = $x->bdiv(3);

    # TEST
    is ($x.'', 7, "x did not change after x->bdiv");

    # TEST
    is ($quo.'', 2, "x->bdiv[quo]");

    # TEST
    is ($rem.'', 1, "x->bdiv[rem]");
}

{
    my $x = Math::GMP->new(200);
    my $ret = $x->div_2exp_gmp(2);

    # TEST
    is ($x.'', 200, "x did not change after x->div_2exp_gmp");

    # TEST
    is ($ret.'', 50, "ret = x->div_2exp_gmp(y) is correct.");
}

{
    my $init_n = 3 * 7 + 2 * 7 * 7 + 6 * 7 * 7 * 7;
    my $x = Math::GMP->new($init_n);
    my $ret = $x->get_str_gmp(7);

    # TEST
    is ($x.'', $init_n, "x did not change after x->get_str_gmp");

    # TEST
    is ($ret, "6230", "ret = x->get_str_gmp(base) is correct.");
}

{
    my $x = Math::GMP->new('2' . ('123' x 100));
    my $y = $x->gmp_copy;

    # TEST
    is ($x.'', '2'. ('123' x 100), "x did not change after x->gmp_copy");

    # TEST
    is ($y.'', '2'. ('123' x 100), "->gmp_copy returned a clone.");

    $y += 1;

    # TEST
    is ($x.'', '2'. ('123' x 100), "x did not change after x->gmp_copy+modify");

    # TEST
    is ($y.'', '2'. ('123' x 99) . '124', "y changed.");
}

{
    my $x = Math::GMP->new(0b1000100);

    # TEST
    is (scalar($x->gmp_tstbit(6)), 1, "gmp_tstbit #1");

    # TEST
    is (scalar($x->gmp_tstbit(4)), 0, "gmp_tstbit #2");
}

{
    my $x = (Math::GMP->new(24) * 5);

    my $ret = $x->intify;

    # TEST
    is ($ret, 120, "test intify");
}

{
    my $x = Math::GMP->new(2 . ('0' x 200) . 4);
    my $y = Math::GMP->new(5);

    my $ret = $x->mmod_gmp($y);

    # TEST
    is ($ret.'', 4, "mmod_gmp");

    # TEST
    is ($x.'', '2' . ('0' x 200) . '4', "mmod_gmp did not change first arg");
}

{
    my $x = Math::GMP->new(0b10001011);
    my $ret = $x->mod_2exp_gmp(4);

    # TEST
    is ($x.'', 0b10001011, "x did not change after x->mod_2exp_gmp");

    # TEST
    is ($ret.'', 0b1011, "ret = x->mod_2exp_gmp(y) is correct.");
}

{
    my $x = Math::GMP->new(0b10001011);
    my $ret = $x->mul_2exp_gmp(4);

    # TEST
    is ($x.'', 0b10001011, "x did not change after x->mul_2exp_gmp");

    # TEST
    is ($ret.'', 0b100010110000, "ret = x->mul_2exp_gmp(y) is correct.");
}

{
    my $x = Math::GMP->new(157);
    my $exp = Math::GMP->new(100);
    my $mod = Math::GMP->new(5013);

    my $ret = $x->powm_gmp($exp, $mod);
    my $brute_force_ret = (($x ** $exp) % $mod);

    # TEST
    is ($x.'', 157, "x did not change after x->powm_gmp");

    # TEST
    is ($ret.'', $brute_force_ret.'',
        "ret = x->powm_gmp(exp, mod) is correct."
    );
}

{
    my $x = Math::GMP->new('2' . ('123' x 100));

    # TEST
    is ($x->sizeinbase_gmp(10), 1 + 3 * 100, "sizeinbase_gmp works");
}
