
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
    
benchmark-main args:
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

    
visit node/List seen/List:
  if node.size != 2: throw "UNEQUAL_NODE_SIZE"
  seen.do: if identical it node: throw "SHARED_NODE"
  seen.add node
  if node[0]:
    visit node[0] seen
    visit node[1] seen
  else if node[1]: throw "UNEQUAL_CHILDREN"
main:
  9.repeat: | depth |
    root := make-tree depth
    seen := []
    visit root seen
    if seen.size != (1 << (depth + 1)) - 1: throw "WRONG_COUNT"
  print "distinct equal-size trees verified"
