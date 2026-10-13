import std/[strutils, tables, algorithm, typedthreads]

const lengths = [1, 2, 3, 4, 6, 12, 18]
const queries = ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]
type Entry = tuple[word: string, count: int64]
type Job = tuple[sequence: ptr string, width: int, output: ptr Table[uint64, int64]]

proc thirdRecord(): string =
  var selected = false
  for line in stdin.readAll().splitLines():
    if line.startsWith(";"):
      continue
    if line.startsWith(">"):
      if selected:
        break
      selected = line == ">THREE" or line.startsWith(">THREE ") or line.startsWith(">THREE\t")
    elif selected:
      result.add(toUpperAscii(line))

proc code(value: char): uint64 {.inline.} =
  case value
  of 'A': 0'u64
  of 'C': 1'u64
  of 'G': 2'u64
  of 'T': 3'u64
  else: raise newException(ValueError, "Invalid nucleotide")

proc histogram(job: Job) {.thread.} =
  job.output[] = initTable[uint64, int64]()
  let mask = (1'u64 shl (2 * job.width)) - 1
  var key = 0'u64
  for index, value in job.sequence[]:
    key = ((key shl 2) or code(value)) and mask
    if index + 1 >= job.width:
      job.output[][key] = job.output[].getOrDefault(key) + 1

proc decode(key: uint64, width: int): string =
  result = newString(width)
  var remaining = key
  for index in countdown(width - 1, 0):
    result[index] = "ACGT"[int(remaining and 3)]
    remaining = remaining shr 2

proc compare(left, right: Entry): int =
  if left.count != right.count:
    cmp(right.count, left.count)
  else:
    cmp(left.word, right.word)

var sequence = thirdRecord()
var counts: array[7, Table[uint64, int64]]
var threads: array[7, Thread[Job]]
for index, width in lengths:
  createThread(threads[index], histogram, (addr sequence, width, addr counts[index]))
for index in 0 ..< 7:
  joinThread(threads[index])
for index in 0 ..< 2:
  var rows: seq[Entry]
  var total = 0'i64
  for key, count in counts[index]:
    rows.add((decode(key, lengths[index]), count))
    total += count
  rows.sort(compare)
  for row in rows:
    echo row.word, " ", formatFloat(100.0 * float64(row.count) / float64(total), ffDecimal, 3)
  echo ""
for index, word in queries:
  var key = 0'u64
  for value in word:
    key = (key shl 2) or code(value)
  echo counts[index + 2].getOrDefault(key), "\t", word
