/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 
   Naive transliteration from bearophile's program 
   contributed by Isaac Gouy
*/

package main

import (
   "bufio"
   "fmt" 
   "os"
   "sort"
   "strings"
)

func seqLines() []string {
   in := bufio.NewScanner(os.Stdin)
   for in.Scan() {
      line := in.Text() 	 
      if strings.HasPrefix(line,">THREE") { break }     
   } 
   lines := []string{}
   for in.Scan() {
      line := in.Text() 	 
      if strings.HasPrefix(line,">") { break }    
      lines = append(lines,line)
   }   
   return lines
}

func baseCounts(bases int, seq string) map[string]int {
   counts := make(map[string]int)
   size := len(seq) + 1 - bases
   for i := 0; i < size; i++ {   
      nucleo := seq[i : i + bases]
      v, ok := counts[nucleo]
      if ok {
          counts[nucleo] = v+1      
      } else {
          counts[nucleo] = 1
      }
   }      
   return counts   
}

   type data struct {
      Key   string
      Count int
      Percent float64      
   }

func sortedFreq(bases int, seq string) []data {
   counts := []data{}
   for k, v := range baseCounts(bases, seq) {
      counts = append(counts, data{k,v,0.0})
   } 
   
   sort.Slice(counts, func(i,j int) bool {
         return counts[i].Count > counts[j].Count
   })
   
   size := len(seq) + 1 - bases   
   freqs := []data{}
   for _,e := range counts {    
      freqs = append(freqs, data{
            e.Key, 
            e.Count, 
            100.0 * float64(e.Count) / float64(size) })   
   }
   return freqs
}

func specificCount(code string, seq string) int {  
   return baseCounts(len(code),seq)[code]
}

func main() {
   lines := seqLines()
   var upper []string
   for _,s := range lines {   
      upper = append(upper,strings.ToUpper(s))   
   }
   seq := strings.Join(upper, "")

   for _, base := range []int{1,2} {
      for _,e := range sortedFreq(base,seq) {    
         fmt.Printf("%s %.3f\n", e.Key, e.Percent)
      }
      fmt.Printf("\n")      
   }     	
   	
   for _, code := range []string{"GGT", "GGTA", "GGTATT",
            "GGTATTTTAATT", "GGTATTTTAATTTATAGT"} {   
      fmt.Printf("%d\t%s\n", specificCount(code,seq), code)            
   }
}
