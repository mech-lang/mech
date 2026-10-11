/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * Contributed by Bogdan Sharkov 
 * Optimized memory access (AVX intrinsics), compile with -march=native -fopenmp for best results 
 *
 * Contributed by Mr Ledrug 
 * Algorithm lifted from Intel Fortran #2 code by Steve Decker et al.
*/

#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <malloc.h>
#include <x86intrin.h>

double dot(double * v, double * u, int n) {
   int i;
   double sum = 0;
   for (i = 0; i < n; i++)
      sum += *v++ * *u++;
   return sum;
}

void mult_Av(double * v, double * out, const int n) {
__m256i ONE = _mm256_set1_epi32(1);   
__m256i EIGHT = _mm256_set1_epi32(8); 
   unsigned int i, j;
   #pragma omp parallel for schedule(static)
   for (i = 1; i <= n; i++) {
	   
	   j = i-1;
	   int k = n+j;
	   __m256i m = _mm256_set1_epi32(i);
	   __m256d mysum =_mm256_setzero_pd();
	   __m256i x = _mm256_setr_epi32(j, j + 1, j + 2, j + 3, j + 4, j + 5, j + 6, j + 7);	   
		
		for (double* pv = v; j < k; j+=8, pv+=4) {
			__m256i y = _mm256_add_epi32(x, ONE);
			__m256i a = _mm256_mullo_epi32(x, y);
			x = _mm256_add_epi32(x, EIGHT);
			a = _mm256_srli_epi32(a, 1);
			a = _mm256_add_epi32(a, m);			
			
			 __m128i a1 = _mm256_castsi256_si128(a);
			 __m256d  mul = _mm256_cvtepi32_pd(a1);
			 __m256d  src = _mm256_load_pd(pv);
			 pv+=4;
			 __m256d  res = _mm256_div_pd(src, mul);	
			 mysum = _mm256_add_pd(mysum, res);			

			   a1 = _mm256_extracti128_si256(a, 1);
			   mul = _mm256_cvtepi32_pd(a1);
			   src = _mm256_load_pd(pv);
			 
			 res = _mm256_div_pd(src, mul);	
			 mysum = _mm256_add_pd(mysum, res);							
		}  
  
	  
  	  out[i-1] = ((double*)&mysum)[0] + ((double*)&mysum)[1] + ((double*)&mysum)[2] + ((double*)&mysum)[3];
   }
	for(i=n;i<n+7;i++) out[i] =0.0;
}

void mult_Atv(double * v, double * out,  const int n) {
	__m256i ONE = _mm256_set1_epi32(1);   
	__m256i EIGHT = _mm256_set1_epi32(8);
   int i;
   
#pragma omp parallel for schedule(static)
   for (i = 0; i < n; i++) {
	   double* pv = v;
	  __m256d mysum =_mm256_setzero_pd();
	   __m256i x = _mm256_setr_epi32(i, i + 1, i + 2, i + 3, i + 4, i + 5, i + 6, i + 7);
	  __m256i aj = _mm256_setr_epi32(1, 2, 3, 4, 5, 6, 7, 8);
	  for (int j=0; j < n;j+=8, pv+=4){		  
			__m256i y = _mm256_add_epi32(x, ONE);
			__m256i a = _mm256_mullo_epi32(x, y);
			x = _mm256_add_epi32(x, EIGHT);
			a = _mm256_srli_epi32(a, 1);
			a = _mm256_add_epi32(a, aj);
			aj = _mm256_add_epi32(aj, EIGHT);
			
			 __m128i a1 = _mm256_castsi256_si128(a);
			 __m256d  mul = _mm256_cvtepi32_pd(a1);
			 __m256d  src = _mm256_load_pd(pv);
			 pv+=4;
			 __m256d  res = _mm256_div_pd(src, mul);	
			 mysum = _mm256_add_pd(mysum, res);			

			 a1 = _mm256_extracti128_si256(a, 1);
			 mul = _mm256_cvtepi32_pd(a1);
			 src = _mm256_load_pd(pv);
			 
			 res = _mm256_div_pd(src, mul);	
			 mysum = _mm256_add_pd(mysum, res);
	  }		  
	  out[i] = ((double*)&mysum)[0] + ((double*)&mysum)[1] + ((double*)&mysum)[2] + ((double*)&mysum)[3];
   }
   
    for(i=n;i<n+7;i++) out[i]=0.0;
}

double *tmp;
void mult_AtAv(double *v, double *out, const int n) {
   mult_Av(v, tmp, n);
   mult_Atv(tmp, out, n);
}

int main(int argc, char**argv) {
   int n = 2000;
   if (argc >= 2) n = atoi(argv[1]);

   double *u, *v;
   u = _mm_malloc((n+7) * sizeof(double),sizeof(__m256d));
   v = _mm_malloc((n+7) * sizeof(double),sizeof(__m256d));
   tmp = _mm_malloc((n+7) * sizeof(double),sizeof(__m256d));
   
   int i;
   for (i = 0; i < n; i++) u[i] = 1;
   for(i=n;i<n+7;i++) u[i]=0.0;
	
   for (i = 0; i < 10; i++) {
      mult_AtAv(u, v, n);
      mult_AtAv(v, u, n);
   }

   printf("%.9f\n", sqrt(dot(u,v, n) / dot(v,v,n)));

	_mm_free(u);
	_mm_free(v);
	_mm_free(tmp);


   return 0; 
}
