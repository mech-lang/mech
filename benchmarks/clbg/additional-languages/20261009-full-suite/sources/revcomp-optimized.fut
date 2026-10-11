import "../stdlib/common"
def complement (c:u8):u8 =
  let from = "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn"
  let to = "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN"
  let i = loop i = 0i64 while i < length from && from[i] != c do i+1
  in if i < length from then to[i] else c
entry main (_n:i64) (input:[]u8):[]u8 =
  let lut = map (\i -> complement (u8.i64 i)) (iota 256)
  let [records] starts : [records]i64 = filter (\i -> input[i] == 62 && (i == 0 || input[i-1] == 10)) (iota (length input))
  let ends = (drop 1 starts ++ [length input]) :> [records]i64
  let sizes = map2 (\start end ->
    let header_end = loop i = start while i < end && input[i] != 10 do i+1
    let size = reduce (+) 0i64 (map (\c -> if c == 10 || c == 13 then 0 else 1) input[header_end+1:end])
    in (header_end-start+1, size, (size+59)/60)) starts ends
  let offsets = [0i64] ++ scan (+) 0i64 (map (\(h,s,l) -> h+s+l) sizes)
  let out = replicate offsets[length starts] 0u8
  in loop out = out for record < length starts do
    let start = starts[record]
    let end = ends[record]
    let (header_size,seq_size,_lines) = sizes[record]
    let sequence = reverse (filter (\c -> c != 10 && c != 13) input[start+header_size:end])
    let base = offsets[record]
    let out = loop out = out for j < header_size do
      let out[base+j] = input[start+j]
      in out
    let bytes = map (\j ->
      let k = j/61*60+j%61
      in if j%61 == 60 || k >= seq_size then 10u8 else lut[i64.u8 sequence[k]]) (iota (seq_size+(seq_size+59)/60))
    let out[base+header_size:base+header_size+length bytes] = bytes
    in out
