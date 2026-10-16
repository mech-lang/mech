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
