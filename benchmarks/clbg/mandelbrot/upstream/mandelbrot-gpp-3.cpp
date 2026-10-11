/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   contributed by Greg Buchholz
   compile:  g++ -O3
*/

#include <stdio.h>
#include <stdlib.h>
#include<iostream>
#include<complex>

int main (int argc, char **argv)
{
  char  bit_num = 0, byte_acc = 0;
  const int iter = 50;
  const double lim = 2.0 * 2.0;
  
  std::ios_base::sync_with_stdio(false);
  int n = atoi(argv[1]);

  std::cout << "P4\n" << n << " " << n << std::endl;

  for(int y=0; y<n; ++y) 
    for(int x=0; x<n; ++x)
    {
       std::complex<double> Z(0,0),C(2*(double)x/n - 1.5, 2*(double)y/n - 1.0);
       
       //manually inlining "norm" results in a 5x-7x speedup on gcc
       for(int i=0; i<iter and Z.real()*Z.real() + Z.imag()*Z.imag() <= lim; ++i)
         Z = Z*Z + C;
        
       byte_acc = (byte_acc << 1) | ((norm(Z) > lim) ? 0x00:0x01);

       if(++bit_num == 8){ std::cout << byte_acc; bit_num = byte_acc = 0; }
       else if(x == n-1) { byte_acc  <<= (8-n%8);
                           std::cout << byte_acc;
                           bit_num = byte_acc = 0; }
    }
}
