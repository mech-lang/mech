import "../stdlib/common"
import "hashmap-fast"
def encode (text:[]u8):u64 =
  loop key = 0u64 for c in text do
    key*4+(if upper c == 65 then 0 else if upper c == 67 then 1 else if upper c == 71 then 2 else 3)
def decode (width:i64) (key:u64):[]u8 =
  let symbols = "ACGT"
  in map (\j -> symbols[i64.u64 ((key >> u64.i64 (2*(width-j-1)))&3)]) (iota width)
def counts (dna:[]u8) (width:i64) (wanted:u64):*[]i64 =
  let limit = length dna-width+1
  let bins = if width <= 2 then 1i64 << (2*width) else 1
  let pieces = map (\piece ->
    let first = piece*limit/32
    let last = (piece+1)*limit/32
    let mask = (1u64 << u64.i64 (2*width))-1
    let key = if first < last then (loop key = 0u64 for c in dna[first:first+width] do key*4+u64.u8 c) else 0u64
    let (table,_key) = loop (t,key) = (empty 16,key) for i in first..<last do
      let value = get t key
      let t = put t key (value+1)
      let key = if i+width < length dna then
          ((key*4) | u64.u8 dna[i+width])&mask else key
      in (t,key)
    in map (\bin -> get table (if width <= 2 then u64.i64 bin else wanted)) (iota bins)) (iota 32)
  in map (\bin -> reduce (+) 0i64 (map (\piece -> piece[bin]) pieces)) (iota bins)
def frequency (dna:[]u8) (width:i64):[]u8 =
  let values = counts dna width 0
  let total = length dna-width+1
  let (_values,text) = loop (values,text) = (values,"") for _i < length values do
    let key = loop best = 0i64 for j < length values do if values[j] > values[best] then j else best
    let value = values[key]
    let text = if value <= 0 then text else
      text ++ decode width (u64.i64 key) ++ " " ++ fixed 3 (100*f64.i64 value/f64.i64 total) ++ "\n"
    let values[key] = -1
    in (values,text)
  in text ++ "\n"
entry main (_n:i64) (input:[]u8):[]u8 =
  let marker = ">THREE"
  let header = loop i = 0i64 while i+6 <= length input && (input[i:i+6] :> [6]u8) != marker do i+1
  let start = 1+loop i = header while i < length input && input[i] != 10 do i+1
  let sequence = filter (\c -> c != 10 && c != 13) (drop start input)
  let dna = map (\c -> if upper c == 65 then 0u8 else if upper c == 67 then 1 else if upper c == 71 then 2 else 3) sequence
  let text = frequency dna 1 ++ frequency dna 2
  let text = loop text = text for i < 5 do
    let queries = ["GGT               ","GGTA              ","GGTATT            ","GGTATTTTAATT      ","GGTATTTTAATTTATAGT"]
    let widths = [3i64,4,6,12,18]
    let query = take widths[i] queries[i]
    let key = encode query
    let hits = counts dna (length query) key
    in text ++ decimal hits[0] ++ "\t" ++ query ++ "\n"
  in text
