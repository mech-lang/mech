# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# regex-dna program contributed by jose fco. gonzalez
# corrected to use regex instead of string substitution
# array not dictionary by Isaac Gouy

seq = STDIN.readlines.join
ilen = seq.size

seq.gsub!(/>.*\n|\n/,"")
clen = seq.length

[
  /agggtaaa|tttaccct/i,
  /[cgt]gggtaaa|tttaccc[acg]/i,
  /a[act]ggtaaa|tttacc[agt]t/i,
  /ag[act]gtaaa|tttac[agt]ct/i,
  /agg[act]taaa|ttta[agt]cct/i,
  /aggg[acg]aaa|ttt[cgt]ccct/i,
  /agggt[cgt]aa|tt[acg]accct/i,
  /agggta[cgt]a|t[acg]taccct/i,
  /agggtaa[cgt]|[acg]ttaccct/i
].each {|f| puts "#{f.source} #{seq.scan(f).size}" }

# ruby 1.8.7: to iterate in-order use array not dictionary
[
[/tHa[Nt]/, '<4>'], [/aND|caN|Ha[DS]|WaS/, '<3>'], [/a[NSt]|BY/, '<2>'], 
[/<[^>]*>/, '|'], [/\|[^|][^|]*\|/, '-']
].each { |f,r| seq.gsub!(f,r) }

puts
puts ilen
puts clen
puts seq.length
