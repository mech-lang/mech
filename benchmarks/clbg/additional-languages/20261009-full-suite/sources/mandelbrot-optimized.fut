import "../stdlib/common"
type quad = (f64,f64,f64,f64)
def plus (a:quad) (b:quad):quad = (a.0+b.0,a.1+b.1,a.2+b.2,a.3+b.3)
def minus (a:quad) (b:quad):quad = (a.0-b.0,a.1-b.1,a.2-b.2,a.3-b.3)
def times (a:quad) (b:quad):quad = (a.0*b.0,a.1*b.1,a.2*b.2,a.3*b.3)
def twice (a:quad):quad = (2*a.0,2*a.1,2*a.2,2*a.3)
def inside (cr:quad) (ci:f64):u8 =
  let zero = (0f64,0f64,0f64,0f64)
  let (_x,_y,_xx,_yy,_k,mask) =
    loop (x,y,xx,yy,k,mask) = (zero,zero,zero,zero,0i64,15u8)
      while k < 50 && mask != 0 do
        let y = plus (times (twice x) y) (ci,ci,ci,ci)
        let x = plus (minus xx yy) cr
        let xx = times x x
        let yy = times y y
        let r = plus xx yy
        let hit = (if r.0 <= 4 then 8u8 else 0) | (if r.1 <= 4 then 4u8 else 0) | (if r.2 <= 4 then 2u8 else 0) | (if r.3 <= 4 then 1u8 else 0)
        in (x,y,xx,yy,k+1,mask & hit)
  in mask
entry main (n:i64) (_input:[]u8):[]u8 =
  let stride = (n+7)/8
  let bytes = map (\p ->
    let y = p/stride
    let x = p%stride*8
    let ci = 2*f64.i64 y/f64.i64 n-1
    let real j = 2*f64.i64 j/f64.i64 n-1.5
    let first = inside (real x,real (x+1),real (x+2),real (x+3)) ci
    let last = inside (real (x+4),real (x+5),real (x+6),real (x+7)) ci
    let mask = if x+8 <= n then 255u8 else 255u8 << u8.i64 (x+8-n)
    in ((first << 4) | last) & mask) (iota (n*stride))
  in "P4\n" ++ decimal n ++ " " ++ decimal n ++ "\n" ++ bytes
