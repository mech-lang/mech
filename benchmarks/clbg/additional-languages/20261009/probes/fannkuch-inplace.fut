import "../stdlib/common"
def flipcount [n] (p:[n]i64):i64 =
  let (_q,f) = loop (q,f) = (copy p,0i64) while q[0] != 0 do
    let k = q[0]
    let q = loop q = q for i < (k+1)/2 do
      let value = q[i]
      let q[i] = q[k-i]
      let q[k-i] = value
      in q
    in (q,f+1)
  in f
def work [n] (facts:[n]i64) (first:i64) (last:i64) =
  let (p,counts,_rank) = loop (p,c,r) = (iota n,replicate n 0i64,first)
    for i in reverse (1..<n) do
      let d = r/facts[i]
      let prefix = take (i+1) p
      let p = rotate d prefix ++ drop (i+1) p
      let c[i] = d
      in (p,c,r%facts[i])
  let (_p,_c,checksum,maximum) = loop (p,c,sum,best) = (p,counts,0i64,0i64)
    for rank in first..<last do
      let f = flipcount p
      let sum = sum+(if rank%2 == 0 then f else -f)
      let best = i64.max best f
      let (p,c,_r,_carry) = loop (p,c,r,carry) = (p,c,1i64,true)
        while carry && r < n do
          let value = p[0]
          let p = loop p = p for i < r do
            let p[i] = p[i+1]
            in p
          let p[r] = value
          let exhausted = c[r] == r
          let c[r] = if exhausted then 0 else c[r]+1
          in (p,c,r+1,exhausted)
      in (p,c,sum,best)
  in (checksum,maximum)
entry main (n:i64) (_input:[]u8):[]u8 =
  let facts = scan (*) 1i64 (map (+1) (iota n))
  let facts = [1i64] ++ take (n-1) facts
  let total = facts[n-1]*n
  let results = map (\worker -> work facts (worker*total/8) ((worker+1)*total/8)) (iota 8)
  let checksum = reduce (+) 0i64 (map (\x -> x.0) results)
  let maximum = reduce i64.max 0i64 (map (\x -> x.1) results)
  in decimal checksum ++ "\nPfannkuchen(" ++ decimal n ++ ") = " ++ decimal maximum ++ "\n"
