import "../stdlib/common"
import "../stdlib/hashmap"
def encode (text:[]u8):u64 =
  loop key = 0u64 for c in text do key*4+(if upper c == 65 then 0 else if upper c == 67 then 1 else if upper c == 71 then 2 else 3)
def decode (width:i64) (key:u64):[]u8 =
  let symbols = "ACGT"
  in map (\j -> symbols[i64.u64 ((key >> u64.i64 (2*(width-j-1)))&3)]) (iota width)
def counts (dna:[]u8) (width:i64):*table =
  let mask = (1u64 << u64.i64 (2*width))-1
  let (table,_key) = loop (table,key) = (empty 16,0u64) for i < length dna do
    let key = ((key*4) | u64.u8 dna[i]) & mask
    let table = if i+1 >= width then increment table key 1 else table
    in (table,key)
  in table
def frequency (dna:[]u8) (width:i64):[]u8 =
  let table = counts dna width
  let values = map (\k -> get table (u64.i64 k)) (iota (1i64 << (2*width)))
  let (_values,text) = loop (values,text) = (values,"") for _i < length values do
    let key = loop best = 0i64 for j < length values do if values[j] > values[best] then j else best
    let value = values[key]
    let text = if value <= 0 then text else text ++ decode width (u64.i64 key) ++ " " ++ fixed 3 (100*f64.i64 value/f64.i64 (length dna-width+1)) ++ "\n"
    let values[key] = -1
    in (values,text)
  in text ++ "\n"
entry main (_n:i64) (input:[]u8):[]u8 =
  let marker = ">THREE"
  let header = loop i = 0i64 while i+6 <= length input && (input[i:i+6] :> [6]u8) != marker do i+1
  let start = 1+loop i = header while i < length input && input[i] != 10 do i+1
  let end = loop i = start while i < length input && input[i] != 62 do i+1
  let dna = map (\c -> if upper c == 65 then 0u8 else if upper c == 67 then 1 else if upper c == 71 then 2 else 3) (filter (\c -> c != 10 && c != 13) input[start:end])
  let text = frequency dna 1 ++ frequency dna 2
  in loop text = text for i < 5 do
    let queries = ["GGT               ","GGTA              ","GGTATT            ","GGTATTTTAATT      ","GGTATTTTAATTTATAGT"]
    let widths = [3i64,4,6,12,18]
    let query = take widths[i] queries[i]
    let table = counts dna widths[i]
    in text ++ decimal (get table (encode query)) ++ "\t" ++ query ++ "\n"
