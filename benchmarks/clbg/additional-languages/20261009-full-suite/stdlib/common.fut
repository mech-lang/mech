-- General text formatting, independent of the benchmark algorithms.
def decimal (x:i64): []u8 =
  let neg = x < 0
  let x = if neg then -x else x
  let (x, digits, used) = loop (v, a, k) = (x, replicate 20 0u8, 0i64)
    while v > 0 || k == 0 do
      let a[19-k] = u8.i64 (v % 10 + 48)
      in (v/10, a, k+1)
  in (if neg then "-" else "") ++ drop (20-used) digits

def fixed (digits:i64) (x:f64): []u8 =
  let scale = loop s = 1i64 for _i < digits do s*10
  let v = i64.f64 (f64.abs x * f64.i64 scale + 0.5)
  let frac = decimal (v%scale)
  in (if x < 0 then "-" else "") ++ decimal (v/scale) ++ "." ++
     replicate (digits-length frac) 48u8 ++ frac

def upper (c:u8) = if c >= 97 && c <= 122 then c-32 else c

-- General fixed-width text wrapping, with a newline after each nonempty line.
def wrap (width:i64) (text:[]u8):[]u8 =
  assert (width > 0)
  let size = length text+(length text+width-1)/width
  in map (\i ->
    let j = i/(width+1)*width+i%(width+1)
    in if i%(width+1) == width || j >= length text then 10u8 else text[j]) (iota size)
