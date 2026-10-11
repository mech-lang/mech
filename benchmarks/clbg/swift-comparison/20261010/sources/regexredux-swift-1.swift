// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Francois Green


import Foundation
import Dispatch

let input = FileHandle.standardInput.readDataToEndOfFile()

var sequence = String(data: input, encoding: .utf8)!

let inputLength = input.count

let regex: (String) -> NSRegularExpression = { pattern in
  return try! NSRegularExpression(pattern: pattern, options: [])
}

sequence = regex(">[^\n]*\n|\n").stringByReplacingMatches(in: sequence, options: [], range: NSRange(location: 0, length: inputLength), withTemplate: "")

let codeLength = sequence.utf8.count

var resultLength: Int?

let group = DispatchGroup()

DispatchQueue.global(qos: .background).async {
  group.enter()
  resultLength = [
    (regex: "tHa[Nt]",            replacement: "<4>"),
    (regex: "aND|caN|Ha[DS]|WaS", replacement: "<3>"),
    (regex: "a[NSt]|BY",          replacement: "<2>"),
    (regex: "<[^>]*>",            replacement: "|"),
    (regex: "\\|[^|][^|]*\\|",    replacement: "-")
  ].reduce(sequence) { buffer, iub in
    return regex(iub.regex).stringByReplacingMatches(in: buffer, options: [], range: NSRange(location: 0, length: buffer.utf16.count), withTemplate: iub.replacement)
  }.utf8.count
  group.leave()
}

[
  "agggtaaa|tttaccct",
  "[cgt]gggtaaa|tttaccc[acg]",
  "a[act]ggtaaa|tttacc[agt]t",
  "ag[act]gtaaa|tttac[agt]ct",
  "agg[act]taaa|ttta[agt]cct",
  "aggg[acg]aaa|ttt[cgt]ccct",
  "agggt[cgt]aa|tt[acg]accct",
  "agggta[cgt]a|t[acg]taccct",
  "agggtaa[cgt]|[acg]ttaccct"
].forEach { variant in
  print(variant, regex(variant).numberOfMatches(in: sequence, options: [], range: NSRange(location: 0, length: sequence.utf8.count)))
}

group.wait()
print("", inputLength, codeLength, resultLength!, separator: "\n")
