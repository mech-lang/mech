# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Andrew Rodland

use strict;

sub eval_A {
  use integer;
  my $div = ( ($_[0] + $_[1]) * ($_[0] + $_[1] + 1) / 2) + $_[0] + 1;
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

sub multiplyAtAv {
  return multiplyAtv( multiplyAv( @_ ) );
}

my $n = @ARGV ? shift : 500;
my @u = (1) x $n;
my @v;
for (0 .. 9) {
  @v = multiplyAtAv( @u );
  @u = multiplyAtAv( @v );
}

my ($vBv, $vv);
for my $i (0 .. $#u) {
  $vBv += $u[$i] * $v[$i];
  $vv += $v[$i] ** 2;
}

printf( "%0.9f\n", sqrt( $vBv / $vv ) );
