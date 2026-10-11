# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by A. Sinan Unur
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

my %results;

# perl is not frequently compiled with threads support on *nix machines
# so I can't rely on them. Splitting the work in two should provide some
# performance boost, especially on those systems where fork tends to be
# the cheapest (relatively speaking)
#
# In my testing, this version completed about 25% faster which means the
# overhead is still significant. Adding another child did not improve
# execution time.

pipe(my $reader, my $writer) or die "Cannot set up pipe: $!";

my $pid = fork;

if ( $pid ) {
    close $writer
        or die "Cannot close child's writer in parent: $!";

    # do our own computations, we'll give the child more work
    for ((1 + @variants/2) .. $#variants) {
        $results{ $variants[$_] } = () = $seq =~ /$variants_re[$_]/g;
    }

    # do the substitutions in the parent
    # we gave the child more work

    $_->($seq) for @iub;

    # collect results from child
    while ( <$reader> ) {
        chomp;
        my ($v, $n) = split /\t/;
        $results{ $v } = $n;
    }
    close $reader
        or die "Cannot close parent's reader in parent: $!";

    waitpid($pid, 0);
}
else {
    defined($pid) or die "Failed to fork: $!";
    close $reader
        or die "Cannot close parent's reader in child: $!";
    for (0 .. @variants/2) {
        printf $writer "%s\t%d\n", $variants[$_], 
            scalar( () = $seq =~ /$variants_re[$_]/g );
    }
    close $writer
        or die "Cannot close child's writer in child: $!";
    exit( 0 );
}

# report

print "$_ $results{$_}\n" for @variants;
print "$_\n" for '', $input_length, $cleaned_length, length( $seq );
