import "../stdlib/common"
def alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
def random_record (n:i64) (seed:i64) (symbols:[]u8) (probabilities:[]f64) =
  let cdf = scan (+) 0f64 probabilities
  let (text,seed) = loop (text,seed) = (replicate n 0u8,seed) for i < n do
    let seed = (seed*3877+29573)%139968
    let p = f64.i64 seed/139968f64
    let k = loop k = 0i64 while k+1 < length cdf && p >= cdf[k] do k+1
    let text[i] = symbols[k]
    in (text,seed)
  in (wrap 60 text,seed)
entry main (n:i64) (_input:[]u8):[]u8 =
  let first = wrap 60 (map (\i -> alu[i%length alu]) (iota (2*n)))
  let (second,seed) = random_record (3*n) 42 "acgtBDHKMNRSVWY" [0.27f64,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02]
  let (third,_seed) = random_record (5*n) seed "acgt" [0.3029549426680f64,0.1979883004921,0.1975473066391,0.3015094502008]
  in ">ONE Homo sapiens alu\n" ++ first ++ ">TWO IUB ambiguity codes\n" ++ second ++ ">THREE Homo sapiens frequency\n" ++ third
