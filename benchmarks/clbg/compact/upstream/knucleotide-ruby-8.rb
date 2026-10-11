# The Computer Language Benchmarks Game
#   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
#   Naive transliteration from bearophile's program
#   contributed by Isaac Gouy 

def seq_lines
   lines = Array.new
   loop do
      line = $stdin.gets()
      break if line.nil? | line&.start_with?(">THREE")
   end 
   loop do
      line = $stdin.gets()
      break if line.nil? | line&.start_with?(">")
      lines << line.chomp   
   end
   return lines
end

def base_counts(bases, seq)  
   counts = {}
   size = seq.length + 1 - bases
   for i in 0 ... size  
      nucleo = seq[i...i+bases]      
      if counts.key?(nucleo)  
         counts[nucleo] += 1  
      else
         counts[nucleo] = 1
      end      
   end
   return counts   
end

def sorted_freq(bases, seq)
   kv_ = base_counts(bases, seq)
   size = seq.length + 1 - bases  
   sorted_ = kv_.to_a.sort {|a,b| b.last <=> a.last} 
   return sorted_.collect {|a| [a.first, 100.0 * a.last  / size]}   
end

def specific_count(code, seq)  
    return base_counts(code.length, seq)[code] 
end

def main
   lines = seq_lines
   seq = lines.map {|s| s.upcase!}.join
   
   for base in 1..2
      for kv in sorted_freq(base, seq)
         puts "%s %.3f" % [kv[0], kv[1]]
      end
      puts
   end
   
   for code in ["GGT", "GGTA", "GGTATT",
         "GGTATTTTAATT", "GGTATTTTAATTTATAGT"]    
      puts "#{specific_count(code, seq).to_s}\t#{code}"
   end
end

if __FILE__ == $0
   main
end
