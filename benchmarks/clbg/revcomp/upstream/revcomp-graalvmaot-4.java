/*
 * The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 * contributed by Anthony Donnefort
 * slightly modified to read 82 bytes at a time by Razii
 */

import java.io.*;
public class revcomp {
   static final byte[] cmp = new byte[128];
   static {
      for (int i = 0; i < cmp.length; i++) cmp[i] = (byte) i;
      cmp['t'] = cmp['T'] = 'A';
      cmp['a'] = cmp['A'] = 'T';
      cmp['g'] = cmp['G'] = 'C';
      cmp['c'] = cmp['C'] = 'G';
      cmp['v'] = cmp['V'] = 'B';
      cmp['h'] = cmp['H'] = 'D';
      cmp['r'] = cmp['R'] = 'Y';
      cmp['m'] = cmp['M'] = 'K';
      cmp['y'] = cmp['Y'] = 'R';
      cmp['k'] = cmp['K'] = 'M';
      cmp['b'] = cmp['B'] = 'V';
      cmp['d'] = cmp['D'] = 'H';
      cmp['u'] = cmp['U'] = 'A';
   }

   static class ReversibleByteArray extends java.io.ByteArrayOutputStream {
      void reverse() throws Exception {
         if (count > 0) {
            int begin = 0, end = count - 1;
            while (buf[begin++] != '\n');
            while (begin <= end) {
               if (buf[begin] == '\n') begin++;
               if (buf[end] == '\n') end--;
               if (begin <= end) {
                  byte tmp = buf[begin];
                  buf[begin++] = cmp[buf[end]];
                  buf[end--] = cmp[tmp];
               }
            }
            System.out.write(buf, 0, count);
         }
      }
   }

   public static void main(String[] args) throws Exception {
      byte[] line = new byte[82];
      int read;
      ReversibleByteArray buf = new ReversibleByteArray();
      while ((read = System.in.read(line)) != -1) {
         int i = 0, last = 0;
         while (i < read) {
            if (line[i] == '>') {
               buf.write(line, last, i - last);
               buf.reverse();
               buf.reset();
               last = i;
            }
            i++;
         }
         buf.write(line, last, read - last);
      }
      buf.reverse();
   }
}
