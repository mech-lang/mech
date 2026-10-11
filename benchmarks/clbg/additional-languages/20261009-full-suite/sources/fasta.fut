import "../stdlib/common"
def alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
def random_record (n:i64) (seed:i64) (symbols:[]u8) (cdf:[]f64) =
  let size = n+(n+59)/60
  let (bytes,seed) = loop (out,seed) = (replicate size 0u8,seed) for i < n do
    let seed = (seed*3877+29573)%139968
    let p = f64.i64 seed/139968f64
    let k = loop k = 0i64 while k+1 < length cdf && p >= cdf[k] do k+1
    let out[i+i/60] = symbols[k]
    in (out,seed)
  let bytes = map (\i -> if i%61 == 60 || i == size-1 then 10u8 else bytes[i]) (iota size)
  in (bytes,seed)
entry main (n:i64) (_input:[]u8):[]u8 =
  let repeat_size = 2*n+(2*n+59)/60
  let repeated = map (\i -> if i%61 == 60 || i == repeat_size-1 then 10u8
      else alu[(i/61*60+i%61)%length alu]) (iota repeat_size)
  let iub = scan (+) 0f64 [0.27f64,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02]
  let homo = scan (+) 0f64 [0.3029549426680f64,0.1979883004921,0.1975473066391,0.3015094502008]
  let (second,seed) = random_record (3*n) 42 "acgtBDHKMNRSVWY" iub
  let (third,_seed) = random_record (5*n) seed "acgt" homo
  in ">ONE Homo sapiens alu\n" ++ repeated ++ ">TWO IUB ambiguity codes\n" ++ second ++ ">THREE Homo sapiens frequency\n" ++ third
