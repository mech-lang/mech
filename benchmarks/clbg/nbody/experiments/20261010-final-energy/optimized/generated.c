#include <stddef.h>
#include <math.h>
void mech_turns(double *state, size_t turns);
void mech_observe(const double *state, double *output);
#define STATE_COUNT 30
#define OUTPUT_COUNT 1
static const double initial[30] = {
0.00000000000000000e0,
4.84143144246472090e0,
8.34336671824457987e0,
1.28943695621391310e1,
1.53796971148509165e1,
0.00000000000000000e0,
-1.16032004402742839e0,
4.12479856412430479e0,
-1.51111514016986312e1,
-2.59193146099879641e1,
0.00000000000000000e0,
-1.03622044471123109e-1,
-4.03523417114321381e-1,
-2.23307578892655734e-1,
1.79258772950371181e-1,
-3.87663407198742666e-4,
6.06326392995832020e-1,
-1.01077434617879236e0,
1.08279100644153536e0,
9.79090732243897977e-1,
-3.27535903717657067e-3,
2.81198684491626016e0,
1.82566237123041186e0,
8.68713018169608220e-1,
5.94698998647676169e-1,
2.39357340800029987e-5,
-2.52183616598876288e-2,
8.41576137658415352e-3,
-1.08326374013636359e-2,
-3.47559555040781037e-2,
};
/* Generic native runtime: execute turns and display the selected Mech output.
 * State initializers and dimensions are generated from the compiled artifact. */
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <time.h>
#include <string.h>

double mech_fused_pow(double x, double y) { return pow(x, y); }

static void print_output(const double *values) {
  if (OUTPUT_COUNT == 1) { printf("%.9f\n", values[0]); return; }
  printf("[");
  for (size_t i = 0; i < OUTPUT_COUNT; ++i) printf("%s%.17g", i ? ", " : "", values[i]);
  printf("]\n");
}

int main(int argc, char **argv) {
  if (argc != 2) return 2;
  errno = 0; char *end;
  unsigned long long turns = strtoull(argv[1], &end, 10);
  if (errno || *end || argv[1][0] == '-' || end == argv[1] || turns > (size_t)-1) return 2;
  double state[STATE_COUNT ? STATE_COUNT : 1], output[OUTPUT_COUNT];
  memcpy(state, initial, STATE_COUNT * sizeof(double));
  mech_observe(state, output);
  print_output(output);
  struct timespec start, finish;
  clock_gettime(CLOCK_MONOTONIC, &start);
  mech_turns(state, (size_t)turns);
  clock_gettime(CLOCK_MONOTONIC, &finish);
  mech_observe(state, output);
  print_output(output);
  if (getenv("MECH_DUMP_STATE")) {
    fprintf(stderr, "STATE [");
    for (size_t i = 0; i < STATE_COUNT; ++i) fprintf(stderr, "%s%.17g", i ? ", " : "", state[i]);
    fprintf(stderr, "]\n");
  }
  if (getenv("MECH_TIMINGS")) {
    double elapsed = finish.tv_sec-start.tv_sec + (finish.tv_nsec-start.tv_nsec)*1e-9;
    fprintf(stderr, "TIMING {\"prepare_seconds\":0,\"kernel_seconds\":%.9g}\n", elapsed);
  }
  return 0;
}
