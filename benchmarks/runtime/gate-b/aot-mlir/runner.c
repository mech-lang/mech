#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

typedef struct {
  double *allocated;
  double *aligned;
  intptr_t offset;
  intptr_t sizes[1];
  intptr_t strides[1];
} MemRef1D;

extern int64_t mech_input_len(void);
extern int64_t mech_state_len(void);
extern void _mlir_ciface_mech_initialize(MemRef1D *state);
extern void _mlir_ciface_mech_run_fast(MemRef1D *inputs, MemRef1D *state,
                                       intptr_t turns);

static double monotonic_seconds(void) {
  struct timespec value;
  if (clock_gettime(CLOCK_MONOTONIC, &value) != 0) {
    perror("clock_gettime");
    exit(1);
  }
  return (double)value.tv_sec + (double)value.tv_nsec * 1e-9;
}

static uint64_t hash_state(const double *state, size_t len) {
  uint64_t hash = UINT64_C(0xcbf29ce484222325);
  for (size_t index = 0; index < len; index++) {
    uint64_t bits;
    memcpy(&bits, &state[index], sizeof(bits));
    hash = (hash ^ bits) * UINT64_C(0x100000001b3);
  }
  return hash;
}

static uint64_t parse_turns(const char *text) {
  errno = 0;
  char *end = NULL;
  unsigned long long value = strtoull(text, &end, 10);
  if (errno != 0 || end == text || *end != '\0' || value == 0) {
    fprintf(stderr, "invalid turn count '%s'\n", text);
    exit(2);
  }
  return (uint64_t)value;
}

int main(int argc, char **argv) {
  if (argc != 2) {
    fprintf(stderr, "usage: mlir-runner TURNS\n");
    return 2;
  }
  uint64_t turns = parse_turns(argv[1]);
  int64_t input_len = mech_input_len();
  int64_t state_len = mech_state_len();
  if (input_len < 0 || state_len <= 0) {
    fprintf(stderr, "invalid generated buffer lengths\n");
    return 1;
  }

  size_t input_allocation = input_len == 0 ? 1 : (size_t)input_len;
  double *inputs = calloc(input_allocation, sizeof(double));
  double *state = calloc((size_t)state_len, sizeof(double));
  if (inputs == NULL || state == NULL) {
    fprintf(stderr, "failed to allocate generated buffers\n");
    free(inputs);
    free(state);
    return 1;
  }
  MemRef1D input_ref = {inputs, inputs, 0, {input_len}, {1}};
  MemRef1D state_ref = {state, state, 0, {state_len}, {1}};
  _mlir_ciface_mech_initialize(&state_ref);

  double started = monotonic_seconds();
  _mlir_ciface_mech_run_fast(&input_ref, &state_ref, (intptr_t)turns);
  double seconds = monotonic_seconds() - started;
  uint64_t checksum = hash_state(state, (size_t)state_len);

  puts("implementation,turns,seconds,ns_per_turn,turns_per_second,state_"
       "checksum");
  printf("mech-aot-mlir-fast,%" PRIu64 ",%.9f,%.3f,%.3f,%" PRIu64 "\n", turns,
         seconds, seconds * 1e9 / (double)turns, (double)turns / seconds,
         checksum);
  free(inputs);
  free(state);
  return 0;
}
