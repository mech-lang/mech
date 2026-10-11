// Faithful Benchmarks Game workload; local Toit candidate.
import io.stdio
import io show Buffer
import regexp show RegExp

replace-all pattern/string replacement/string subject/string -> string:
  expression := RegExp pattern
  result := Buffer
  offset := 0
  expression.all-matches subject 0: | match |
    result.write subject offset match.index
    result.write replacement
    offset = match.end-index
  result.write subject offset subject.size
  return result.bytes.to-string

main args:
  raw := stdio.stdin.read-all
  subject := raw ? raw.to-string : ""
  input-size := subject.size
  subject = replace-all ">[^\n]*\n|\n" "" subject
  cleaned-size := subject.size
  ["agggtaaa|tttaccct", "[cgt]gggtaaa|tttaccc[acg]", "a[act]ggtaaa|tttacc[agt]t", "ag[act]gtaaa|tttac[agt]ct", "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct", "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct", "agggtaa[cgt]|[acg]ttaccct"].do: | pattern |
    count := 0
    (RegExp pattern).all-matches subject 0: count++
    print "$pattern $count"
  [["tHa[Nt]", "<4>"], ["aND|caN|Ha[DS]|WaS", "<3>"], ["a[NSt]|BY", "<2>"], ["<[^>]*>", "|"], ["\\|[^|][^|]*\\|", "-"]].do:
    subject = replace-all it[0] it[1] subject
  print "\n$input-size\n$cleaned-size\n$(subject.size)"
