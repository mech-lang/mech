import "../stdlib/common"
def coefficient (i:i64) (j:i64) = 1f64 / f64.i64 ((i+j)*(i+j+1)/2+i+1)
def multiply [n] (transpose:bool) (u:[n]f64):[n]f64 =
  let tiles = map (\tile ->
    let i = 4*tile
    let sums = #[unsafe] loop (a,b,c,d) = (0f64,0f64,0f64,0f64) for j < n do
      let x = u[j]
      let aa = if transpose then coefficient j i else coefficient i j
      let bb = if transpose then coefficient j (i+1) else coefficient (i+1) j
      let cc = if transpose then coefficient j (i+2) else coefficient (i+2) j
      let dd = if transpose then coefficient j (i+3) else coefficient (i+3) j
      in (a+x*aa,b+x*bb,c+x*cc,d+x*dd)
    in [sums.0,sums.1,sums.2,sums.3]) (iota ((n+3)/4))
  in take n (flatten tiles)
def av [n] (u:[n]f64) = multiply false u
def atv [n] (u:[n]f64) = multiply true u
def step [n] (u:[n]f64) = atv (av u)
entry main (n:i64) (_input:[]u8):[]u8 =
  let (u,v) = loop (u,v) = (replicate n 1f64, replicate n 0f64) for _i < 10 do
    let v = step u
    let u = step v
    in (u,v)
  let uv = reduce (+) 0f64 (map2 (*) u v)
  let vv = reduce (+) 0f64 (map (\x -> x*x) v)
  in fixed 9 (f64.sqrt (uv/vv)) ++ "\n"
