/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   
   Naive transliteration from bearophile's program 
   contributed by Isaac Gouy
*/

import java.io.*;
import java.util.*;
import java.util.stream.*;

public final class knucleotide {

   static ArrayList<String> seqLines() throws IOException 
   {
      final var in = new BufferedReader(new InputStreamReader(System.in));
      String line;
      while ((line = in.readLine()) != null) {
         if (line.startsWith(">THREE")) break;
      }
      final var lines = new ArrayList<String>();
      while ((line = in.readLine()) != null) {
         if (line.startsWith(">")) break;
         lines.add(line);
      }    
      return lines;      
    }         

   static HashMap<String,Integer> baseCounts(int bases, String seq)
   {
      var counts = new HashMap<String,Integer>();  
      final int size = seq.length() + 1 - bases;     
      for (int i = 0; i < size; i++) {
         var nucleo = seq.substring(i,i+bases);
         Integer v;
         if ((v = counts.get(nucleo)) != null) {
            counts.put(nucleo, v+1);
         } else {
            counts.put(nucleo, 1);
         }                           
      }      
      return counts;
   }
   
   static List<String> sortedFreq(int bases, String seq)
   {
      final int size = seq.length() + 1 - bases;
      return baseCounts(bases, seq).entrySet()     
         .stream()
         .sorted(Map.Entry.comparingByValue(Comparator.reverseOrder()))
         .map(e -> String.format("%s %.3f", 
               e.getKey(), 100.0 * e.getValue() / size))
         .toList();
   }   
   
   static int specificCount(String code, String seq)
   {
      return baseCounts(code.length(),seq).getOrDefault(code,0);   
   }

   public static void main(String[] args) throws Exception 
   {
      final var seq = seqLines().stream()
         .map(s -> s.toUpperCase())
         .collect(Collectors.joining(""));
      
      for (int i : Arrays.asList(1,2)) {  
         for (String s : sortedFreq(i,seq)) {      
            System.out.println(s); 
         }
         System.out.println();          
      }
      
      for (String code : Arrays.asList("GGT", "GGTA", "GGTATT",
            "GGTATTTTAATT", "GGTATTTTAATTTATAGT")) {       
         System.out.println(specificCount(code, seq) + "\t" + code);                        
      }
   }
}
