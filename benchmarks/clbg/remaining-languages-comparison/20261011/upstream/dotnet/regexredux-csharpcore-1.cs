/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 * 
 * regex-dna program contributed by Isaac Gouy 
*/

using System;
using System.Text.RegularExpressions;

class regexredux
{
   static void Main(string[] args){
                   
      // read FASTA sequence
      String sequence = Console.In.ReadToEnd();
      int initialLength = sequence.Length;

      // remove FASTA sequence descriptions and new-lines
      Regex r = new Regex(">.*\n|\n", RegexOptions.Compiled);
      sequence = r.Replace(sequence,"");
      int codeLength = sequence.Length;


      // regex match
      string[] variants = {
         "agggtaaa|tttaccct"
         ,"[cgt]gggtaaa|tttaccc[acg]"
         ,"a[act]ggtaaa|tttacc[agt]t"
         ,"ag[act]gtaaa|tttac[agt]ct"
         ,"agg[act]taaa|ttta[agt]cct"
         ,"aggg[acg]aaa|ttt[cgt]ccct"
         ,"agggt[cgt]aa|tt[acg]accct"
         ,"agggta[cgt]a|t[acg]taccct"
         ,"agggtaa[cgt]|[acg]ttaccct"
      }; 

      int count;
      foreach (string v in variants){
         count = 0;
         r = new Regex(v, RegexOptions.Compiled);

         for (Match m = r.Match(sequence); m.Success; m = m.NextMatch()) count++;
         Console.WriteLine("{0} {1}", v, count);
      }


      // regex substitution
      IUB[] codes = {
          new IUB("tHa[Nt]", "<4>")
         ,new IUB("aND|caN|Ha[DS]|WaS", "<3>")
         ,new IUB("a[NSt]|BY", "<2>")
         ,new IUB("<[^>]*>", "|")
         ,new IUB("\\|[^|][^|]*\\|" , "-")
      }; 

      foreach (IUB iub in codes) {
         r = new Regex(iub.code, RegexOptions.Compiled);
         sequence = r.Replace(sequence,iub.alternatives);
      }
      Console.WriteLine("\n{0}\n{1}\n{2}", 
         initialLength, codeLength, sequence.Length);
   }


   struct IUB 
   {
      public string code;
      public string alternatives;

      public IUB(string code, string alternatives) {
         this.code = code;
         this.alternatives = alternatives;
      }
   }
}
