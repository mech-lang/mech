<?php
#
# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# reverse complement in PHP
# contributed by Danny Sauer
#

# We'll need some definitions
define( 'LINE_LENGTH', 60 );
define( 'SRC', 'CGATMKRYVBHD');
define( 'DST', 'GCTAKMYRBVDH');
$str = '';
$seq = '';

# read in the file, a line at a time
while( !feof(STDIN) ) {
    $str = trim(fgets(STDIN));
    if( $str && $str[0] == '>' ){
        # if we're on a comment line, print the previous seq and move on
        print_seq();
        echo $str, "\n";
    }else{
        # otherwise, just append to the sequence
        $seq .= $str;
    }
}
print_seq();

exit;

# print the sequence out, if it exists
function print_seq(){
    global $seq; # no time-consuming argument passing for us! :)
    if($seq != ''){
        echo wordwrap( strrev( strtr(strtoupper($seq), SRC, DST) ),
                       LINE_LENGTH, "\n", true ), "\n";
    }
    $seq = '';
}
?>
