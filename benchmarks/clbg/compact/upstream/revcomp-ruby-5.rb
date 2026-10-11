# The Computer Language Benchmarks Game
# http://benchmarksgame.alioth.debian.org
#
# Contributed by Aaron Tavistock

LEADER = ">".freeze
CR = "\n".freeze

STDIN.each_line(LEADER).lazy.each do |chunk|
  header, segment = chunk.split(CR, 2)
  next unless segment

  segment.delete!(CR+LEADER)
  segment.reverse!.tr!('wsatugcyrkmbdhvnATUGCYRKMBDHVN','WSTAACGRYMKVHDBNTAACGRYMKVHDBN')

  STDOUT.write LEADER, header, CR
  segment_size = segment.size
  idx=0
  while idx < segment_size do
    STDOUT.write segment.byteslice(idx,60), CR
    idx += 60
  end
end
