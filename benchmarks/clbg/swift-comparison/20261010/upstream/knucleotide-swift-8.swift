/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from bearophile's program
   contributed by Isaac Gouy 
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

func base_counts(_ bases: Int, _ seq: String.UTF8View) -> [String: Int] {
   var counts: [String: Int] = [:]
   let size = seq.count + 1 - bases  
   var i = 0
   while i < size {
      let start_offset = seq.index( seq.startIndex, offsetBy: i)
      let end = seq.index( start_offset, offsetBy: bases)         
      if let nucleo = String( seq[start_offset..<end] ) {
         if let v = counts[nucleo] {   
            counts[nucleo] = v + 1  
         } else {
            counts[nucleo] = 1 
         }
      } 
      i += 1      
   }
   return counts   
}

func sorted_freq(_ bases: Int, _ seq: String.UTF8View) -> [(String, Double)] {
   let kv_ = base_counts(bases, seq)
   let size = Double(seq.count + 1 - bases)  
   let sorted_ = kv_.sorted{ $1.1 < $0.1 }
   return sorted_.map{ ($0.0, 100.0 * Double($0.1) / size) }
}

func specific_count(_ code: String, _ seq: String.UTF8View) -> Int {  
    return base_counts(code.count, seq)[code] ?? 0 
}    

func main() {
   let lines = seq_lines()
   let seq = lines.map{$0.uppercased()}.joined().utf8
  
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
