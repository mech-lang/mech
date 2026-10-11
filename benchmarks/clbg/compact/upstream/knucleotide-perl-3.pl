# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#  contributed by Mykola Zubach

use strict;
use threads;

my($sequence, $seq_len, $thr, @threads, %h);

sub read_sequence($) {
  my $id = shift;
  my $out;

  while(<STDIN>) {
    last if /^>$id /;
  }

  while(<STDIN>) {
    last if /^>/;
    chomp;
    $out .= uc($_);
  }

  return $out;
}

sub count_freq($) {
  my $len = shift;
  my $end = $seq_len - $len;

  %h = ();
  for(my $i = 0; $i <= $end; $i++) {
    $h{substr($sequence, $i, $len)} ++;
  }
}

sub count_len($) {
  my $len = shift;
  my $sum = $seq_len - $len + 1;
  my $out = '';

  count_freq($len);

  foreach my $s  (sort { $h{$b} <=> $h{$a} || $a cmp $b } keys %h) {
    $out .= sprintf "$s %.3f\n", $h{$s} * 100 / $sum;
  }
  $out .= "\n";

  return $out;
}

sub count_sequence($) {
  my $s = shift;

# count ALL sequences of specified length!
  count_freq(length($s));

  return sprintf("%d\t$s\n", $h{$s});
}

## MAIN()

$sequence = read_sequence('THREE');
$seq_len = length($sequence);

foreach my $len (1,2) {
  push @threads, threads->create(\&count_len, $len);
}

foreach my $s (qw(GGT GGTA GGTATT GGTATTTTAATT GGTATTTTAATTTATAGT)) {
  push @threads, threads->create(\&count_sequence, $s);
}

foreach $thr (@threads) {
  print $thr->join();
}
