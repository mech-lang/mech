import std/strutils

const symbols = "ACBDGHKMNSRUTWVYacbdghkmnsrutwvy"
const complements = "TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR"
var complement: array[256, char]
for index, value in symbols:
  complement[ord(value)] = complements[index]

proc output(sequence: var string) =
  var left = 0
  var right = sequence.len - 1
  while left <= right:
    let first = complement[ord(sequence[left])]
    sequence[left] = complement[ord(sequence[right])]
    sequence[right] = first
    inc left
    dec right
  var buffer = newStringOfCap(64 * 1024)
  var start = 0
  while start < sequence.len:
    buffer.add(sequence[start ..< min(start + 60, sequence.len)])
    buffer.add('\n')
    start += 60
    if buffer.len >= 60 * 1024:
      stdout.write(buffer)
      buffer.setLen(0)
  stdout.write(buffer)

var sequence = newStringOfCap(64 * 1024)
for line in stdin.readAll().splitLines():
  if line.startsWith(">"):
    output(sequence)
    sequence.setLen(0)
    echo line
  elif not line.startsWith(";"):
    sequence.add(line)
output(sequence)
