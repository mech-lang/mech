# The Computer Language Benchmarks Game
#  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#  contributed by Mykola Zubach

use strict;
use integer;
use threads;

my($sequence, $seq_len, $thr, @threads, %h);

sub count_frame {
   my $len = shift;
   for (0 .. $seq_len - $len) {
      $h{substr($sequence, $_, $len)} ++;
   }
}

sub count_len {
   my $len = shift;
   my $out;
   count_frame($len);

   no integer;
   if(@_) {
      $out = join '', map sprintf("%d\t%s\n", $h{$_}, $_), @_;
   } else {
      my $sum = $seq_len - $len + 1;
      for (sort { $h{$b} <=> $h{$a} || $a cmp $b } keys %h) {
         $out .= sprintf "%s %.3f\n", $_, $h{$_} * 100 / $sum;
      }
      $out .= "\n";
   }
   return $out;
}

## MAIN()

while(<STDIN>) {
   last if /^>THREE /;
}
while(<STDIN>) {
   last if /^>/;
   chomp;
   $sequence .= uc;
}
$seq_len = length($sequence);

for (1,2) {
   push @threads, threads->create(\&count_len, $_);
}

for (qw(GGT GGTA GGTATT GGTATTTTAATT GGTATTTTAATTTATAGT)) {
   push @threads, threads->create(\&count_len, length, $_);
}

foreach $thr (@threads) {
   print $thr->join();
}
