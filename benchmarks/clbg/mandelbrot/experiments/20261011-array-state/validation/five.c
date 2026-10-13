#define MAP_INPUT_COUNT 10
#define MAP_LANES 8
#define MAP_PACKED_BITS 0
#define GRID_WIDTH_INPUT 0
#define GRID_COLUMN_INPUT 1
#define GRID_ROW_INPUT 9
#define GRID_WIDTH_ARG 1
#define GRID_WIDTH_CONST 0
#define GRID_HEIGHT_ARG 1
#define GRID_HEIGHT_CONST 0
#define GRID_RUNTIME_THREADS_ARG 2
/* General grid worker scheduling, argument access and PBM library IO.
   Column traversal and every map invocation are in the Cranelift object. */
#include <errno.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
void mech_grid_row(size_t width, size_t row, unsigned char *destination);
double mech_fused_pow(double x, double y) { return pow(x, y); }
static size_t width, height, stride;
static unsigned char *image;
static atomic_size_t next_row;
static int argument(int argc,char **argv,size_t ordinal,size_t *out) {
  if (ordinal >= (size_t)argc) return 0;
  errno=0;char *end;unsigned long value=strtoul(argv[ordinal],&end,10);
  if (errno || end==argv[ordinal] || *end || !value || value>1000000) return 0;
  *out=value;return 1;
}
static void *worker(void *unused) {
  (void)unused;
  for (;;) {
    size_t row=atomic_fetch_add_explicit(&next_row,1,memory_order_relaxed);
    if (row>=height) break;
    mech_grid_row(width,row,image+row*stride);
  }
  return NULL;
}
int main(int argc,char **argv) {
#if GRID_WIDTH_ARG
  if (!argument(argc,argv,GRID_WIDTH_ARG,&width)) return 2;
#else
  width=GRID_WIDTH_CONST;
#endif
#if GRID_HEIGHT_ARG
  if (!argument(argc,argv,GRID_HEIGHT_ARG,&height)) return 2;
#else
  height=GRID_HEIGHT_CONST;
#endif
  if (!width || !height || width>1000000 || height>1000000) return 2;
  stride=(width+7)/8;
  if (height>SIZE_MAX/stride) return 2;
  size_t threads=1;
  if ((size_t)argc>GRID_RUNTIME_THREADS_ARG && (!argument(argc,argv,GRID_RUNTIME_THREADS_ARG,&threads) || threads>256)) return 2;
  image=calloc(height*stride,1);pthread_t *workers=calloc(threads,sizeof(pthread_t));
  if (!image || !workers) return 2;
  for (size_t t=1;t<threads;t++) if (pthread_create(&workers[t],NULL,worker,NULL)) return 2;
  worker(NULL);
  for (size_t t=1;t<threads;t++) if (pthread_join(workers[t],NULL)) return 2;
  if (printf("P4\n%zu %zu\n",width,height)<0 || fwrite(image,1,height*stride,stdout)!=height*stride || fflush(stdout)) return 2;
  free(workers);free(image);return 0;
}
