/* Generic grid indexing, worker scheduling and PBM serialization only. */
#include <errno.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#if MAP_PACKED_BITS
uint64_t mech_map_bits(const double *input);
#else
void mech_map_batch(const double *input, double *output);
#endif
double mech_fused_pow(double x, double y);
#include <math.h>
double mech_fused_pow(double x, double y) { return pow(x, y); }

static size_t width, stride;
static unsigned char *image;
static atomic_size_t next_row;
static double seconds(void) {
  struct timespec t;
  clock_gettime(CLOCK_MONOTONIC, &t);
  return (double)t.tv_sec + (double)t.tv_nsec / 1e9;
}
static void *worker(void *unused) {
  (void)unused;
  double inputs[MAP_INPUT_COUNT] = {0};
#if !MAP_PACKED_BITS
  double output[MAP_LANES];
#endif
  inputs[GRID_WIDTH_INPUT] = (double)width;
  for (;;) {
    size_t y = atomic_fetch_add_explicit(&next_row, 1, memory_order_relaxed);
    if (y >= width) break;
    inputs[GRID_ROW_INPUT] = (double)y;
    for (size_t x = 0; x < width; x += MAP_LANES) {
      for (size_t lane = 0; lane < MAP_LANES; ++lane)
        inputs[GRID_COLUMN_INPUT + lane] = (double)(x + lane);
#if MAP_PACKED_BITS
      uint64_t bits = mech_map_bits(inputs);
#if MAP_LANES == 8
      if (width - x < 8) bits &= (uint64_t)255 << (8 - (width - x));
      image[y * stride + x / 8] = (unsigned char)bits;
#else
      for (size_t lane = 0; lane < MAP_LANES && x + lane < width; ++lane)
        if ((bits >> (MAP_LANES - 1 - lane)) & 1)
          image[y * stride + (x + lane) / 8] |= (unsigned char)(128 >> ((x + lane) % 8));
#endif
#else
      mech_map_batch(inputs, output);
      for (size_t lane = 0; lane < MAP_LANES && x + lane < width; ++lane)
        if (output[lane] != 0.0)
          image[y * stride + (x + lane) / 8] |= (unsigned char)(128 >> ((x + lane) % 8));
#endif
    }
  }
  return NULL;
}
int main(int argc, char **argv) {
  if (argc < 2 || argc > 3) { fprintf(stderr, "usage: map WIDTH [THREADS]\n"); return 2; }
  char *end;
  errno = 0;
  unsigned long n = strtoul(argv[1], &end, 10);
  if (errno || *end || !n || n > 1000000) return 2;
  width = (size_t)n;
  stride = (width + 7) / 8;
  if (width > SIZE_MAX / stride) return 2;
  unsigned long threads = 1;
  if (argc == 3) {
    errno = 0;
    threads = strtoul(argv[2], &end, 10);
    if (errno || *end || !threads || threads > 256) return 2;
  }
  image = calloc(width * stride, 1);
  pthread_t *workers = calloc(threads, sizeof(pthread_t));
  if (!image || !workers) return 2;
  double start = seconds();
  for (size_t t = 1; t < threads; ++t)
    if (pthread_create(&workers[t], NULL, worker, NULL)) return 2;
  worker(NULL);
  for (size_t t = 1; t < threads; ++t)
    if (pthread_join(workers[t], NULL)) return 2;
  double kernel = seconds() - start;
  if (printf("P4\n%zu %zu\n", width, width) < 0 ||
      fwrite(image, 1, width * stride, stdout) != width * stride || fflush(stdout)) return 2;
  if (getenv("MECH_TIMINGS")) fprintf(stderr, "TIMING {\"kernel_seconds\":%.9f}\n", kernel);
  free(workers);
  free(image);
  return 0;
}
