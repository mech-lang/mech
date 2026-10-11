/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   line-by-line from Greg Buchholz's C program 
*/

import 'dart:io';
import 'dart:typed_data';

void main(List<String> args) 
{
    int w, h, bit_num = 0;
    final byte_acc = Uint8List(1);
    var i; const iter = 50;
    var x, y; const limit = 2.0;    
    double Zr, Zi, Cr, Ci, Tr, Ti;

    h = (args.length > 0) ? int.parse(args[0]) : 200; w = h;

    stdout.write('P4\n$w $h\n');

    for (y = 0; y < h; ++y) 
    {
        for (x = 0; x < w; ++x) 
        {
            Zr = Zi = Tr = Ti = 0.0;
            Cr = 2.0*x/w - 1.5; Ci = 2.0*y/h - 1.0;
            
            for (i= 0; i<iter && (Tr+Ti <= limit*limit); ++i) 
            {
                Zi = 2.0*Zr*Zi + Ci;
                Zr = Tr - Ti + Cr;
                Tr = Zr * Zr;
                Ti = Zi * Zi;
            }

            byte_acc[0] <<= 1;
            if (Tr+Ti <= limit*limit) byte_acc[0] |= 0x01;

            ++bit_num;
            
            if (bit_num == 8) 
            {
                stdout.add(byte_acc); 
                byte_acc[0] = 0;
                bit_num = 0;
            } 
            else if (x == w-1) 
            {
                byte_acc[0] <<= (8-w%8);   
                stdout.add(byte_acc);                 
                byte_acc[0] = 0;
                bit_num = 0;
            }
        }
    }
}
