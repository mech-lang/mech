import std/[strutils, tables, algorithm]

const lengths = [1, 2, 3, 4, 6, 12, 18]
const queries = ["GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]
type Entry = tuple[word: string, count: int64]

proc thirdRecord(): string =
  var selected = false
  for line in stdin.readAll().splitLines():
    if line.startsWith(";"):
      continue
    if line.startsWith(">"):
      if selected:
        break
      selected = line == ">THREE" or line.startsWith(">THREE ") or
          line.startsWith(">THREE\t")
    elif selected:
      result.add(toUpperAscii(line))

proc compare(left, right: Entry): int =
  if left.count != right.count:
    cmp(right.count, left.count)
  else:
    cmp(left.word, right.word)

let sequence = thirdRecord()
var counts: array[7, Table[string, int64]]
for index, width in lengths:
  counts[index] = initTable[string, int64]()
  for start in 0 ..< max(0, sequence.len - width + 1):
    let word = sequence[start ..< start + width]
    counts[index][word] = counts[index].getOrDefault(word) + 1
for index in 0 ..< 2:
  var rows: seq[Entry]
  var total = 0'i64
  for word, count in counts[index]:
    rows.add((word, count))
    total += count
  rows.sort(compare)
  for row in rows:
    echo row.word, " ", formatFloat(100.0 * float64(row.count) / float64(total),
        ffDecimal, 3)
  echo ""
for index, word in queries:
  echo counts[index + 2].getOrDefault(word), "\t", word
