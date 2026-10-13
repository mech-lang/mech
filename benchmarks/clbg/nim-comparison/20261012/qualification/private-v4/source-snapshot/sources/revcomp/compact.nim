import std/[strutils, algorithm]

const symbols = "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy"
const complements = "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"
var complement: array[256, char]
for index, value in symbols:
  complement[ord(value)] = complements[index]

proc output(sequence: var string) =
  sequence.reverse()
  var start = 0
  while start < sequence.len:
    stdout.write(sequence[start ..< min(start + 60, sequence.len)], "\n")
    start += 60

var sequence = ""
for line in stdin.readAll().splitLines():
  if line.startsWith(">"):
    output(sequence)
    sequence.setLen(0)
    echo line
  elif not line.startsWith(";"):
    for value in line:
      sequence.add(complement[ord(value)])
output(sequence)
