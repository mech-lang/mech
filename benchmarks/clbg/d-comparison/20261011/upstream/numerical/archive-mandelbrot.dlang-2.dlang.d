/* The Great Computer Language Shootout
   http://shootout.alioth.debian.org/

   Contributed by Dave Fladebo
   compile: dmd -O -inline -release mandelbrot2.d
*/

import std.stdio, std.string;

void main(char[][] args)
{
  char bit_num = 0, byte_acc = 0;
  int n = args.length > 1 ? atoi(args[1]) : 1;
  const iter = 50;
  const lim = 2.0 * 2.0;

  writefln("P4\n%d %d",n,n);

  for(auto y=0; y<n; y++)
  {
    for(auto x=0; x<n; x++)
    {
       auto Z = 0 + 0i; auto C = 2*cast(double)x/n - 1.5 + 2i*cast(double)y/n - 1i;

       for(auto i = 0; i < iter && norm(Z) <= lim; i++)
         Z = Z*Z + C;

       byte_acc = (byte_acc << 1) | ((norm(Z) > lim) ? 0x00:0x01);

       bit_num++;
       if(bit_num == 8)
       {
           putc(byte_acc,stdout);
           bit_num = byte_acc = 0;
       }
       else if(x == n-1)
       {
           byte_acc  <<= (8-n%8);
           putc(byte_acc,stdout);
           bit_num = byte_acc = 0;
       }
    }
  }
}

double norm(cdouble C)
{
    return C.re*C.re + C.im*C.im;
}
