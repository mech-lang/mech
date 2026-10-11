<?php 
/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   contributed by Isaac Gouy, transliterated from Mike Pall's Lua program 
   further optimization by Oleksii Prudkyi
*/

$n = (int)$argv[1];
$s = range(0, $n - 1);
$i = $maxflips = $checksum = 0; 
$sign = 1; 
$m = $n - 1;
$p = $q = $s;
do {
   // Copy and flip.
   $q0 = $p[0];
   if ($q0 != 0){
      $q = $p;
      $flips = 1;
      do { 
         $qq = $q[$q0]; 
         if ($qq == 0){
            $checksum += $sign*$flips;
            if ($flips > $maxflips) $maxflips = $flips;
            break; 
         } 
         $q[$q0] = $q0; 
         if ($q0 >= 3){
            $i = 1; $j = $q0 - 1;
            do { 
               $t = $q[$i]; 
               $q[$i] = $q[$j]; 
               $q[$j] = $t; 
               ++$i;
               --$j;
            } while ($i < $j); 
         }
         $q0 = $qq; 
         ++$flips;
      } while (true); 
   }
   // Permute.
   if ($sign == 1){
      $t = $p[1]; $p[1] = $p[0]; $p[0] = $t; $sign = -1; // Rotate 0<-1.
   } else { 
      $t = $p[1]; $p[1] = $p[2]; $p[2] = $t; $sign = 1;  // Rotate 1<-2.
      for($i=2; $i<$n; ){ 
         $sx = &$s[$i];
         if ($sx != 0)
         {
            --$sx; 
            break; 
         }
         if ($i == $m){
            printf("%d\nPfannkuchen(%d) = %d\n", $checksum, $n, $maxflips);// Out of permutations.
            return;
         }
         $s[$i] = $i;
         // Rotate 0<-...<-i+1.
         $t = $p[0]; 
         for($j=0; $j<=$i; ){ $p[$j++] = $p[$j]; } 
         ++$i;
         $p[$i] = $t;
      }
   }
} while (true);
