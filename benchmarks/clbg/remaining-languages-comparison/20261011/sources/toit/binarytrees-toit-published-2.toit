
/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   transliterated from Antoine Pitrou's Python program by Isaac Gouy
   */  
  
make-tree depth/int -> List:
  if depth == 0: return [null, null]
  depth -= 1
  return [make-tree depth, make-tree depth]
  
count-nodes node/List -> int:
  left := node[0]
  if not left: return 1
  right := node[1]  
  return 1 + (count-nodes left) + (count-nodes right)
    
main args:
  n := args.size == 1 ? int.parse args[0] : 0
  min-depth := 4
  max-depth := max (min-depth + 2) n
  stretch-depth := max-depth + 1  
  
  print "stretch tree of depth $stretch-depth\t "
      + "check: $(count-nodes (make-tree stretch-depth))"     
       
  long-lived-tree := make-tree max-depth
  
  for depth := min-depth; depth <= max-depth; depth += 2: 
    iterations := 1 << max-depth - depth + min-depth  
    check := 0    
    iterations.repeat: check += count-nodes (make-tree depth)      
    print "$iterations\t trees of depth $depth\t check: $check"        
  
  print "long lived tree of depth $max-depth\t "
      + "check: $(count-nodes long-lived-tree)"   

    