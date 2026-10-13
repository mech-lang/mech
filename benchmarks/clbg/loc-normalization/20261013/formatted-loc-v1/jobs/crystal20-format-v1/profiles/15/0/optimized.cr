PATTERNS = [
  /agggtaaa|tttaccct/,
  /[cgt]gggtaaa|tttaccc[acg]/,
  /a[act]ggtaaa|tttacc[agt]t/,
  /ag[act]gtaaa|tttac[agt]ct/,
  /agg[act]taaa|ttta[agt]cct/,
  /aggg[acg]aaa|ttt[cgt]ccct/,
  /agggt[cgt]aa|tt[acg]accct/,
  /agggta[cgt]a|t[acg]taccct/,
  /agggtaa[cgt]|[acg]ttaccct/,
]
SUBSTITUTIONS = [
  {/tHa[Nt]/, "<4>"},
  {/aND|caN|Ha[DS]|WaS/, "<3>"},
  {/a[NSt]|BY/, "<2>"},
  {/<[^>]*>/, "|"},
  {/\|[^|][^|]*\|/, "-"},
]

def count_matches(sequence : String, pattern : Regex)
  count = 0
  position = 0
  while match = pattern.match(sequence, position)
    count += 1
    position = match.end
  end
  count
end

input = STDIN.gets_to_end
original_length = input.bytesize
sequence = input.gsub(/>[^\n]*\n|\n/, "")
cleaned_length = sequence.bytesize

def count_job(sequence : String, source : String, index : Int32, done : Channel(Tuple(Int32, Int32)))
  pattern = Regex.new(source)
  done.send({index, count_matches(sequence, pattern)})
end

Fiber::ExecutionContext.default.resize(8)
done = Channel(Tuple(Int32, Int32)).new(8)
8.times { |index| spawn count_job(sequence, PATTERNS[index].source, index, done) }
counts = Array.new(9, 0)
8.times do
  record = done.receive
  counts[record[0]] = record[1]
end
counts[8] = count_matches(sequence, PATTERNS[8])
PATTERNS.each_with_index { |pattern, index| puts "#{pattern.source} #{counts[index]}" }

SUBSTITUTIONS.each { |pair| sequence = sequence.gsub(pair[0], pair[1]) }
puts ""
puts original_length
puts cleaned_length
puts sequence.bytesize
