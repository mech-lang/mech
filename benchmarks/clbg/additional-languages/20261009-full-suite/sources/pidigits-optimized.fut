import "../stdlib/common"
import "../stdlib/bigint-fast"
entry main (n:i64) (_input:[]u8):[]u8 =
  let (_q,_r,_t,_k,_written,digits) =
    loop (q,r,t,k,written,out) = (integer 1,integer 0,integer 1,0i64,0i64,replicate n 0u8)
      while written < n do
        let k = k+1
        let odd = u32.i64 (2*k+1)
        let r = multiply_small (add r (multiply_small q 2)) odd
        let q = multiply_small q (u32.i64 k)
        let t = multiply_small t odd
        in if compare q r > 0 then (q,r,t,k,written,out)
           else let d = to_i64 (quotient (add (multiply_small q 3) r) t)
                let e = to_i64 (quotient (add (multiply_small q 4) r) t)
                in if d != e then (q,r,t,k,written,out)
                   else let out[written] = u8.i64 (d+48)
                        let r = multiply_small (subtract r (multiply_small t (u32.i64 d))) 10
                        let q = multiply_small q 10
                        in (q,r,t,k,written+1,out)
  in loop text = "" for line < (n+9)/10 do
    let used = i64.min n ((line+1)*10)
    let chunk = digits[line*10:used]
    in text ++ chunk ++ replicate (10-length chunk) 32u8 ++ "\t:" ++ decimal used ++ "\n"
