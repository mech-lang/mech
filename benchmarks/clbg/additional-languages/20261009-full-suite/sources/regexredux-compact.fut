import "../stdlib/common"
import "../stdlib/regex"
def substitute (text:[]u8) (pattern:[]u8) (replacement:[]u8) = replace (compile pattern) text replacement
entry main (_n:i64) (input:[]u8):[]u8 =
  let initial = length input
  let sequence = substitute input ">.*\n|\n" ""
  let cleaned = length sequence
  let patterns = ["agggtaaa|tttaccct        ","[cgt]gggtaaa|tttaccc[acg]",
     "a[act]ggtaaa|tttacc[agt]t",
     "ag[act]gtaaa|tttac[agt]ct",
     "agg[act]taaa|ttta[agt]cct",
     "aggg[acg]aaa|ttt[cgt]ccct",
     "agggt[cgt]aa|tt[acg]accct",
     "agggta[cgt]a|t[acg]taccct",
     "agggtaa[cgt]|[acg]ttaccct"]
  let text = loop text = "" for i < 9 do
    let pattern = take (if i == 0 then 17 else 25) patterns[i]
    in text ++ pattern ++ " " ++ decimal (count (compile pattern) sequence) ++ "\n"
  let sequence = substitute sequence "tHa[Nt]" "<4>"
  let sequence = substitute sequence "aND|caN|Ha[DS]|WaS" "<3>"
  let sequence = substitute sequence "a[NSt]|BY" "<2>"
  let sequence = substitute sequence "<[^>]*>" "|"
  let sequence = substitute sequence "\\|[^|][^|]*\\|" "-"
  in text ++ "\n" ++ decimal initial ++ "\n" ++ decimal cleaned ++ "\n" ++ decimal (length sequence) ++ "\n"
