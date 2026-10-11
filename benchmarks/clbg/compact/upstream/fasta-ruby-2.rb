# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
# Contributed by Sokolov Yura
# Modified by Joseph LaFata

$last = 42.0
IM=139968
IA=3877
IC=29573
def gen_random (max)
    (max * ($last = ($last * IA + IC) % IM)) / IM
end

alu =
   "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG"+
   "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA"+
   "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT"+
   "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA"+
   "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG"+
   "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"+
   "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"

iub = [
    ["a", 0.27],
    ["c", 0.12],
    ["g", 0.12],
    ["t", 0.27],

    ["B", 0.02],
    ["D", 0.02],
    ["H", 0.02],
    ["K", 0.02],
    ["M", 0.02],
    ["N", 0.02],
    ["R", 0.02],
    ["S", 0.02],
    ["V", 0.02],
    ["W", 0.02],
    ["Y", 0.02],
]
homosapiens = [
    ["a", 0.3029549426680],
    ["c", 0.1979883004921],
    ["g", 0.1975473066391],
    ["t", 0.3015094502008],
]

def make_repeat_fasta(id, desc, src, n)
    puts ">#{id} #{desc}"
    l = src.length
    s = src * ((n / l) + 1)
    s.slice!(n, l)
    0.step(s.length-1,60) {|x| print s[x,60] , "\n"}
end

def make_random_fasta(id, desc, table, n)
    puts ">#{id} #{desc}"
    rand, v = nil,nil
    prob = 0.0
    table.each{|v| v[1]= (prob += v[1])}
    output = ""
    n.times do
      rand = gen_random(1.0)
      table.each do |v|
	if v[1] > rand then
	  output << v[0]
	  break
	end
      end
    end
    0.step(output.length-1,60) {|x| print output[x,60] , "\n"}
end


n = (ARGV[0] or 27).to_i

make_repeat_fasta('ONE', 'Homo sapiens alu', alu, n*2)
make_random_fasta('TWO', 'IUB ambiguity codes', iub, n*3)
make_random_fasta('THREE', 'Homo sapiens frequency', homosapiens, n*5)
