/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Greg Buchholz's simple C #2 program transliterated, by Isaac Gouy. 
   
   stdout NOT BUFFERED   
*/

import 'dart:io';
import 'dart:typed_data';

void main(List<String> args) {
  int bit_num = 0; 
  const iter = 50, limit = 2.0;
  double zr, zi, cr, ci, tr, ti;

  final h = (args.length > 0) ? int.parse(args[0]) : 200, w = h;
  final byte_acc = Uint8List(1);

  stdout.write('P4\n$w $h\n');

  for (int y = 0; y < h; ++y) {



    for (int x = 0; x < w; ++x) {
      zr = zi = tr = ti = 0.0;
      cr = 2.0 * x / w - 1.5;
      ci = 2.0 * y / h - 1.0;
      for (int i = 0; i < iter && (tr + ti <= limit * limit); ++i) {
        zi = 2.0 * zr * zi + ci;
        zr = tr - ti + cr;
        tr = zr * zr;
        ti = zi * zi;
      }

      byte_acc[0] <<= 1;
      if (tr + ti <= limit * limit) byte_acc[0] |= 0x01;
      ++bit_num;

      if (bit_num == 8) {
        stdout.add(byte_acc);
        byte_acc[0] = 0;
        bit_num = 0;
      } else if (x == w - 1) {
        byte_acc[0] <<= (8 - w % 8);
        stdout.add(byte_acc);
        byte_acc[0] = 0;
        bit_num = 0;
      }
    }

  }
}
