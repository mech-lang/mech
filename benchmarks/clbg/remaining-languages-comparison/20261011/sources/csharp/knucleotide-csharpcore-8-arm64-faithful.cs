/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from bearophile's program 
   contributed by Isaac Gouy
*/



using System.Collections.Generic;

class Knucleotide {

   static List<String> SeqLines()
   {
      string line;
      while ((line = Console.ReadLine()) != null) {      
         if (line.StartsWith(">THREE")) break;
      }         
      var lines = new List<string>();
      while ((line = Console.ReadLine()) != null) {      
         if (line.StartsWith(">")) break;
         lines.Add(line);         
      } 
      return lines;
    }   
          
          
    static Dictionary<string,int> BaseCounts(int bases, string seq)
    {     
        var counts = new Dictionary<string,int>();   
        int size = seq.Count() + 1 - bases;  
        for (int i = 0; i < size; i++) {  
            var nucleo = seq.Substring(i,bases);        
            int v;     
            if (counts.TryGetValue(nucleo, out v)) {            
                counts[nucleo] = v+1;
            } else {
                counts.Add(nucleo, 1);
            }            
        }    
        return counts;
    }   
    
   static List<KeyValuePair<string,double>> SortedFreq(int bases, string seq)
    {    
        int size = seq.Count() + 1 - bases;   
        var counts = BaseCounts(bases, seq).ToList();
        counts.Sort((a, b) => b.Value != a.Value ? b.Value.CompareTo(a.Value) : StringComparer.Ordinal.Compare(a.Key, b.Key));
        var freqs = new List<KeyValuePair<string,double>>();
        foreach (var kvp in counts) {
            freqs.Add( new KeyValuePair<string,double>(
                    kvp.Key, 100.0 * kvp.Value / size) );             
        }
        return freqs;          
    } 
    
    static int SpecificCount(string code, string seq)
    {        
         int v; BaseCounts(code.Count(),seq).TryGetValue(code, out v); 
         return v;        
    }    

    public static void Main(String[] args) 
    {           
         var seq = string.Join( "", 
             SeqLines()
             .Select(s => s.ToUpper()) );           

        foreach (int i in new []{1,2}) {   
            foreach (KeyValuePair<string,double> kvp in SortedFreq(i,seq)) {         
                Console.WriteLine("{0} {1:f3}", kvp.Key, kvp.Value);
            }
            Console.WriteLine("");            
        }             
        
        foreach (string code in new []{"GGT", "GGTA", "GGTATT",
                "GGTATTTTAATT", "GGTATTTTAATTTATAGT"}) {          
        
            Console.WriteLine("{0}\t{1}", SpecificCount(code,seq), code);        
        }        
    }
}
