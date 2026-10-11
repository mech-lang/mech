<? /* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from bearophile's program
   contributed by Isaac Gouy   
*/


function seq_lines() {
   while ($line = fgets(STDIN)) {
      if (str_starts_with($line,">THREE")) break;      
   }
   $lines = [];
   while ($line = fgets(STDIN)) {
      if (str_starts_with($line,">")) break;  
      $lines[] = rtrim($line);      
   }   
   return $lines;
}

function base_counts($bases, $seq) { 
   $counts = [];
   $size = strlen($seq) + 1 - $bases;        
   for ($i = 0; $i < $size; $i++) {  
      $nucleo = substr($seq, $i, $bases);          
      if (isset($counts[$nucleo]))       
         $counts[$nucleo] += 1;  
      else
         $counts[$nucleo] = 1;              
   } 
   return $counts;
}

function sorted_freq($bases, $seq) {
   $size = strlen($seq) + 1 - $bases;   
   $counts = base_counts($bases, $seq);   
   arsort($counts);
   $freq = [];
   foreach($counts as $k => $v) { $freq[$k] = 100.0 * $v / $size; }
   return $freq;
}

function specific_count($code, $seq) {  
    $counts = base_counts(strlen($code),$seq);
    return (isset($counts[$code])) ? $counts[$code] : 0;
}    

function main() {
   $lines = seq_lines();
   $seq =  implode(array_map('strtoupper', $lines)); unset($lines);
   
   foreach(array(1,2) as $base) {    
      foreach(sorted_freq($base,$seq) as $k => $v){
         printf ("%s %.3f\n", $k, $v);         
      }
      echo "\n";         
   }
   
   foreach(array("GGT", "GGTA", "GGTATT",
         "GGTATTTTAATT", "GGTATTTTAATTTATAGT") as $code) {   
      printf ("%d\t%s\n", specific_count( $code,$seq), $code);       
   }  
}

main();
?>
