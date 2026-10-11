# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by Peter Bjarke Olsen
# Modified by Doug King
# Modified by Joseph LaFata

seq=""

def revcomp(seq)
  seq.reverse!.tr!('wsatugcyrkmbdhvnATUGCYRKMBDHVN','WSTAACGRYMKVHDBNTAACGRYMKVHDBN')
  stringlen=seq.length-1
  0.step(stringlen,60) {|x| print seq[x,60] , "\n"}
end

STDIN.each do |line|
  if line.include? '>'
    if !seq.empty?
      revcomp(seq)
      seq=""
    end
    puts line
  else
    line.chomp!
    seq << line
  end
end
revcomp(seq)
