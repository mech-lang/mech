<?php
/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   line-by-line from Greg Buchholz's C program
*/



function main($args) {
    global $argv;
    $w = 0; $h = 0; $bit_num = 0;
    $byte_acc = 0;
    $i = 0; $iter = 50;
    $x = 0.0; $y = 0.0; $limit = 2.0;
    $Zr = 0.0; $Zi = 0.0; $Cr = 0.0; $Ci = 0.0; $Tr = 0.0; $Ti = 0.0;

    $h = $args[1]; $w = $h;

    printf ("P4\n%d %d\n", $w, $h);

    for ($y = 0 ; $y < $h ; $y++)
    {
        for ($x = 0 ; $x < $w ; $x++)
        {
            $Zr = 0.0; $Zi = 0.0; $Tr = 0.0; $Ti = 0.0;
            $Cr = (2.0 * $x / $w - 1.5); $Ci = (2.0 * $y / $h - 1.0);

            for ($i = 0 ; $i < $iter && ($Tr+$Ti <= $limit*$limit); ++$i)     
            {
                $Zi = 2.0*$Zr*$Zi + $Ci;
                $Zr = $Tr - $Ti + $Cr;
                $Tr = $Zr * $Zr;
                $Ti = $Zi * $Zi;        
            }
        
            $byte_acc = $byte_acc << 1;  
            if ($Tr+$Ti <= $limit*$limit) $byte_acc = $byte_acc | 0x01;       

            ++$bit_num; 

            if ($bit_num == 8)
            {
                echo chr ($byte_acc);
                $byte_acc = 0;
                $bit_num = 0;
            }
            else if ($x == $w - 1)
            {
                $byte_acc = $byte_acc << (8-$w%8);
                echo chr ($byte_acc);
                $byte_acc = 0;
                $bit_num = 0;
            }
        }        
    }
}

main($argv);
