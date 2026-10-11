# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Andrew Rodland

use strict;
use IO::Select;

our ($n, $size_of_float, $threads, @ranges, $begin, $end);

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
  } $begin .. $end;
}

sub multiplyAtv {
  return map {
    my ($i, $sum) = ($_);
    $sum += eval_A($_, $i) * $_[$_] for 0 .. $#_;
    $sum;
  } $begin .. $end;
}

sub do_parallel {
  my $func = shift;

  my @out;
  my (@fd, @ptr, %fh2proc);
  for my $proc (0 .. $threads - 1) {
    ($begin, $end) = @{ $ranges[$proc] };
    my $pid = open $fd[$proc], "-|";
    if ($pid == 0) {
      print pack "F*", $func->( @_ );
      exit 0;
    } else {
      $fh2proc{ $fd[$proc] } = $proc;
      $ptr[$proc] = $begin;
    }
  }

  my $select = IO::Select->new(@fd);

  while ($select->count) {
    for my $fh ($select->can_read) {
      my $proc = $fh2proc{$fh};
      while (read $fh, my $data, $size_of_float) {
        $out[ $ptr[$proc] ++ ] = unpack "F", $data;
      }
      $select->remove($fh) if eof($fh);
    }
  }

  return @out;
}

sub multiplyAtAv {
  my @array = do_parallel(\&multiplyAv, @_);
  return do_parallel(\&multiplyAtv, @array);
}

sub num_cpus {
  open my $fh, '</proc/cpuinfo' or return;
  my $cpus;
  while (<$fh>) {
    $cpus ++ if /^processor\s+:/;
  }
  return $cpus;
}

sub init {
  $size_of_float = length pack "F", 0;

  $n = @ARGV ? $ARGV[0] : 500;
  $threads = num_cpus() || 1;

  if ($threads > $n) {
    $threads = $n;
  }

  for my $i (0 .. $threads - 1) {
    use integer;
    $ranges[$i][0] = $n * $i / $threads;
    $ranges[$i][1] = $n * ($i + 1) / $threads - 1;
    no integer;
  }
}

init();

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
