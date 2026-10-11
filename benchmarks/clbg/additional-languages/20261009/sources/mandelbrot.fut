import "../stdlib/common"
def inside (cr:f64) (ci:f64):u8 =
  let (x,y,xx,yy,k) = loop (x,y,xx,yy,k) = (0f64,0f64,0f64,0f64,0i64)
    while k < 50 && xx+yy <= 4 do
      let y = 2*x*y+ci
      let x = xx-yy+cr
      in (x,y,x*x,y*y,k+1)
  in if xx+yy <= 4 then 1u8 else 0u8
entry main (n:i64) (_input:[]u8):[]u8 =
  let stride = (n+7)/8
  let bytes = map (\p ->
    let y = p/stride
    let byte = p%stride
    in loop bits = 0u8 for b < 8 do
      let x = byte*8+b
      let hit = if x < n then inside (2*f64.i64 x/f64.i64 n-1.5)
                                    (2*f64.i64 y/f64.i64 n-1) else 0u8
      in bits | (hit << u8.i64 (7-b))) (iota (n*stride))
  in "P4\n" ++ decimal n ++ " " ++ decimal n ++ "\n" ++ bytes
