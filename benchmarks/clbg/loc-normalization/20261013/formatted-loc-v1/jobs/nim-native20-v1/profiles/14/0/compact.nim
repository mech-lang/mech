import std/re

let input = stdin.readAll()
var sequence = replace(input, re">.*\n|\n", "")
let cleanLength = sequence.len
const variants = [
  "agggtaaa|tttaccct",
  "[cgt]gggtaaa|tttaccc[acg]",
  "a[act]ggtaaa|tttacc[agt]t",
  "ag[act]gtaaa|tttac[agt]ct",
  "agg[act]taaa|ttta[agt]cct",
  "aggg[acg]aaa|ttt[cgt]ccct",
  "agggt[cgt]aa|tt[acg]accct",
  "agggta[cgt]a|t[acg]taccct",
  "agggtaa[cgt]|[acg]ttaccct"]
for pattern in variants:
  echo pattern, " ", findAll(sequence, re(pattern)).len
const substitutions = [
  ("tHa[Nt]", "<4>"),
  ("aND|caN|Ha[DS]|WaS", "<3>"),
  ("a[NSt]|BY", "<2>"),
  ("<[^>]*>", "|"),
  ("\\|[^|][^|]*\\|", "-")]
for substitution in substitutions:
  sequence = replace(sequence, re(substitution[0]), substitution[1])
echo ""
echo input.len
echo cleanLength
echo sequence.len
