/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  contributed by Paolo Bonzini
  further optimized by Jason Garrett-Glaser
  pthreads added by Eckehard Berns
  further optimized by Ryan Henszey
  modified by Samy Al Bahra (use GCC atomic builtins)
  modified by Kenneth Jonsson
*/

#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef double v2df __attribute__ ((vector_size(16))); /* vector of two doubles */
typedef int v4si __attribute__ ((vector_size(16))); /* vector of four ints */

/* 3 workers + the main thread to get a total of 4 active threads */
#define NWORKERS 3

int N;
int bytes_per_row;
double inverse_w;
double inverse_h;
const v2df zero = { 0.0, 0.0 };
const v2df four = { 4.0, 4.0 };

uint8_t *bitmap;
int next_y = 0;

static void * worker(void *_args) {
    uint8_t *row_bitmap;
    int x, y;

    for (;;) {
        y = __sync_fetch_and_add(&next_y, 1);
        if (y >= N)
            return NULL;
        row_bitmap = bitmap + (bytes_per_row * y);

        for (x=0; x<N; x+=2)
        {
            v2df Crv = { (x+1)*inverse_w-1.5, (x)*inverse_w-1.5 };
            v2df Civ = { y*inverse_h-1.0, y*inverse_h-1.0 };
            v2df Zrv = zero;
            v2df Ziv = zero;
            v2df Trv = zero;
            v2df Tiv = zero;
            int i = 0;
            int two_pixels;

            do {
                Ziv = (Zrv*Ziv) + (Zrv*Ziv) + Civ;
                Zrv = Trv - Tiv + Crv;
                Trv = Zrv * Zrv;
                Tiv = Ziv * Ziv;

                /* from mandelbrot C++ GNU g++ #5 program  */
                v2df delta = (v2df)__builtin_ia32_cmplepd( (Trv + Tiv), four);
                two_pixels = __builtin_ia32_movmskpd(delta);
            } while (++i < 50 && two_pixels);

            /*
             * The pixel bits must be in the most and second most
             * significant position
             */
            two_pixels <<= 6;

            /*
             * Add the two pixels to the bitmap, all bits are
             * initially zero since the area was allocated with
             * calloc()
             */
            row_bitmap[x >> 3] |= (uint8_t) (two_pixels >> (x & 7));
        }
    }
}

int main (int argc, char **argv)
{
    pthread_t ids[NWORKERS];
    int i;

    N = atoi(argv[1]);
    bytes_per_row = (N + 7) >> 3;

    inverse_w = 2.0 / (bytes_per_row << 3);
    inverse_h = 2.0 / N;

    bitmap = calloc(bytes_per_row, N);

    for (i = 0; i < NWORKERS; i++)
        pthread_create(&ids[i], NULL, worker, NULL);
    worker(NULL);
    for (i = 0; i < NWORKERS; i++)
        pthread_join(ids[i], NULL);

    printf("P4\n%d %d\n", N, N);
    fwrite(bitmap, bytes_per_row, N, stdout);
    free(bitmap);
    return 0;
}
