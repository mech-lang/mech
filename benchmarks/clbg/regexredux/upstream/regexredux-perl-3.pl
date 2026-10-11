# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by A. Sinan Unur
#
# This version uses both fork and threads. The
# child does the matching work and parallelizes
# it using threads while the parent does the
# substitutions.
#
# Mixing threads and forks may not be good for
# your health, but here it is.

use threads;

use constant ITEMS_PER_THREAD => 3;

run();

sub count_matches {
    my $seq = shift;
    my $keys = shift;
    my $patterns = shift;

    my %results;
    for my $i (0 .. $#$patterns) {
        $results{ $keys->[$i] } = () = $seq =~ /$patterns->[$i]/g;
    }
    return %results;
}

sub run {

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

    my @variants_re = map qr/$_/iaa, @variants;

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

    pipe(my $reader, my $writer)
        or die "Failed to set up pipe: $!";

    my $pid = fork;

    if (!$pid) {
        # we are in the child
        die "Failed to fork: $!" unless defined $pid;
        close $reader or die "Failed to close parent's reader in child: $!";

        my @threads = map threads->create(
            { context => 'list' },
            \&count_matches,
            $seq,
            [    @variants[$_ .. ($_ + (ITEMS_PER_THREAD - 1))] ],
            [ @variants_re[$_ .. ($_ + (ITEMS_PER_THREAD - 1))] ],
        ), grep !($_ % ITEMS_PER_THREAD), 0 .. $#variants ;

        my %results = map $_->join, @threads;

        print $writer "$_ $results{$_}\n" for @variants;

        close $writer or die "Failed to close child's writer in child: $!";
        exit( 0 );
    }
    else {
        # we are in the parent
        close $writer or die "Failed to close child's writer in parent: $!";

        # do our own work
        $_->($seq) for @iub;

        # print match counts from child
        print while <$reader>;

        close $reader or die "Failed to close parent's reader in parent: $!";

        waitpid($pid, 0);

        print "$_\n" for '', $input_length, $cleaned_length, length( $seq );
    }
}
