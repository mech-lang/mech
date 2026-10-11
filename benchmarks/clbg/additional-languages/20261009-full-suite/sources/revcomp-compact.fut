import "../stdlib/common"
entry main (_n:i64) (input:[]u8):[]u8 =
  let starts = filter (\i -> input[i] == 62 && (i == 0 || input[i-1] == 10)) (iota (length input))
  let ends = drop 1 starts ++ [length input]
  let from = "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn"
  let to = "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN"
  in loop text = "" for r < length starts do
    let start = starts[r]
    let end = ends[r]
    let h = 1+loop i = start while i < end && input[i] != 10 do i+1
    let sequence = reverse (filter (\c -> c != 10 && c != 13) input[h:end])
    let bytes = map (\c ->
      let k = loop k = 0i64 while k < length from && from[k] != c do k+1
      in if k < length to then to[k] else c) sequence
    in text ++ input[start:h] ++ wrap 60 bytes
