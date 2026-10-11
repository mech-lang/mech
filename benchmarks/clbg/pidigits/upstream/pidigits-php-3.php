<?php 
/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Oleksii Prudkyi
   port from pidigits.lua-5.lua (Mike Pall, Wim Couwenberg)
*/

$u = gmp_init(0);
$v = gmp_init(0);
$w = gmp_init(0);

function produce(&$n1, &$n2, &$d, $k) 
{
   global $u, $v, $w;
   $u = gmp_mul($n1, 2*$k-1);
   $v = gmp_add($n2, $n2);
   $w = gmp_mul($n1, $k-1);
   $n1 = gmp_add($u, $v);
   $u = gmp_mul($n2, $k+2);
   $n2 = gmp_add($w, $u);
   $d = gmp_mul($d, 2*$k+1);
}

function extractd(&$n1, &$n2, $d, $y)
{
   global $u;
   $u = gmp_mul($d, gmp_mul(-10, $y));
   $n1 = gmp_mul($n1, 10);
   $n1 = gmp_add($n1, $u);
   $n2 = gmp_mul($n2, 10);
   $n2 = gmp_add($n2, $u);
}

function digit($n1, $n2, $d)
{
   global $u, $v;
   $u = gmp_div_q($n1, $d);
   $v = gmp_div_q($n2, $d);
   if(gmp_cmp($u, $v) == 0) {
      return $u;
   }
   return false;
}

//Generate successive digits of PI.
function pidigits($N)
{
   $k = 1;
   $n1 = gmp_init(4);
   $n2 = gmp_init(3);
   $d = gmp_init(1);
   
   $i = 0;
   while($i < $N) {
      $y = digit($n1, $n2, $d);
      if($y !== false) {
         echo gmp_strval($y);
         $i++;
         if($i % 10 == 0) {
            echo "\t:" , $i , "\n";
         }
         extractd($n1, $n2, $d, $y);
      } else {
         produce($n1, $n2, $d, $k);
         $k++;
      }
   }
   if($i % 10 != 0) {
      echo str_repeat(' ', 10 - $N % 10), "\t:", $N, "\n";
   }
}

$N = (int) $argv[1];
ob_start();
pidigits($N);
ob_end_flush();
