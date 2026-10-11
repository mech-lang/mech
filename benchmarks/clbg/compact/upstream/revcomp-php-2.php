<?php
#
# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# reverse complement in PHP
# contributed by Danny Sauer
# modified by Craig Russell
#


ob_implicit_flush(1);
ob_start(NULL, 4096);

$seq = '';

# read in the file, a line at a time
do {
    $str = trim(fgets(STDIN));
    if( $str && $str[0] == '>' ){
        # if we're on a comment line, print the previous seq and move on
		if($seq != ''){
		   echo wordwrap( strrev( strtr(strtoupper($seq),
			  'CGATMKRYVBHD', 'GCTAKMYRBVDH') ), 60, "\n", true ), "\n";
		   $seq = '';
		}
        echo $str, "\n";
    }else{
        # otherwise, just append to the sequence
        $seq .= $str;
    }
} while( !feof(STDIN) );
if($seq != ''){
	echo wordwrap( strrev( strtr(strtoupper($seq),
	   'CGATMKRYVBHD', 'GCTAKMYRBVDH') ), 60, "\n", true ), "\n";
}
