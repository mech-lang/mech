import std/[re, typedthreads]

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
type Job = tuple[sequence: ptr string, first: int, last: int, counts: ptr array[9, int]]

proc patterns(job: Job) {.thread.} =
  for index in job.first ..< job.last:
    let expression = re(variants[index])
    var start = 0
    var count = 0
    while start <= job.sequence[].len:
      let bounds = findBounds(job.sequence[], expression, start)
      if bounds.first < 0:
        break
      inc count
      start = bounds.last + 1
    job.counts[][index] = count

let input = stdin.readAll()
var sequence = replace(input, re">.*\n|\n", "")
let cleanLength = sequence.len
var counts: array[9, int]
var threads: array[8, Thread[Job]]
for worker in 0 ..< 8:
  createThread(threads[worker], patterns, (addr sequence, 9 * worker div 8, 9 *
      (worker + 1) div 8, addr counts))
for worker in 0 ..< 8:
  joinThread(threads[worker])
for index, pattern in variants:
  echo pattern, " ", counts[index]
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
