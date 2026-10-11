/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Greg Buchholz's simple C #2 program buffered, by Isaac Gouy. 
*/

import 'dart:io';
import 'dart:typed_data';

void main(List<String> args) {
  int bit_num = 0, byte_acc = 0;
  const iter = 50, limit = 2.0;
  double zr, zi, cr, ci, tr, ti;

  final h = (args.length > 0) ? int.parse(args[0]) : 200, w = h;
  final bytesPerRow = w >> 3;

  stdout.write('P4\n$w $h\n');

  for (int y = 0; y < h; ++y) {
    var row = Uint8List(bytesPerRow);
    var xbyte = 0;

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

      byte_acc <<= 1;
      if (tr + ti <= limit * limit) byte_acc |= 0x01;
      ++bit_num;

      if (bit_num == 8) {
        row[xbyte++] = byte_acc;
        byte_acc = 0;
        bit_num = 0;
      } else if (x == w - 1) {
        byte_acc <<= (8 - w % 8);
        row[xbyte++] = byte_acc;
        byte_acc = 0;
        bit_num = 0;
      }
    }
    stdout.add(row);
  }
}
