-- General signed, unbounded integers, stored as little-endian u32 limbs.
-- Arrays grow with the value; precision is never chosen from an input size.
type~ integer = (bool,[]u32)
def normalise (negative:bool) (a:[]u32):integer =
  let used = loop k = length a while k > 1 && a[k-1] == 0 do k-1
  let limbs = take used a
  in (negative && !(used == 1 && limbs[0] == 0),limbs)
def integer (x:i64):integer =
  let magnitude = u64.i64 (if x < 0 then -x else x)
  in normalise (x < 0) [u32.u64 magnitude,u32.u64 (magnitude >> 32)]
def compare_abs (a:[]u32) (b:[]u32):i64 =
  if length a != length b then if length a < length b then -1 else 1
  else let (_i,result) = loop (i,result) = (length a-1,0i64)
         while i >= 0 && result == 0 do
           (i-1,if a[i] < b[i] then -1 else if a[i] > b[i] then 1 else 0)
       in result
def compare (a:integer) (b:integer):i64 =
  if a.0 != b.0 then if a.0 then -1 else 1
  else (if a.0 then -1 else 1)*compare_abs a.1 b.1
def add_abs (a:[]u32) (b:[]u32):[]u32 =
  let size = i64.max (length a) (length b)
  let (out,carry) = loop (out,carry) = (replicate (size+1) 0u32,0u64) for i < size do
    let x = if i < length a then u64.u32 a[i] else 0
    let y = if i < length b then u64.u32 b[i] else 0
    let sum = x+y+carry
    let out[i] = u32.u64 sum
    in (out,sum >> 32)
  let out[size] = u32.u64 carry
  in (normalise false out).1
def sub_abs (a:[]u32) (b:[]u32):[]u32 =
  let (out,_borrow) = loop (out,borrow) = (replicate (length a) 0u32,0u64) for i < length a do
    let x = u64.u32 a[i]
    let y = (if i < length b then u64.u32 b[i] else 0)+borrow
    let out[i] = u32.u64 (x-y)
    in (out,if x < y then 1u64 else 0)
  in (normalise false out).1
def add (a:integer) (b:integer):integer =
  if a.0 == b.0 then normalise a.0 (add_abs a.1 b.1)
  else if compare_abs a.1 b.1 >= 0 then normalise a.0 (sub_abs a.1 b.1)
  else normalise b.0 (sub_abs b.1 a.1)
def negate (a:integer):integer = normalise (!a.0) a.1
def subtract (a:integer) (b:integer):integer = add a (negate b)
def mul_abs (a:[]u32) (b:u32):[]u32 =
  let (out,carry) = loop (out,carry) = (replicate (length a+1) 0u32,0u64) for i < length a do
    let product = u64.u32 a[i]*u64.u32 b+carry
    let out[i] = u32.u64 product
    in (out,product >> 32)
  let out[length a] = u32.u64 carry
  in (normalise false out).1
def multiply_small (a:integer) (b:u32):integer = normalise a.0 (mul_abs a.1 b)
def quotient (a:integer) (b:integer):integer =
  assert (!(length b.1 == 1 && b.1[0] == 0))
  let factor = u32.u64 (4294967296u64/(u64.u32 b.1[length b.1-1]+1))
  let numerator = mul_abs a.1 factor
  let divisor = mul_abs b.1 factor
  let slots = i64.max 1 (length numerator-length divisor+1)
  let (q,remainder) = loop (q,r) = (replicate slots 0u32,numerator)
    for shift in reverse (iota slots) do
      let high_index = length divisor+shift
      let high = if high_index < length r then u64.u32 r[high_index] else 0
      let low = if high_index-1 < length r then u64.u32 r[high_index-1] else 0
      let estimate = u32.u64 (u64.min 4294967295u64 (((high << 32)|low)/u64.u32 divisor[length divisor-1]))
      let shifted = replicate shift 0u32 ++ divisor
      let product = (normalise false (replicate shift 0u32 ++ mul_abs divisor estimate)).1
      let (digit,product) = loop (digit,product) = (estimate,product)
        while compare_abs product r > 0 do (digit-1,sub_abs product shifted)
      let r = sub_abs r product
      let q[shift] = digit
      in (q,r)
  let result = normalise false q
  let different_sign = a.0 != b.0
  in if !different_sign then result
     else if length remainder == 1 && remainder[0] == 0 then negate result
     else negate (add result (integer 1))
def to_i64 (a:integer):i64 =
  assert (length a.1 <= 2)
  let value = u64.u32 a.1[0] | (if length a.1 > 1 then u64.u32 a.1[1] << 32 else 0)
  in (if a.0 then -1 else 1)*i64.u64 value
