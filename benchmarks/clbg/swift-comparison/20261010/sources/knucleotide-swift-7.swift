/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from bearophile's program
   contributed by Isaac Gouy    
   (indices usage by Kedar Sukerkar SO)
*/

import Foundation

func seq_lines() -> [String] {
   while let line = readLine() {   
      if (line.hasPrefix(">THREE")) { break }     
   }  
   var lines: [String] = []
   while let line = readLine() {   
      if (line.hasPrefix(">")) { break }   
      lines.append(line)
   } 
   return lines
}

func base_counts(_ bases: Int, _ seq: String) -> [String: Int] {
   var counts: [String: Int] = [:]   
   for i in seq.indices.dropLast(bases - 1) { 
      let nucleo = String(seq[i..<seq.index(i, offsetBy: bases)])      
      if let v = counts[nucleo] {   
         counts[nucleo] = v + 1  
      } else {
         counts[nucleo] = 1 
      }            
   }       
   return counts   
}

func sorted_freq(_ bases: Int, _ seq: String) -> [(String, Double)] {
   let kv_ = base_counts(bases, seq)
   let size = Double(seq.count + 1 - bases)  
   let sorted_ = kv_.sorted{ $1.1 < $0.1 }
   return sorted_.map{ ($0.0, 100.0 * Double($0.1) / size) }
}

func specific_count(_ code: String, _ seq: String) -> Int {  
    return base_counts(code.count, seq)[code] ?? 0 
}    

func main() {
   let lines = seq_lines()
   let seq = lines.map{$0.uppercased()}.joined()
  
   for base in [1,2] {
      for (k,v) in sorted_freq(base, seq) {   
         print( k, String(format: "%.3f", v))          
      }
      print()      
   }
   
   for code in ["GGT", "GGTA", "GGTATT",
         "GGTATTTTAATT", "GGTATTTTAATTTATAGT"] {    
      print("\(specific_count(code, seq))\t\(code)")         
   }
}

main()
