/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Matthew McMullan
 * based on C source by Ledrug Katz
 *
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <cstring>
#include <algorithm>
#include <xmmintrin.h>
#include <tmmintrin.h>

/* this depends highly on the platform.  It might be faster to use
    char type on 32-bit systems; it might be faster to use unsigned. */

typedef char elem;

elem s[16] __attribute__ ((aligned (16)));

int maxflips = 0;
int max_n;
int odd = 0;
int checksum = 0;
// naieve method of rotation using basic sisd instructions for sanity's sake
inline void rotate_sisd(int n) {
   elem c;
   register int i;
   c = s[0];
   for (i = 1; i <= n; i++) s[i-1] = s[i];
   s[n] = c;
}
// flip and rotation masks needed to use SSE for rotations and flipping
// the number of these remains constant for all sizes
__m128i flip_masks[16];
__m128i rotate_masks[16];
__m128i MM_ITRUE;
// populate the data in the masks. could be hard coded. will never change.
void popmasks() {
   char mask[16];
   for (unsigned i = 0; i<16; ++i) {
      for (unsigned j = 0; j<16; ++j) mask[j] = j;
      // this is actually slower than a for loop for small arrays
      std::reverse(mask,mask+i+1);
      flip_masks[i] = _mm_loadu_si128((__m128i*)mask);
      
      for (unsigned j = 0; j<16; ++j) s[j] = j;
      rotate_sisd(i);
      rotate_masks[i] = _mm_load_si128((__m128i*)s);
   }
   char truth[16] = {0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0};
   MM_ITRUE = _mm_loadu_si128((__m128i*)truth);
}
inline void rotate(int n) {
   // use SSE to rotate the values
   // n could get as high as the max for the range,
   //   but only 16 constants will ever be needed
   _mm_store_si128((__m128i*)s,
      _mm_shuffle_epi8(_mm_load_si128((__m128i*)s),rotate_masks[n]));
}

void tk(int n) {
   // for flipping
   char tmp[16] __attribute__ ((aligned (16)));
   char tmp2[16] __attribute__ ((aligned (16)));
   // a place to put the backlog of permutations
   struct Perm {
      __m128i perm;
      elem start;
      short odd;
   } perms[60];

   int i = 0;
   elem c[16] = {0};
   int perm_max = 0;
   while (i < n) {
      /* Tompkin-Paige iterative perm generation */
      // fill the queue up to 60
      while (i<n && perm_max<60) {
         rotate(i);
         if (c[i] >= i) {
            c[i++] = 0;
            continue;
         }

         c[i]++;
         i = 1;
         odd = ~odd;
         if (*s) {
            if (s[(int)s[0]]) {
               perms[perm_max].perm = _mm_load_si128((__m128i*)s);
               perms[perm_max].start = *s;
               perms[perm_max].odd = odd;
               perm_max++;
            } else {
               if (maxflips==0) maxflips = 1;
               checksum += odd ? -1 : 1;
            }
         }
      }
      // process the queue
      int k;
      // do 2 at a time when possible to take advantage of pipelining
      // see the next loop for implementation logic
      for (k=0; k<perm_max-1; k+=2) {
         __m128i perm1 = perms[k].perm;
         __m128i perm2 = perms[k+1].perm;
         
         
         int f1 = 0, f2 = 0;
         int toterm1 = perms[k].start, toterm2 = perms[k+1].start;
         while (toterm1 && toterm2) {
            perm1 = _mm_shuffle_epi8(perm1,flip_masks[toterm1]);
            perm2 = _mm_shuffle_epi8(perm2,flip_masks[toterm2]);
            _mm_storel_epi64((__m128i*)tmp,perm1);
            _mm_storel_epi64((__m128i*)tmp2,perm2);
            toterm1 = tmp[0];
            toterm2 = tmp2[0];
            ++f1; ++f2;
         }
         while (toterm1) {
            perm1 = _mm_shuffle_epi8(perm1,flip_masks[toterm1]);
            _mm_storel_epi64((__m128i*)tmp,perm1);
            toterm1 = tmp[0];
            ++f1;
         }
         while (toterm2) {
            perm2 = _mm_shuffle_epi8(perm2,flip_masks[toterm2]);
            _mm_storel_epi64((__m128i*)tmp2,perm2);
            toterm2 = tmp2[0];
            ++f2;
         }
         
         if (f1 > maxflips) maxflips = f1;
         if (f2 > maxflips) maxflips = f2;
         checksum += perms[k].odd ? -f1 : f1;
         checksum += perms[k+1].odd ? -f2 : f2;
      }
      // finish up one at a time
      for (;k<perm_max;++k) {
         // get the data out of the structure
         // the whole array is packed into an sse integer type
         // we could use more fairly easily if we wanted to
         __m128i perm = perms[k].perm;
         int f = 0, toterm = perms[k].start;
         while (toterm) {
            // hardware support for reversing arbitrary subsequences
            perm = _mm_shuffle_epi8(perm,flip_masks[toterm]);
            // check the first number. this is ~1/3 of the execution time
            _mm_storel_epi64((__m128i*)tmp,perm);
            toterm = tmp[0];
            ++f;
         }
         
         if (f > maxflips) maxflips = f;
         checksum += perms[k].odd ? -f : f;
      }
      perm_max = 0;
   }
}

int main(int argc, char **v) {
   int i;
   popmasks();
   if (argc < 2) {
      fprintf(stderr, "usage: %s number\n", v[0]);
      exit(1);
   }

   max_n = atoi(v[1]);
   if (max_n < 3 || max_n > 15) {
      fprintf(stderr, "range: must be 3 <= n <= 12\n");
      exit(1);
   }

   for (i = 0; i < max_n; i++) s[i] = i;
   tk(max_n);
   printf("%d\nPfannkuchen(%d) = %d\n", checksum, max_n, maxflips);

   return 0;
}
