import "../stdlib/common"
def build (depth:i64) (identity:i64):[][2]i64 =
  -- Keep each requested construction inside its iteration, and materialise
  -- every two-child node, including leaves.  No shared/constant leaves.
  let d = (#[opaque] (depth,identity)).0
  let size = (1i64 << (d+1))-1
  in #[opaque] map (\i -> if 2*i+1 < size then [2*i+1,2*i+2] else [-1i64,-1]) (iota size)
def check [n] (tree:[n][2]i64) (root:i64):i64 =
  if root < 0 then 0 else 1+check tree tree[root,0]+check tree tree[root,1]
entry main (n:i64) (_input:[]u8):[]u8 =
  let depth = i64.max 6 n
  let stretch = build (depth+1) (-1)
  let stretch_text = "stretch tree of depth " ++ decimal (depth+1) ++ "\t check: " ++ decimal (check stretch 0) ++ "\n"
  let long_lived = build depth (-2)
  let lines = loop text = stretch_text for index < (depth-4)/2+1 do
    let d = 4+2*index
    let iterations = 1i64 << (depth-d+4)
    let partials = map (\w ->
      loop sum = 0i64 for i in w*iterations/8..<(w+1)*iterations/8 do
        let tree = build d i
        in sum+check tree 0) (iota 8)
    let sum = reduce (+) 0i64 partials
    in text ++ decimal iterations ++ "\t trees of depth " ++ decimal d ++ "\t check: " ++ decimal sum ++ "\n"
  in lines ++ "long lived tree of depth " ++ decimal depth ++ "\t check: " ++ decimal (check long_lived 0) ++ "\n"
