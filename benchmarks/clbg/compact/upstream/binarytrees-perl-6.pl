# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# contributed by Emanuele Zeppieri
# *reset* by A. Sinan Unur
# thread and depth tweaks by Richard Leach

use threads;
my @threads;

use threads::shared;
my @tasks :shared;
my @checks :shared;
my $phase :shared;

run( @ARGV );

sub bottomup_tree {
    my $depth = $_[0];
    (--$depth)
    ? [ bottomup_tree($depth), bottomup_tree($depth) ]
    : []
    ;
}

sub check_tree {
    1 +
    ( ( $_[0]->@* ) ? check_tree($_[0][0]) + check_tree($_[0][1]) : 2 )
    ;
}

sub stretch_tree {
    my ($stretch_depth) = @_;
    print "stretch tree of depth $stretch_depth\t check: ",
    check_tree( bottomup_tree($stretch_depth) ), "\n";
}

sub num_cpus {
   open my $fh, '</proc/cpuinfo' or return 4;
   my $cpus;
   while(<$fh>) {
      $cpus ++ if /^processor\s+:/;
   }
   return $cpus;
}

sub depth_iteration {
    until ($phase) { sleep 1; }
    while ( scalar(@tasks) > 0) {
    my ($depth, $iterations, $check) = (shift @tasks, shift @tasks, 0);
        foreach (1..$iterations) {
            $check += check_tree( bottomup_tree($depth) );
        }
        $checks[$depth] += $check;
    }
}

sub run {
    my ($max_depth, $min_depth) = (shift, 4);
    $max_depth = $min_depth + 2 if $min_depth + 2 > $max_depth;

    my %depths;
    my $cpu_count = num_cpus();

    $phase = 0;

    stretch_tree( $max_depth + 1 );

    for ( 1 .. $cpu_count - 1){
        push @threads, threads->create(
            \&depth_iteration);
    }

    my $longlived_tree = bottomup_tree($max_depth);

    for ( my $depth = $min_depth; $depth <= $max_depth; $depth += 2 ) {
        my $iterations = 2**($max_depth - $depth + $min_depth);

        my $per_thread_iterations = int($iterations / $cpu_count) || 1;
        my $iterations_allocated = 0;

        while ($iterations_allocated < $iterations) {
            push @tasks, ( $depth, $per_thread_iterations );
            $iterations_allocated += $per_thread_iterations;
        }

        $depths{$depth}{iterations} = $iterations;
    }

    $phase = 1;
    depth_iteration();

    for (@threads) {
        $_->join;
    }

    for my $depth ( sort { $a <=> $b } keys %depths ) {
        print $depths{$depth}{iterations}, "\t trees of depth $depth\t check: ", $checks[$depth], "\n";
    }

    print "long lived tree of depth $max_depth\t check: ",
        check_tree($longlived_tree), "\n"
}
