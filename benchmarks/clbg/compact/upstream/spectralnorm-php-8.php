<? /* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from Sebastien Loisel's C program
   contributed by Isaac Gouy   
*/

function eval_A($i, $j) { return 1.0/(($i+ $j)*($i+$j+1)/2+$i+1); }

function eval_A_times_u($N, $u, &$Au)
{
  $i = $j = 0;
  for($i=0;$i<$N;$i++)
    {
      $Au[$i]=0;
      for($j=0;$j<$N;$j++) $Au[$i]+=eval_A($i,$j)*$u[$j];
    }
}

function eval_At_times_u($N, $u, &$Au)
{
  $i = $j = 0;
  for($i=0;$i<$N;$i++)
    {
      $Au[$i]=0;
      for($j=0;$j<$N;$j++) $Au[$i]+=eval_A($j,$i)*$u[$j];
    }
}

function eval_AtA_times_u($N, $u, &$AtAu)
{ 
  $v = array_fill(0, $N, 0.0); 
  eval_A_times_u($N,$u,$v); eval_At_times_u($N,$v,$AtAu); 
}

function main($N) 
{
  $u = array_fill(0, $N, 1.0); 
  $v = array_fill(0, $N, 0.0); 
  for($i=0;$i<10;$i++)
    {
      eval_AtA_times_u($N,$u,$v);
      eval_AtA_times_u($N,$v,$u);
    }
  $vBv = $vv = 0.0;  
  for($i=0;$i<$N;$i++) { $vBv+=$u[$i]*$v[$i]; $vv+=$v[$i]*$v[$i]; }  
  printf("%0.9f\n",sqrt($vBv/$vv));     
}

$n = $argc > 1 ? $argv[1] : 100;
main($n);
?>
