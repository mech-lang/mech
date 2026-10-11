/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Isaac Gouy
   modified by Robert F. Tobler to perform actual computations on byte arrays
   modified again by Robert F. Tobler to use BinaryWriter with larger buffers
*/

using System;
using System.IO;
using System.Text;
using System.Collections.Generic;

class revcomp
{
   static byte[] ByteArray(string line)
   {
      byte[] array = new byte[line.Length];
      for (int i = 0; i < line.Length; i++)
         array[i] = (byte)line[i];
      return array;
   }

   const byte Gt = (byte)'>';

   static void Main(string[] args){
      InitializeComplements();

      List<byte[]> builder = new List<byte[]>();
      string line;
      byte[] array, desc = new byte[0];

      using (StreamReader r = new StreamReader(Console.OpenStandardInput())) {
         using (BinaryWriter w = new BinaryWriter(Console.OpenStandardOutput())) {
            while ((line = r.ReadLine()) != null) {
               array = ByteArray(line);
               byte c = array[0];
               if (c == Gt) {
                  if (desc.Length > 0) {
                     WriteReverseFasta(desc, builder, w);
                     builder.Clear();
                  }
                  desc = array;
               }
               else if (c != ';') {
                  builder.Add(array);
               }
            }
            if (builder.Count > 0) {
               WriteReverseFasta(desc, builder, w);
            }
         }
      }
   }

   static byte[] iubComp = new byte[256];
   static void InitializeComplements(){
      for (byte i = 0; i < 255; i++) iubComp[i] = i;
          
      iubComp[(byte)'A'] = (byte)'T'; iubComp[(byte)'a'] = (byte)'T';
      iubComp[(byte)'B'] = (byte)'V'; iubComp[(byte)'b'] = (byte)'V';
      iubComp[(byte)'C'] = (byte)'G'; iubComp[(byte)'c'] = (byte)'G';
      iubComp[(byte)'D'] = (byte)'H'; iubComp[(byte)'d'] = (byte)'H';
      iubComp[(byte)'G'] = (byte)'C'; iubComp[(byte)'g'] = (byte)'C';
      iubComp[(byte)'H'] = (byte)'D'; iubComp[(byte)'h'] = (byte)'D';
      iubComp[(byte)'K'] = (byte)'M'; iubComp[(byte)'k'] = (byte)'M';
      iubComp[(byte)'M'] = (byte)'K'; iubComp[(byte)'m'] = (byte)'K';
      iubComp[(byte)'R'] = (byte)'Y'; iubComp[(byte)'r'] = (byte)'Y';
      iubComp[(byte)'T'] = (byte)'A'; iubComp[(byte)'t'] = (byte)'A';
      iubComp[(byte)'V'] = (byte)'B'; iubComp[(byte)'v'] = (byte)'B';
      iubComp[(byte)'Y'] = (byte)'R'; iubComp[(byte)'y'] = (byte)'R';
   }

   const byte Lf = (byte)'\n';
   const int LineLength = 61;
   const int BufferSize = LineLength * 269; // about 16k
   static byte[] buffer = new byte[BufferSize];

   static void WriteReverseFasta(byte[] desc, List<byte[]> b, BinaryWriter w) {
      w.Write(desc); w.Write(Lf);
      int bi = 0;
      int trigger = LineLength - 1;
      for (int rli = b.Count-1; rli >= 0; rli--) {
         byte[] line = b[rli];
         for (int rci = line.Length-1; rci >= 0; rci--) {
            buffer[bi++] = iubComp[line[rci]];
            if (bi == trigger) {
               buffer[bi++] = Lf; trigger += LineLength;
               if (bi >= BufferSize) {
                  w.Write(buffer); bi = 0; trigger = LineLength - 1;
               }
            }
         }
      }
      if (bi > 0) {
          if (buffer[bi-1] != Lf) buffer[bi++] = Lf;
          w.Write(buffer, 0, bi);
      }
   }
}
