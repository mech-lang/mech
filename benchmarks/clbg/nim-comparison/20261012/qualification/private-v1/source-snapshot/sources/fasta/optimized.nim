import std/[os, strutils]

const alu = "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
type Probability = tuple[symbol: char, probability: float64]

proc cumulative(input: openArray[Probability]): seq[Probability] =
  var total = 0.0
  for entry in input:
    total += entry.probability
    result.add((entry.symbol, total))

let iub = cumulative([
  ('a', 0.27),
  ('c', 0.12),
  ('g', 0.12),
  ('t', 0.27),
  ('B', 0.02),
  ('D', 0.02),
  ('H', 0.02),
  ('K', 0.02),
  ('M', 0.02),
  ('N', 0.02),
  ('R', 0.02),
  ('S', 0.02),
  ('V', 0.02),
  ('W', 0.02),
  ('Y', 0.02)])
let homo = cumulative([
  ('a', 0.3029549426680),
  ('c', 0.1979883004921),
  ('g', 0.1975473066391),
  ('t', 0.3015094502008)])
var seed = 42

proc randomBase(cdf: openArray[Probability]): char {.inline.} =
  seed = (seed * 3877 + 29573) mod 139968
  let draw = float64(seed) / 139968.0
  for entry in cdf:
    if draw < entry.probability:
      return entry.symbol
  return cdf[^1].symbol

proc repeatSequence(title: string, length: int) =
  stdout.write(title)
  var buffer = newStringOfCap(64 * 1024)
  var column = 0
  var position = 0
  for index in 0 ..< length:
    buffer.add(alu[position])
    inc position
    if position == alu.len:
      position = 0
    inc column
    if column == 60 or index + 1 == length:
      buffer.add('\n')
      column = 0
    if buffer.len >= 60 * 1024:
      stdout.write(buffer)
      buffer.setLen(0)
  stdout.write(buffer)

proc randomSequence(title: string, cdf: openArray[Probability], length: int) =
  stdout.write(title)
  var buffer = newStringOfCap(64 * 1024)
  var column = 0
  for index in 0 ..< length:
    buffer.add(randomBase(cdf))
    inc column
    if column == 60 or index + 1 == length:
      buffer.add('\n')
      column = 0
    if buffer.len >= 60 * 1024:
      stdout.write(buffer)
      buffer.setLen(0)
  stdout.write(buffer)

let n = parseInt(paramStr(1))
doAssert n >= 0
repeatSequence(">ONE Homo sapiens alu\n", 2 * n)
randomSequence(">TWO IUB ambiguity codes\n", iub, 3 * n)
randomSequence(">THREE Homo sapiens frequency\n", homo, 5 * n)
