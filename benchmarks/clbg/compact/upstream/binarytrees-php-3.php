<?php 
/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Peter Baltruschat
   modified by Levi Cameron
   *reset*
*/

function bottomUpTree($depth)
{
   if (!$depth) return array(null,null);
   $depth--;
   return array(
      bottomUpTree($depth),
      bottomUpTree($depth));
}

function itemCheck($treeNode) { 
   return 1
      + ($treeNode[0][0] === null ? 1 : itemCheck($treeNode[0]))
      + ($treeNode[1][0] === null ? 1 : itemCheck($treeNode[1]));
}

$minDepth = 4;

$n = ($argc == 2) ? $argv[1] : 1;
$maxDepth = max($minDepth + 2, $n);
$stretchDepth = $maxDepth + 1;

$stretchTree = bottomUpTree($stretchDepth);
printf("stretch tree of depth %d\t check: %d\n",
$stretchDepth, itemCheck($stretchTree));
unset($stretchTree);

$longLivedTree = bottomUpTree($maxDepth);

$iterations = 1 << ($maxDepth);
do
{
   $check = 0;
   for($i = 1; $i <= $iterations; ++$i)
   {
      $t = bottomUpTree($minDepth);
      $check += itemCheck($t);
      unset($t);
   }
   
   printf("%d\t trees of depth %d\t check: %d\n", $iterations, $minDepth, $check);
   
   $minDepth += 2;
   $iterations >>= 2;
}
while($minDepth <= $maxDepth);

printf("long lived tree of depth %d\t check: %d\n",
$maxDepth, itemCheck($longLivedTree));
?>
