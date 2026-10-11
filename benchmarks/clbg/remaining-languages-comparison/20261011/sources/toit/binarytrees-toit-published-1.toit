
/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   transliterated from Antoine Pitrou's Python program by Isaac Gouy  
*/  

class Tree:  
  left /Tree? := ?
  right /Tree? := ?
  
  constructor .left .right:  
  
  static with depth/int -> Tree:
    if depth == 0: return Tree null null
    depth -= 1
    return Tree (Tree.with depth) (Tree.with depth)
  
  node-count -> int:
    if not this.left: return 1
    return 1 + this.left.node-count + this.right.node-count
    
main args:
  n := args.size == 1 ? int.parse args[0] : 0
  min-depth := 4
  max-depth := max (min-depth + 2) n
  stretch-depth := max-depth + 1  
  
  print "stretch tree of depth $stretch-depth\t "
      + "check: $((Tree.with stretch-depth).node-count)"     
       
  long-lived-tree := Tree.with max-depth
  
  for depth := min-depth; depth <= max-depth; depth += 2: 
    iterations := 1 << max-depth - depth + min-depth  
    check := 0    
    iterations.repeat: check += (Tree.with depth).node-count     
    print "$iterations\t trees of depth $depth\t check: $check"        
  
  print "long lived tree of depth $max-depth\t "
      + "check: $(long-lived-tree.node-count)"   

    