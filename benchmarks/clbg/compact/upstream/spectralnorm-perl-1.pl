# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Andrew Rodland
# modified by R. Jelinek

use strict;

my $n = @ARGV ? shift : 500;
my @v = multiplyAtAv(
    multiplyAtAv(
        multiplyAtAv(
            (1) x $n
        )
    )
    );

my @u = multiplyAtAv( @v );

my ($vBv, $vv);
my $i = 0;
for my $v (@v) {
    $vBv += $u[$i++] * $v;
    $vv += $v ** 2;
}

printf( "%0.9f\n", sqrt( $vBv / $vv ) );

sub multiplyAtAv {
    return multiplyAtv( multiplyAv( @_ ) );
}

sub eval_A {
    use integer;
    my $sum = $_[0] + $_[1];
    my $div = ($sum**2 + $sum) / 2 + $_[0] + 1;
    no integer;
    1 / $div;
}

sub multiplyAv {
    return map {
        my ($i, $sum) = ($_);
        $sum += eval_A($i, $_) * $_[$_] for 0 .. $#_;
        $sum;
    } 0 .. $#_;
}

sub multiplyAtv {
    return map {
        my ($i, $sum) = ($_);
        $sum += eval_A($_, $i) * $_[$_] for 0 .. $#_;
        $sum;
    } 0 .. $#_;
}
