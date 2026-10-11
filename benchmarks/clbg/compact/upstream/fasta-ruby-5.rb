# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
# Contributed by Sokolov Yura
# Modified by Rick Branson, Andy Fingerhut

$last = 42.0

GR_IM = 139968.0
GR_IA = 3877.0
GR_IC = 29573.0

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

def generate_rand_finder(tbl)
  rb = "lambda do |n| \n"

  tbl.each do |va, vb|
    rb += "return #{va.inspect} if #{vb.inspect} > n\n"
  end

  rb += "end\n"

  eval rb
end

def make_repeat_fasta(id, desc, src, n)
    puts ">#{id} #{desc}"
    v = nil
    width = 60
    l = src.length
    s = src * (((width + l - 1) / l) + 1)
    i = 0
    p = []
    p[i] = s.slice(i,width)
    i = (i + width) % l
    while i != 0 do
      p[i] = s[i,width]
      i = (i + width) % l
    end
    i = 0
    printed = 0
    while printed <= (n - width) do
      puts "#{p[i]}"
      printed += width
      i = (i + width) % l
    end
    if printed < n
      puts "#{p[i].slice(0, n-printed)}"
    end
end

def make_random_fasta(id, desc, table, n)
    puts ">#{id} #{desc}"
    rand, v = nil,nil
    width = 60
    chunk = 1 * width
    prob = 0.0
    rwidth = (1..width)
    table.each{|v| v[1]= (prob += v[1])}
    f = generate_rand_finder(table)

    if RUBY_PLATFORM == "java"
      collector = lambda do |x|
        rand = ($last = ($last * GR_IA + GR_IC) % GR_IM) / GR_IM
        table.find { |va, vb| vb > rand }[0]
      end
    else
      collector = lambda do |x|
        rand = ($last = ($last * GR_IA + GR_IC) % GR_IM) / GR_IM
        f.call(rand)
      end
    end

    for i in 1..(n/width)
      puts rwidth.collect(&collector).join
    end
    if n%width != 0
      puts (1..(n%width)).collect(&collector).join
    end
end


n = (ARGV[0] or 27).to_i

make_repeat_fasta('ONE', 'Homo sapiens alu', alu, n*2)
make_random_fasta('TWO', 'IUB ambiguity codes', iub, n*3)
make_random_fasta('THREE', 'Homo sapiens frequency', homosapiens, n*5)
