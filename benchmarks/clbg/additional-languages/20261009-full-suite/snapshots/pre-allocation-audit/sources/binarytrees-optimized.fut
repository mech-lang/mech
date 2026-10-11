import "../stdlib/common"
def build (depth:i64) (identity:i64):[](i32,i32) =
  let d = (#[opaque] (depth,identity)).0
  let size = (1i64 << (d+1))-1
  in #[opaque] map (\i ->
    if 2*i+1 < size then (i32.i64 (2*i+1),i32.i64 (2*i+2)) else (-1i32,-1i32)) (iota size)
def check [n] (tree:[n](i32,i32)) (root:i32):i64 =
  if root < 0 then 0 else
    let node = #[unsafe] tree[i64.i32 root]
    in if node.0 < 0 && node.1 < 0 then 1 else 1+check tree node.0+check tree node.1
entry main (n:i64) (_input:[]u8):[]u8 =
  let depth = i64.max 6 n
  let valid = assert (depth < 30) depth
  let stretch = build (valid+1) (-1)
  let stretch_text = "stretch tree of depth " ++ decimal (depth+1) ++ "\t check: " ++ decimal (check stretch 0) ++ "\n"
  let long_lived = build valid (-2)
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
