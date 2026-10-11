import "../stdlib/common"
def alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
def random_record (n:i64) (seed:u32) (symbols:[]u8) (probabilities:[]f64) =
  let cdf = scan (+) 0f64 probabilities
  let (states,seed) = #[unsafe] loop (states,seed) = (replicate n 0u32,seed) for i < n do
    let seed = (seed*3877+29573)%139968
    let states[i] = seed
    in (states,seed)
  let bytes = map (\s ->
    let p = f64.u32 s/139968f64
    let k = loop k = 0i64 while k+1 < length cdf && p >= cdf[k] do k+1
    in symbols[k]) states
  in (wrap 60 bytes,seed)
entry main (n:i64) (_input:[]u8):[]u8 =
  let first = wrap 60 (map (\i -> alu[i%length alu]) (iota (2*n)))
  let (second,seed) = random_record (3*n) 42 "acgtBDHKMNRSVWY" [0.27f64,0.12,0.12,0.27,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02,0.02]
  let (third,_seed) = random_record (5*n) seed "acgt" [0.3029549426680f64,0.1979883004921,0.1975473066391,0.3015094502008]
  in ">ONE Homo sapiens alu\n" ++ first ++ ">TWO IUB ambiguity codes\n" ++ second ++ ">THREE Homo sapiens frequency\n" ++ third
