# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by A. Sinan Unur

use strict;

my @variants = qw/
   agggtaaa|tttaccct
   [cgt]gggtaaa|tttaccc[acg]
   a[act]ggtaaa|tttacc[agt]t
   ag[act]gtaaa|tttac[agt]ct
   agg[act]taaa|ttta[agt]cct
   aggg[acg]aaa|ttt[cgt]ccct
   agggt[cgt]aa|tt[acg]accct
   agggta[cgt]a|t[acg]taccct
   agggtaa[cgt]|[acg]ttaccct
/;

my @variants_re = map qr/$_/xiaa, @variants;

my @iub = map { my $x = $_; sub { $_[0] =~ s/$x->[0]/$x->[1]/g }} (
    [ qr{ tHa [Nt] }x,                 '<4>' ],
    [ qr{ aND | caN | Ha[DS] | WaS }x, '<3>' ],
    [ qr{ a [NSt] | BY }x,             '<2>' ],
    [ qr{ < [^>]* > }x,                '|'   ],
    [ qr{ \| [^|] [^|]* \| }x,         '-'   ],
);

my $seq = do { local $/; <STDIN> };

my $input_length = length( $seq );

$seq =~ s/>.*\n|\n//g;

my $cleaned_length = length( $seq );

my @results = map scalar( () = $seq =~ /$_/g ), @variants;

$_->($seq) for @iub;

# report

print "$variants[$_] $results[$_]\n" for 0 .. $#variants;
print "$_\n" for '', $input_length, $cleaned_length, length( $seq );
