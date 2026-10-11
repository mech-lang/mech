/*   
   The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Paolo Bonzini
   further optimized by Jason Garrett-Glaser
   OpenMP by The Anh Tran
   10-11-2010, modified by The Anh Tran:
      _ copy bit shift idea from C entry
   C version by Des Nerger
*/

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <sched.h>
#include <memory.h>

#include <omp.h>
#include <sys/types.h>

#define L2_CACHE_LINE   64
#define ALIGN         __attribute__ ((aligned(L2_CACHE_LINE)))


typedef unsigned char   byte;
typedef double         v2d   __attribute__ ((vector_size(16)));
typedef int32_t       v4i   __attribute__ ((vector_size(16)));

const v2d v10      = { 1.0, 1.0 };
const v2d v15      = { 1.5, 1.5 };
const v2d v40      = { 4.0, 4.0 };

v2d inv_2n;   // {2.0/N, 2.0/N}


int
GetThreadCount()
{
   cpu_set_t cs;
   CPU_ZERO(&cs);
   sched_getaffinity(0, sizeof(cs), &cs);

   int count = 0;
   for (int i = 0; i < CPU_SETSIZE; ++i)
      count += CPU_ISSET(i, &cs);

   return count;
}


void
mandelbrot(int N, byte* data)
{
   ALIGN int row_processed = 0;

   #pragma omp parallel default(shared) num_threads(GetThreadCount())
   {
      int y = 0;
      while ((y = __sync_fetch_and_add(&row_processed, 1)) < N)
      {
         byte* row_output = data + y * (N >> 3);
         
         v2d Civ = {y, y};
         Civ = Civ * inv_2n - v10;

            for (int x = 0; x < N; x += 2)
            {
            v2d   Crv = {x+1, x};
            Crv = Crv * inv_2n - v15;
            v2d Zrv = Crv;
            v2d Ziv = Civ;
            v2d Trv = Crv * Crv;
            v2d Tiv = Civ * Civ;

            int result = 3; // assume that 2 elements belong to MB set
            int i = 1;

            while ( result && (i++ < 50) )
            {
               v2d ZZ = Zrv * Ziv;
               Zrv = Trv - Tiv + Crv;
               Ziv = ZZ + ZZ + Civ;
               Trv = Zrv * Zrv;
               Tiv = Ziv * Ziv;
           
               // trv + tiv <= 4.0
               v2d delta = (v2d)__builtin_ia32_cmplepd( (Trv + Tiv), v40 );
               result = __builtin_ia32_movmskpd(delta);              
            }

            {
               int bit_shift = 6 - (x & 7);
               row_output[x >> 3] |= (byte)(result << bit_shift);
            }
         }
      }
   }
}


int
main (int argc, char **argv)
{
   const int N = (argc == 2) ? (int)strtol(argv[1], NULL, 10) : 200;
   assert((N % 8) == 0);
   printf("P4\n%d %d\n", N, N);

   {
      double* p_iv = (double*)(&inv_2n);
      p_iv[0] = p_iv[1] = 2.0 / N;
   }

   const int bytes_count = (N >> 3) * N;
   byte* data = 0;
   assert(   posix_memalign((void**)(&data), L2_CACHE_LINE, bytes_count)
         == 0);
   memset(data, 0, bytes_count);

   mandelbrot(N, data);

   fwrite( data, bytes_count, 1, stdout);
   fflush(stdout);
   free(data);

   return 0;
}
