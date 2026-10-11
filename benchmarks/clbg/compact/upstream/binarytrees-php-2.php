<?php 
/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Peter Baltruschat
   modified by Levi Cameron
   modified by Craig Russell
   *reset*
*/

class Tree {
   public $l;
   public $r;
   
   public function __construct($depth) {
      if($depth) {
         $this->l = new Tree(--$depth);
         $this->r = new Tree($depth);
      }
   }
   
   public function check() {
      return 1
         + ($this->l->l === null ? 1 : $this->l->check())
         + ($this->r->l === null ? 1 : $this->r->check());
   }
}

$minDepth = 4;

$n = $argc == 2 ? $argv[1] : 1;
$maxDepth = $minDepth + 2 > $n ? $minDepth + 2 : $n;
$stretchDepth = $maxDepth + 1;

$stretch = new Tree($stretchDepth);
printf("stretch tree of depth %d\t check: %d\n",
   $stretchDepth, $stretch->check());
unset($stretch);

$longLivedTree = new Tree($maxDepth);

$iterations = 1 << $maxDepth;
do
{
   $check = 0; 
   for($i = 1; $i <= $iterations; ++$i)
   {
      $check += (new Tree($minDepth))->check();
   }
   
   printf("%d\t trees of depth %d\t check: %d\n",
      $iterations, $minDepth, $check);
   
   $minDepth += 2;
   $iterations >>= 2;
}
while($minDepth <= $maxDepth);

printf("long lived tree of depth %d\t check: %d\n",
   $maxDepth, $longLivedTree->check());
