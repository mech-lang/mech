/* Generic CLI/stdin/stdout adapter.  All computation and text formatting
   execute in Futhark.  Every entry has the same (i64, []u8) -> []u8 ABI. */
#include "program.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
  int64_t n = argc > 1 ? strtoll(argv[1], NULL, 10) : 0;
  size_t used = 0, capacity = 65536;
  uint8_t *input = malloc(capacity);
  for (;;) {
    if (used == capacity) {
      capacity *= 2;
      input = realloc(input, capacity);
      if (!input) return 2;
    }
    size_t got = fread(input+used, 1, capacity-used, stdin);
    used += got;
    if (got == 0) break;
  }
  struct futhark_context_config *cfg = futhark_context_config_new();
#ifdef FUTHARK_BACKEND_multicore
  futhark_context_config_set_num_threads(cfg, 8);
#endif
  struct futhark_context *ctx = futhark_context_new(cfg);
  struct futhark_u8_1d *in = futhark_new_u8_1d(ctx, input, used);
  struct futhark_u8_1d *out = NULL;
  int err = futhark_entry_main(ctx, &out, n, in);
  err |= futhark_context_sync(ctx);
  if (err) {
    char *message = futhark_context_get_error(ctx);
    fprintf(stderr, "%s\n", message ? message : "Futhark execution failed");
    free(message);
    return 2;
  }
  int64_t size = futhark_shape_u8_1d(ctx, out)[0];
  uint8_t *bytes = malloc(size ? size : 1);
  err = futhark_values_u8_1d(ctx, out, bytes);
  err |= futhark_context_sync(ctx);
  if (!err && fwrite(bytes, 1, size, stdout) != (size_t)size) err = 2;
  futhark_free_u8_1d(ctx, out);
  futhark_free_u8_1d(ctx, in);
  futhark_context_free(ctx);
  futhark_context_config_free(cfg);
  free(bytes);
  free(input);
  return err ? 2 : 0;
}
