
// We need to define _GNU_SOURCE before
// _any_ headers files are imported to get
// the usage statistics of a thread (i.e. have RUSAGE_THREAD) on GNU/Linux
// https://manpages.courier-mta.org/htmlman2/getrusage.2.html
#ifndef _GNU_SOURCE // Avoid possible double-definition warning.
#define _GNU_SOURCE
#endif
#define _FILE_OFFSET_BITS 64

#ifdef __clang__
#pragma clang diagnostic ignored "-Wunused-function"
#pragma clang diagnostic ignored "-Wunused-variable"
#pragma clang diagnostic ignored "-Wunused-const-variable"
#pragma clang diagnostic ignored "-Wparentheses"
#pragma clang diagnostic ignored "-Wunused-label"
#pragma clang diagnostic ignored "-Wunused-but-set-variable"
#elif __GNUC__
#pragma GCC diagnostic ignored "-Wunused-function"
#pragma GCC diagnostic ignored "-Wunused-variable"
#pragma GCC diagnostic ignored "-Wunused-const-variable"
#pragma GCC diagnostic ignored "-Wparentheses"
#pragma GCC diagnostic ignored "-Wunused-label"
#pragma GCC diagnostic ignored "-Wunused-but-set-variable"
#endif

// Headers
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdio.h>
#include <float.h>

#ifdef __cplusplus
extern "C" {
#endif

// Initialisation
struct futhark_context_config;
struct futhark_context_config *futhark_context_config_new(void);
void futhark_context_config_free(struct futhark_context_config *cfg);
int futhark_context_config_set_tuning_param(struct futhark_context_config *cfg, const char *param_name, size_t new_value);
struct futhark_context;
struct futhark_context *futhark_context_new(struct futhark_context_config *cfg);
void futhark_context_free(struct futhark_context *ctx);
void futhark_context_config_set_debugging(struct futhark_context_config *cfg, int flag);
void futhark_context_config_set_profiling(struct futhark_context_config *cfg, int flag);
void futhark_context_config_set_logging(struct futhark_context_config *cfg, int flag);
int futhark_get_tuning_param_count(void);
const char *futhark_get_tuning_param_name(int);
const char *futhark_get_tuning_param_class(int);

// Arrays
struct futhark_i64_1d;
struct futhark_i64_1d *futhark_new_i64_1d(struct futhark_context *ctx, const int64_t *data, int64_t dim0);
struct futhark_i64_1d *futhark_new_raw_i64_1d(struct futhark_context *ctx, unsigned char *data, int64_t dim0);
int futhark_free_i64_1d(struct futhark_context *ctx, struct futhark_i64_1d *arr);
int futhark_values_i64_1d(struct futhark_context *ctx, struct futhark_i64_1d *arr, int64_t *data);
int futhark_index_i64_1d(struct futhark_context *ctx, int64_t *out, struct futhark_i64_1d *arr, int64_t i0);
unsigned char *futhark_values_raw_i64_1d(struct futhark_context *ctx, struct futhark_i64_1d *arr);
const int64_t *futhark_shape_i64_1d(struct futhark_context *ctx, struct futhark_i64_1d *arr);
struct futhark_u32_1d;
struct futhark_u32_1d *futhark_new_u32_1d(struct futhark_context *ctx, const uint32_t *data, int64_t dim0);
struct futhark_u32_1d *futhark_new_raw_u32_1d(struct futhark_context *ctx, unsigned char *data, int64_t dim0);
int futhark_free_u32_1d(struct futhark_context *ctx, struct futhark_u32_1d *arr);
int futhark_values_u32_1d(struct futhark_context *ctx, struct futhark_u32_1d *arr, uint32_t *data);
int futhark_index_u32_1d(struct futhark_context *ctx, uint32_t *out, struct futhark_u32_1d *arr, int64_t i0);
unsigned char *futhark_values_raw_u32_1d(struct futhark_context *ctx, struct futhark_u32_1d *arr);
const int64_t *futhark_shape_u32_1d(struct futhark_context *ctx, struct futhark_u32_1d *arr);
struct futhark_u64_1d;
struct futhark_u64_1d *futhark_new_u64_1d(struct futhark_context *ctx, const uint64_t *data, int64_t dim0);
struct futhark_u64_1d *futhark_new_raw_u64_1d(struct futhark_context *ctx, unsigned char *data, int64_t dim0);
int futhark_free_u64_1d(struct futhark_context *ctx, struct futhark_u64_1d *arr);
int futhark_values_u64_1d(struct futhark_context *ctx, struct futhark_u64_1d *arr, uint64_t *data);
int futhark_index_u64_1d(struct futhark_context *ctx, uint64_t *out, struct futhark_u64_1d *arr, int64_t i0);
unsigned char *futhark_values_raw_u64_1d(struct futhark_context *ctx, struct futhark_u64_1d *arr);
const int64_t *futhark_shape_u64_1d(struct futhark_context *ctx, struct futhark_u64_1d *arr);
struct futhark_u8_1d;
struct futhark_u8_1d *futhark_new_u8_1d(struct futhark_context *ctx, const uint8_t *data, int64_t dim0);
struct futhark_u8_1d *futhark_new_raw_u8_1d(struct futhark_context *ctx, unsigned char *data, int64_t dim0);
int futhark_free_u8_1d(struct futhark_context *ctx, struct futhark_u8_1d *arr);
int futhark_values_u8_1d(struct futhark_context *ctx, struct futhark_u8_1d *arr, uint8_t *data);
int futhark_index_u8_1d(struct futhark_context *ctx, uint8_t *out, struct futhark_u8_1d *arr, int64_t i0);
unsigned char *futhark_values_raw_u8_1d(struct futhark_context *ctx, struct futhark_u8_1d *arr);
const int64_t *futhark_shape_u8_1d(struct futhark_context *ctx, struct futhark_u8_1d *arr);

// Opaque values
struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64;
struct futhark_opaque_tup2_bool_arr1d_u32;
struct futhark_opaque_tup2_i64_arr1d_u8;
int futhark_free_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(struct futhark_context *ctx, struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj);
int futhark_store_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(struct futhark_context *ctx, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj, void **p, size_t *n);
struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *futhark_restore_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(struct futhark_context *ctx, const void *p);
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_0(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj);
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_1(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj);
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_2(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj);
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_3(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj);
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_4(struct futhark_context *ctx, int64_t *out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj);
int futhark_new_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(struct futhark_context *ctx, struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 **out, const struct futhark_opaque_tup2_bool_arr1d_u32 *f_0, const struct futhark_opaque_tup2_bool_arr1d_u32 *f_1, const struct futhark_opaque_tup2_bool_arr1d_u32 *f_2, const struct futhark_opaque_tup2_bool_arr1d_u32 *f_3, const int64_t f_4);
int futhark_free_opaque_tup2_bool_arr1d_u32(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 *obj);
int futhark_store_opaque_tup2_bool_arr1d_u32(struct futhark_context *ctx, const struct futhark_opaque_tup2_bool_arr1d_u32 *obj, void **p, size_t *n);
struct futhark_opaque_tup2_bool_arr1d_u32 *futhark_restore_opaque_tup2_bool_arr1d_u32(struct futhark_context *ctx, const void *p);
int futhark_project_opaque_tup2_bool_arr1d_u32_0(struct futhark_context *ctx, bool *out, const struct futhark_opaque_tup2_bool_arr1d_u32 *obj);
int futhark_project_opaque_tup2_bool_arr1d_u32_1(struct futhark_context *ctx, struct futhark_u32_1d **out, const struct futhark_opaque_tup2_bool_arr1d_u32 *obj);
int futhark_new_opaque_tup2_bool_arr1d_u32(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const bool f_0, const struct futhark_u32_1d *f_1);
int futhark_free_opaque_tup2_i64_arr1d_u8(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 *obj);
int futhark_store_opaque_tup2_i64_arr1d_u8(struct futhark_context *ctx, const struct futhark_opaque_tup2_i64_arr1d_u8 *obj, void **p, size_t *n);
struct futhark_opaque_tup2_i64_arr1d_u8 *futhark_restore_opaque_tup2_i64_arr1d_u8(struct futhark_context *ctx, const void *p);
int futhark_project_opaque_tup2_i64_arr1d_u8_0(struct futhark_context *ctx, int64_t *out, const struct futhark_opaque_tup2_i64_arr1d_u8 *obj);
int futhark_project_opaque_tup2_i64_arr1d_u8_1(struct futhark_context *ctx, struct futhark_u8_1d **out, const struct futhark_opaque_tup2_i64_arr1d_u8 *obj);
int futhark_new_opaque_tup2_i64_arr1d_u8(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 **out, const int64_t f_0, const struct futhark_u8_1d *f_1);

// Entry points
int futhark_entry_arithmetic(struct futhark_context *ctx, struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 **out, const bool in0, const struct futhark_u32_1d *in1, const bool in2, const struct futhark_u32_1d *in3, const uint32_t in4);
int futhark_entry_hashmap_increment(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0, const struct futhark_i64_1d *in1);
int futhark_entry_hashmap_reserved(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0, const struct futhark_i64_1d *in1, const int64_t in2);
int futhark_entry_hashmap_test(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0);
int futhark_entry_regex_test(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 **out, const struct futhark_u8_1d *in0, const struct futhark_u8_1d *in1, const struct futhark_u8_1d *in2);
int futhark_entry_wrap_test(struct futhark_context *ctx, struct futhark_u8_1d **out, const int64_t in0, const struct futhark_u8_1d *in1);

// Miscellaneous
int futhark_context_sync(struct futhark_context *ctx);
void futhark_context_config_set_cache_file(struct futhark_context_config *cfg, const char *f);
char *futhark_context_get_error(struct futhark_context *ctx);
void futhark_context_set_logging_file(struct futhark_context *ctx, FILE *f);
void futhark_context_pause_profiling(struct futhark_context *ctx);
void futhark_context_unpause_profiling(struct futhark_context *ctx);
char *futhark_context_report(struct futhark_context *ctx);
int futhark_context_clear_caches(struct futhark_context *ctx);
#define FUTHARK_BACKEND_c
#define FUTHARK_SUCCESS 0
#define FUTHARK_PROGRAM_ERROR 2
#define FUTHARK_OUT_OF_MEMORY 3

#ifdef __cplusplus
}
#endif

#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <math.h>
#include <stdint.h>
// If NDEBUG is set, the assert() macro will do nothing. Since Futhark
// (unfortunately) makes use of assert() for error detection (and even some
// side effects), we want to avoid that.
#undef NDEBUG
#include <assert.h>
#include <stdarg.h>
#define SCALAR_FUN_ATTR static inline
// Start of util.h.
//
// Various helper functions that are useful in all generated C code.

#include <errno.h>
#include <string.h>

static const char *fut_progname = "(embedded Futhark)";

static void futhark_panic(int eval, const char *fmt, ...) __attribute__((noreturn));
static char* msgprintf(const char *s, ...);
static void* slurp_file(const char *filename, size_t *size);
static int dump_file(const char *file, const void *buf, size_t n);
struct str_builder;
static void str_builder_init(struct str_builder *b);
static void str_builder(struct str_builder *b, const char *s, ...);
static char *strclone(const char *str);

static void futhark_panic(int eval, const char *fmt, ...) {
  va_list ap;
  va_start(ap, fmt);
  fprintf(stderr, "%s: ", fut_progname);
  vfprintf(stderr, fmt, ap);
  va_end(ap);
  exit(eval);
}

// For generating arbitrary-sized error messages.  It is the callers
// responsibility to free the buffer at some point.
static char* msgprintf(const char *s, ...) {
  va_list vl;
  va_start(vl, s);
  size_t needed = 1 + (size_t)vsnprintf(NULL, 0, s, vl);
  char *buffer = (char*) malloc(needed);
  va_start(vl, s); // Must re-init.
  vsnprintf(buffer, needed, s, vl);
  return buffer;
}

static inline void check_err(int errval, int sets_errno, const char *fun, int line,
                             const char *msg, ...) {
  if (errval) {
    char errnum[10];

    va_list vl;
    va_start(vl, msg);

    fprintf(stderr, "ERROR: ");
    vfprintf(stderr, msg, vl);
    fprintf(stderr, " in %s() at line %d with error code %s\n",
            fun, line,
            sets_errno ? strerror(errno) : errnum);
    exit(errval);
  }
}

#define CHECK_ERR(err, ...) check_err(err, 0, __func__, __LINE__, __VA_ARGS__)
#define CHECK_ERRNO(err, ...) check_err(err, 1, __func__, __LINE__, __VA_ARGS__)

// Read the rest of an open file into a NUL-terminated string; returns
// NULL on error.
static void* fslurp_file(FILE *f, size_t *size) {
  off_t start = ftello(f);
  fseeko(f, 0, SEEK_END);
  off_t src_size = ftello(f)-start;
  fseeko(f, start, SEEK_SET);
  unsigned char *s = (unsigned char*) malloc((size_t)src_size + 1);
  if (fread(s, 1, (size_t)src_size, f) != (size_t)src_size) {
    free(s);
    s = NULL;
  } else {
    s[src_size] = '\0';
  }

  if (size) {
    *size = (size_t)src_size;
  }

  return s;
}

// Read a file into a NUL-terminated string; returns NULL on error.
static void* slurp_file(const char *filename, size_t *size) {
  FILE *f = fopen(filename, "rb"); // To avoid Windows messing with linebreaks.
  if (f == NULL) return NULL;
  unsigned char *s = fslurp_file(f, size);
  fclose(f);
  return s;
}

// Dump 'n' bytes from 'buf' into the file at the designated location.
// Returns 0 on success.
static int dump_file(const char *file, const void *buf, size_t n) {
  FILE *f = fopen(file, "w");

  if (f == NULL) {
    return 1;
  }

  if (fwrite(buf, sizeof(char), n, f) != n) {
    return 1;
  }

  if (fclose(f) != 0) {
    return 1;
  }

  return 0;
}

struct str_builder {
  char *str;
  size_t capacity; // Size of buffer.
  size_t used; // Bytes used, *not* including final zero.
};

static void str_builder_init(struct str_builder *b) {
  b->capacity = 10;
  b->used = 0;
  b->str = malloc(b->capacity);
  b->str[0] = 0;
}

static void str_builder(struct str_builder *b, const char *s, ...) {
  va_list vl;
  va_start(vl, s);
  size_t needed = (size_t)vsnprintf(NULL, 0, s, vl);

  while (b->capacity < b->used + needed + 1) {
    b->capacity *= 2;
    b->str = realloc(b->str, b->capacity);
  }

  va_start(vl, s); // Must re-init.
  vsnprintf(b->str+b->used, b->capacity-b->used, s, vl);
  b->used += needed;
}

static void str_builder_str(struct str_builder *b, const char *s) {
  size_t needed = strlen(s);
  if (b->capacity < b->used + needed + 1) {
    b->capacity *= 2;
    b->str = realloc(b->str, b->capacity);
  }
  strcpy(b->str+b->used, s);
  b->used += needed;
}

static void str_builder_char(struct str_builder *b, char c) {
  size_t needed = 1;
  if (b->capacity < b->used + needed + 1) {
    b->capacity *= 2;
    b->str = realloc(b->str, b->capacity);
  }
  b->str[b->used] = c;
  b->str[b->used+1] = 0;
  b->used += needed;
}

static void str_builder_json_str(struct str_builder* sb, const char* s) {
  str_builder_char(sb, '"');
  for (int j = 0; s[j]; j++) {
    char c = s[j];
    switch (c) {
    case '\n':
      str_builder_str(sb, "\\n");
      break;
    case '"':
      str_builder_str(sb, "\\\"");
      break;
    default:
      str_builder_char(sb, c);
    }
  }
  str_builder_char(sb, '"');
}

static char *strclone(const char *str) {
  size_t size = strlen(str) + 1;
  char *copy = (char*) malloc(size);
  if (copy == NULL) {
    return NULL;
  }

  memcpy(copy, str, size);
  return copy;
}

// Assumes NULL-terminated.
static char *strconcat(const char *src_fragments[]) {
  size_t src_len = 0;
  const char **p;

  for (p = src_fragments; *p; p++) {
    src_len += strlen(*p);
  }

  char *src = (char*) malloc(src_len + 1);
  size_t n = 0;
  for (p = src_fragments; *p; p++) {
    strcpy(src + n, *p);
    n += strlen(*p);
  }

  return src;
}

// End of util.h.
// Start of cache.h

#define CACHE_HASH_SIZE 8 // In 32-bit words.

struct cache_hash {
  uint32_t hash[CACHE_HASH_SIZE];
};

// Initialise a blank cache.
static void cache_hash_init(struct cache_hash *c);

// Hash some bytes and add them to the accumulated hash.
static void cache_hash(struct cache_hash *out, const char *in, size_t n);

// Try to restore cache contents from a file with the given name.
// Assumes the cache is invalid if it contains the given hash.
// Allocates memory and reads the cache conents, which is returned in
// *buf with size *buflen.  If the cache is successfully loaded, this
// function returns 0.  Otherwise it returns nonzero.  Errno is set if
// the failure to load the cache is due to anything except invalid
// cache conents.  Note that failing to restore the cache is not
// necessarily a problem: it might just be invalid or not created yet.
static int cache_restore(const char *fname, const struct cache_hash *hash,
                         unsigned char **buf, size_t *buflen);

// Store cache contents in the given file, with the given hash.
static int cache_store(const char *fname, const struct cache_hash *hash,
                       const unsigned char *buf, size_t buflen);

// Now for the implementation.

static void cache_hash_init(struct cache_hash *c) {
  memset(c->hash, 0, CACHE_HASH_SIZE * sizeof(uint32_t));
}

static void cache_hash(struct cache_hash *out, const char *in, size_t n) {
  // Adaptation of djb2 for larger output size by storing intermediate
  // states.
  uint32_t hash = 5381;
  for (size_t i = 0; i < n; i++) {
    hash = ((hash << 5) + hash) + in[i];
    out->hash[i % CACHE_HASH_SIZE] ^= hash;
  }
}

#define CACHE_HEADER_SIZE 8
static const char cache_header[CACHE_HEADER_SIZE] = "FUTHARK";

static int cache_restore(const char *fname, const struct cache_hash *hash,
                         unsigned char **buf, size_t *buflen) {
  FILE *f = fopen(fname, "rb");

  if (f == NULL) {
    return 1;
  }

  char f_header[CACHE_HEADER_SIZE];

  if (fread(f_header, sizeof(char), CACHE_HEADER_SIZE, f) != CACHE_HEADER_SIZE) {
    goto error;
  }

  if (memcmp(f_header, cache_header, CACHE_HEADER_SIZE) != 0) {
    goto error;
  }

  if (fseek(f, 0, SEEK_END) != 0) {
    goto error;
  }
  int64_t f_size = (int64_t)ftello(f);
  if (fseeko(f, CACHE_HEADER_SIZE, SEEK_SET) != 0) {
    goto error;
  }

  int64_t expected_size;

  if (fread(&expected_size, sizeof(int64_t), 1, f) != 1) {
    goto error;
  }

  if (f_size != expected_size) {
    errno = 0;
    goto error;
  }

  int32_t f_hash[CACHE_HASH_SIZE];

  if (fread(f_hash, sizeof(int32_t), CACHE_HASH_SIZE, f) != CACHE_HASH_SIZE) {
    goto error;
  }

  if (memcmp(f_hash, hash->hash, CACHE_HASH_SIZE) != 0) {
    errno = 0;
    goto error;
  }

  *buflen = f_size - CACHE_HEADER_SIZE - sizeof(int64_t) - CACHE_HASH_SIZE*sizeof(int32_t);
  *buf = malloc(*buflen);
  if (fread(*buf, sizeof(char), *buflen, f) != *buflen) {
    free(*buf);
    goto error;
  }

  fclose(f);

  return 0;

 error:
  fclose(f);
  return 1;
}

static int cache_store(const char *fname, const struct cache_hash *hash,
                       const unsigned char *buf, size_t buflen) {
  FILE *f = fopen(fname, "wb");

  if (f == NULL) {
    return 1;
  }

  if (fwrite(cache_header, CACHE_HEADER_SIZE, 1, f) != 1) {
    goto error;
  }

  int64_t size = CACHE_HEADER_SIZE + sizeof(int64_t) + CACHE_HASH_SIZE*sizeof(int32_t) + buflen;

  if (fwrite(&size, sizeof(size), 1, f) != 1) {
    goto error;
  }

  if (fwrite(hash->hash, sizeof(int32_t), CACHE_HASH_SIZE, f) != CACHE_HASH_SIZE) {
    goto error;
  }

  if (fwrite(buf, sizeof(unsigned char), buflen, f) != buflen) {
    goto error;
  }

  fclose(f);

  return 0;

 error:
  fclose(f);
  return 1;
}

// End of cache.h
// Start of half.h.

// Conversion functions are from http://half.sourceforge.net/, but
// translated to C.
//
// Copyright (c) 2012-2021 Christian Rau
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#ifndef __OPENCL_VERSION__
#define __constant
#endif

__constant static const uint16_t base_table[512] = {
  0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
  0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
  0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
  0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
  0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
  0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
  0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0001, 0x0002, 0x0004, 0x0008, 0x0010, 0x0020, 0x0040, 0x0080, 0x0100,
  0x0200, 0x0400, 0x0800, 0x0C00, 0x1000, 0x1400, 0x1800, 0x1C00, 0x2000, 0x2400, 0x2800, 0x2C00, 0x3000, 0x3400, 0x3800, 0x3C00,
  0x4000, 0x4400, 0x4800, 0x4C00, 0x5000, 0x5400, 0x5800, 0x5C00, 0x6000, 0x6400, 0x6800, 0x6C00, 0x7000, 0x7400, 0x7800, 0x7C00,
  0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00,
  0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00,
  0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00,
  0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00,
  0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00,
  0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00,
  0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00, 0x7C00,
  0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000,
  0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000,
  0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000,
  0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000,
  0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000,
  0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000,
  0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8000, 0x8001, 0x8002, 0x8004, 0x8008, 0x8010, 0x8020, 0x8040, 0x8080, 0x8100,
  0x8200, 0x8400, 0x8800, 0x8C00, 0x9000, 0x9400, 0x9800, 0x9C00, 0xA000, 0xA400, 0xA800, 0xAC00, 0xB000, 0xB400, 0xB800, 0xBC00,
  0xC000, 0xC400, 0xC800, 0xCC00, 0xD000, 0xD400, 0xD800, 0xDC00, 0xE000, 0xE400, 0xE800, 0xEC00, 0xF000, 0xF400, 0xF800, 0xFC00,
  0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00,
  0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00,
  0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00,
  0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00,
  0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00,
  0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00,
  0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00, 0xFC00 };

__constant static const unsigned char shift_table[512] = {
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
  13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 13,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13,
  13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
  24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 13 };

__constant static const uint32_t mantissa_table[2048] = {
  0x00000000, 0x33800000, 0x34000000, 0x34400000, 0x34800000, 0x34A00000, 0x34C00000, 0x34E00000, 0x35000000, 0x35100000, 0x35200000, 0x35300000, 0x35400000, 0x35500000, 0x35600000, 0x35700000,
  0x35800000, 0x35880000, 0x35900000, 0x35980000, 0x35A00000, 0x35A80000, 0x35B00000, 0x35B80000, 0x35C00000, 0x35C80000, 0x35D00000, 0x35D80000, 0x35E00000, 0x35E80000, 0x35F00000, 0x35F80000,
  0x36000000, 0x36040000, 0x36080000, 0x360C0000, 0x36100000, 0x36140000, 0x36180000, 0x361C0000, 0x36200000, 0x36240000, 0x36280000, 0x362C0000, 0x36300000, 0x36340000, 0x36380000, 0x363C0000,
  0x36400000, 0x36440000, 0x36480000, 0x364C0000, 0x36500000, 0x36540000, 0x36580000, 0x365C0000, 0x36600000, 0x36640000, 0x36680000, 0x366C0000, 0x36700000, 0x36740000, 0x36780000, 0x367C0000,
  0x36800000, 0x36820000, 0x36840000, 0x36860000, 0x36880000, 0x368A0000, 0x368C0000, 0x368E0000, 0x36900000, 0x36920000, 0x36940000, 0x36960000, 0x36980000, 0x369A0000, 0x369C0000, 0x369E0000,
  0x36A00000, 0x36A20000, 0x36A40000, 0x36A60000, 0x36A80000, 0x36AA0000, 0x36AC0000, 0x36AE0000, 0x36B00000, 0x36B20000, 0x36B40000, 0x36B60000, 0x36B80000, 0x36BA0000, 0x36BC0000, 0x36BE0000,
  0x36C00000, 0x36C20000, 0x36C40000, 0x36C60000, 0x36C80000, 0x36CA0000, 0x36CC0000, 0x36CE0000, 0x36D00000, 0x36D20000, 0x36D40000, 0x36D60000, 0x36D80000, 0x36DA0000, 0x36DC0000, 0x36DE0000,
  0x36E00000, 0x36E20000, 0x36E40000, 0x36E60000, 0x36E80000, 0x36EA0000, 0x36EC0000, 0x36EE0000, 0x36F00000, 0x36F20000, 0x36F40000, 0x36F60000, 0x36F80000, 0x36FA0000, 0x36FC0000, 0x36FE0000,
  0x37000000, 0x37010000, 0x37020000, 0x37030000, 0x37040000, 0x37050000, 0x37060000, 0x37070000, 0x37080000, 0x37090000, 0x370A0000, 0x370B0000, 0x370C0000, 0x370D0000, 0x370E0000, 0x370F0000,
  0x37100000, 0x37110000, 0x37120000, 0x37130000, 0x37140000, 0x37150000, 0x37160000, 0x37170000, 0x37180000, 0x37190000, 0x371A0000, 0x371B0000, 0x371C0000, 0x371D0000, 0x371E0000, 0x371F0000,
  0x37200000, 0x37210000, 0x37220000, 0x37230000, 0x37240000, 0x37250000, 0x37260000, 0x37270000, 0x37280000, 0x37290000, 0x372A0000, 0x372B0000, 0x372C0000, 0x372D0000, 0x372E0000, 0x372F0000,
  0x37300000, 0x37310000, 0x37320000, 0x37330000, 0x37340000, 0x37350000, 0x37360000, 0x37370000, 0x37380000, 0x37390000, 0x373A0000, 0x373B0000, 0x373C0000, 0x373D0000, 0x373E0000, 0x373F0000,
  0x37400000, 0x37410000, 0x37420000, 0x37430000, 0x37440000, 0x37450000, 0x37460000, 0x37470000, 0x37480000, 0x37490000, 0x374A0000, 0x374B0000, 0x374C0000, 0x374D0000, 0x374E0000, 0x374F0000,
  0x37500000, 0x37510000, 0x37520000, 0x37530000, 0x37540000, 0x37550000, 0x37560000, 0x37570000, 0x37580000, 0x37590000, 0x375A0000, 0x375B0000, 0x375C0000, 0x375D0000, 0x375E0000, 0x375F0000,
  0x37600000, 0x37610000, 0x37620000, 0x37630000, 0x37640000, 0x37650000, 0x37660000, 0x37670000, 0x37680000, 0x37690000, 0x376A0000, 0x376B0000, 0x376C0000, 0x376D0000, 0x376E0000, 0x376F0000,
  0x37700000, 0x37710000, 0x37720000, 0x37730000, 0x37740000, 0x37750000, 0x37760000, 0x37770000, 0x37780000, 0x37790000, 0x377A0000, 0x377B0000, 0x377C0000, 0x377D0000, 0x377E0000, 0x377F0000,
  0x37800000, 0x37808000, 0x37810000, 0x37818000, 0x37820000, 0x37828000, 0x37830000, 0x37838000, 0x37840000, 0x37848000, 0x37850000, 0x37858000, 0x37860000, 0x37868000, 0x37870000, 0x37878000,
  0x37880000, 0x37888000, 0x37890000, 0x37898000, 0x378A0000, 0x378A8000, 0x378B0000, 0x378B8000, 0x378C0000, 0x378C8000, 0x378D0000, 0x378D8000, 0x378E0000, 0x378E8000, 0x378F0000, 0x378F8000,
  0x37900000, 0x37908000, 0x37910000, 0x37918000, 0x37920000, 0x37928000, 0x37930000, 0x37938000, 0x37940000, 0x37948000, 0x37950000, 0x37958000, 0x37960000, 0x37968000, 0x37970000, 0x37978000,
  0x37980000, 0x37988000, 0x37990000, 0x37998000, 0x379A0000, 0x379A8000, 0x379B0000, 0x379B8000, 0x379C0000, 0x379C8000, 0x379D0000, 0x379D8000, 0x379E0000, 0x379E8000, 0x379F0000, 0x379F8000,
  0x37A00000, 0x37A08000, 0x37A10000, 0x37A18000, 0x37A20000, 0x37A28000, 0x37A30000, 0x37A38000, 0x37A40000, 0x37A48000, 0x37A50000, 0x37A58000, 0x37A60000, 0x37A68000, 0x37A70000, 0x37A78000,
  0x37A80000, 0x37A88000, 0x37A90000, 0x37A98000, 0x37AA0000, 0x37AA8000, 0x37AB0000, 0x37AB8000, 0x37AC0000, 0x37AC8000, 0x37AD0000, 0x37AD8000, 0x37AE0000, 0x37AE8000, 0x37AF0000, 0x37AF8000,
  0x37B00000, 0x37B08000, 0x37B10000, 0x37B18000, 0x37B20000, 0x37B28000, 0x37B30000, 0x37B38000, 0x37B40000, 0x37B48000, 0x37B50000, 0x37B58000, 0x37B60000, 0x37B68000, 0x37B70000, 0x37B78000,
  0x37B80000, 0x37B88000, 0x37B90000, 0x37B98000, 0x37BA0000, 0x37BA8000, 0x37BB0000, 0x37BB8000, 0x37BC0000, 0x37BC8000, 0x37BD0000, 0x37BD8000, 0x37BE0000, 0x37BE8000, 0x37BF0000, 0x37BF8000,
  0x37C00000, 0x37C08000, 0x37C10000, 0x37C18000, 0x37C20000, 0x37C28000, 0x37C30000, 0x37C38000, 0x37C40000, 0x37C48000, 0x37C50000, 0x37C58000, 0x37C60000, 0x37C68000, 0x37C70000, 0x37C78000,
  0x37C80000, 0x37C88000, 0x37C90000, 0x37C98000, 0x37CA0000, 0x37CA8000, 0x37CB0000, 0x37CB8000, 0x37CC0000, 0x37CC8000, 0x37CD0000, 0x37CD8000, 0x37CE0000, 0x37CE8000, 0x37CF0000, 0x37CF8000,
  0x37D00000, 0x37D08000, 0x37D10000, 0x37D18000, 0x37D20000, 0x37D28000, 0x37D30000, 0x37D38000, 0x37D40000, 0x37D48000, 0x37D50000, 0x37D58000, 0x37D60000, 0x37D68000, 0x37D70000, 0x37D78000,
  0x37D80000, 0x37D88000, 0x37D90000, 0x37D98000, 0x37DA0000, 0x37DA8000, 0x37DB0000, 0x37DB8000, 0x37DC0000, 0x37DC8000, 0x37DD0000, 0x37DD8000, 0x37DE0000, 0x37DE8000, 0x37DF0000, 0x37DF8000,
  0x37E00000, 0x37E08000, 0x37E10000, 0x37E18000, 0x37E20000, 0x37E28000, 0x37E30000, 0x37E38000, 0x37E40000, 0x37E48000, 0x37E50000, 0x37E58000, 0x37E60000, 0x37E68000, 0x37E70000, 0x37E78000,
  0x37E80000, 0x37E88000, 0x37E90000, 0x37E98000, 0x37EA0000, 0x37EA8000, 0x37EB0000, 0x37EB8000, 0x37EC0000, 0x37EC8000, 0x37ED0000, 0x37ED8000, 0x37EE0000, 0x37EE8000, 0x37EF0000, 0x37EF8000,
  0x37F00000, 0x37F08000, 0x37F10000, 0x37F18000, 0x37F20000, 0x37F28000, 0x37F30000, 0x37F38000, 0x37F40000, 0x37F48000, 0x37F50000, 0x37F58000, 0x37F60000, 0x37F68000, 0x37F70000, 0x37F78000,
  0x37F80000, 0x37F88000, 0x37F90000, 0x37F98000, 0x37FA0000, 0x37FA8000, 0x37FB0000, 0x37FB8000, 0x37FC0000, 0x37FC8000, 0x37FD0000, 0x37FD8000, 0x37FE0000, 0x37FE8000, 0x37FF0000, 0x37FF8000,
  0x38000000, 0x38004000, 0x38008000, 0x3800C000, 0x38010000, 0x38014000, 0x38018000, 0x3801C000, 0x38020000, 0x38024000, 0x38028000, 0x3802C000, 0x38030000, 0x38034000, 0x38038000, 0x3803C000,
  0x38040000, 0x38044000, 0x38048000, 0x3804C000, 0x38050000, 0x38054000, 0x38058000, 0x3805C000, 0x38060000, 0x38064000, 0x38068000, 0x3806C000, 0x38070000, 0x38074000, 0x38078000, 0x3807C000,
  0x38080000, 0x38084000, 0x38088000, 0x3808C000, 0x38090000, 0x38094000, 0x38098000, 0x3809C000, 0x380A0000, 0x380A4000, 0x380A8000, 0x380AC000, 0x380B0000, 0x380B4000, 0x380B8000, 0x380BC000,
  0x380C0000, 0x380C4000, 0x380C8000, 0x380CC000, 0x380D0000, 0x380D4000, 0x380D8000, 0x380DC000, 0x380E0000, 0x380E4000, 0x380E8000, 0x380EC000, 0x380F0000, 0x380F4000, 0x380F8000, 0x380FC000,
  0x38100000, 0x38104000, 0x38108000, 0x3810C000, 0x38110000, 0x38114000, 0x38118000, 0x3811C000, 0x38120000, 0x38124000, 0x38128000, 0x3812C000, 0x38130000, 0x38134000, 0x38138000, 0x3813C000,
  0x38140000, 0x38144000, 0x38148000, 0x3814C000, 0x38150000, 0x38154000, 0x38158000, 0x3815C000, 0x38160000, 0x38164000, 0x38168000, 0x3816C000, 0x38170000, 0x38174000, 0x38178000, 0x3817C000,
  0x38180000, 0x38184000, 0x38188000, 0x3818C000, 0x38190000, 0x38194000, 0x38198000, 0x3819C000, 0x381A0000, 0x381A4000, 0x381A8000, 0x381AC000, 0x381B0000, 0x381B4000, 0x381B8000, 0x381BC000,
  0x381C0000, 0x381C4000, 0x381C8000, 0x381CC000, 0x381D0000, 0x381D4000, 0x381D8000, 0x381DC000, 0x381E0000, 0x381E4000, 0x381E8000, 0x381EC000, 0x381F0000, 0x381F4000, 0x381F8000, 0x381FC000,
  0x38200000, 0x38204000, 0x38208000, 0x3820C000, 0x38210000, 0x38214000, 0x38218000, 0x3821C000, 0x38220000, 0x38224000, 0x38228000, 0x3822C000, 0x38230000, 0x38234000, 0x38238000, 0x3823C000,
  0x38240000, 0x38244000, 0x38248000, 0x3824C000, 0x38250000, 0x38254000, 0x38258000, 0x3825C000, 0x38260000, 0x38264000, 0x38268000, 0x3826C000, 0x38270000, 0x38274000, 0x38278000, 0x3827C000,
  0x38280000, 0x38284000, 0x38288000, 0x3828C000, 0x38290000, 0x38294000, 0x38298000, 0x3829C000, 0x382A0000, 0x382A4000, 0x382A8000, 0x382AC000, 0x382B0000, 0x382B4000, 0x382B8000, 0x382BC000,
  0x382C0000, 0x382C4000, 0x382C8000, 0x382CC000, 0x382D0000, 0x382D4000, 0x382D8000, 0x382DC000, 0x382E0000, 0x382E4000, 0x382E8000, 0x382EC000, 0x382F0000, 0x382F4000, 0x382F8000, 0x382FC000,
  0x38300000, 0x38304000, 0x38308000, 0x3830C000, 0x38310000, 0x38314000, 0x38318000, 0x3831C000, 0x38320000, 0x38324000, 0x38328000, 0x3832C000, 0x38330000, 0x38334000, 0x38338000, 0x3833C000,
  0x38340000, 0x38344000, 0x38348000, 0x3834C000, 0x38350000, 0x38354000, 0x38358000, 0x3835C000, 0x38360000, 0x38364000, 0x38368000, 0x3836C000, 0x38370000, 0x38374000, 0x38378000, 0x3837C000,
  0x38380000, 0x38384000, 0x38388000, 0x3838C000, 0x38390000, 0x38394000, 0x38398000, 0x3839C000, 0x383A0000, 0x383A4000, 0x383A8000, 0x383AC000, 0x383B0000, 0x383B4000, 0x383B8000, 0x383BC000,
  0x383C0000, 0x383C4000, 0x383C8000, 0x383CC000, 0x383D0000, 0x383D4000, 0x383D8000, 0x383DC000, 0x383E0000, 0x383E4000, 0x383E8000, 0x383EC000, 0x383F0000, 0x383F4000, 0x383F8000, 0x383FC000,
  0x38400000, 0x38404000, 0x38408000, 0x3840C000, 0x38410000, 0x38414000, 0x38418000, 0x3841C000, 0x38420000, 0x38424000, 0x38428000, 0x3842C000, 0x38430000, 0x38434000, 0x38438000, 0x3843C000,
  0x38440000, 0x38444000, 0x38448000, 0x3844C000, 0x38450000, 0x38454000, 0x38458000, 0x3845C000, 0x38460000, 0x38464000, 0x38468000, 0x3846C000, 0x38470000, 0x38474000, 0x38478000, 0x3847C000,
  0x38480000, 0x38484000, 0x38488000, 0x3848C000, 0x38490000, 0x38494000, 0x38498000, 0x3849C000, 0x384A0000, 0x384A4000, 0x384A8000, 0x384AC000, 0x384B0000, 0x384B4000, 0x384B8000, 0x384BC000,
  0x384C0000, 0x384C4000, 0x384C8000, 0x384CC000, 0x384D0000, 0x384D4000, 0x384D8000, 0x384DC000, 0x384E0000, 0x384E4000, 0x384E8000, 0x384EC000, 0x384F0000, 0x384F4000, 0x384F8000, 0x384FC000,
  0x38500000, 0x38504000, 0x38508000, 0x3850C000, 0x38510000, 0x38514000, 0x38518000, 0x3851C000, 0x38520000, 0x38524000, 0x38528000, 0x3852C000, 0x38530000, 0x38534000, 0x38538000, 0x3853C000,
  0x38540000, 0x38544000, 0x38548000, 0x3854C000, 0x38550000, 0x38554000, 0x38558000, 0x3855C000, 0x38560000, 0x38564000, 0x38568000, 0x3856C000, 0x38570000, 0x38574000, 0x38578000, 0x3857C000,
  0x38580000, 0x38584000, 0x38588000, 0x3858C000, 0x38590000, 0x38594000, 0x38598000, 0x3859C000, 0x385A0000, 0x385A4000, 0x385A8000, 0x385AC000, 0x385B0000, 0x385B4000, 0x385B8000, 0x385BC000,
  0x385C0000, 0x385C4000, 0x385C8000, 0x385CC000, 0x385D0000, 0x385D4000, 0x385D8000, 0x385DC000, 0x385E0000, 0x385E4000, 0x385E8000, 0x385EC000, 0x385F0000, 0x385F4000, 0x385F8000, 0x385FC000,
  0x38600000, 0x38604000, 0x38608000, 0x3860C000, 0x38610000, 0x38614000, 0x38618000, 0x3861C000, 0x38620000, 0x38624000, 0x38628000, 0x3862C000, 0x38630000, 0x38634000, 0x38638000, 0x3863C000,
  0x38640000, 0x38644000, 0x38648000, 0x3864C000, 0x38650000, 0x38654000, 0x38658000, 0x3865C000, 0x38660000, 0x38664000, 0x38668000, 0x3866C000, 0x38670000, 0x38674000, 0x38678000, 0x3867C000,
  0x38680000, 0x38684000, 0x38688000, 0x3868C000, 0x38690000, 0x38694000, 0x38698000, 0x3869C000, 0x386A0000, 0x386A4000, 0x386A8000, 0x386AC000, 0x386B0000, 0x386B4000, 0x386B8000, 0x386BC000,
  0x386C0000, 0x386C4000, 0x386C8000, 0x386CC000, 0x386D0000, 0x386D4000, 0x386D8000, 0x386DC000, 0x386E0000, 0x386E4000, 0x386E8000, 0x386EC000, 0x386F0000, 0x386F4000, 0x386F8000, 0x386FC000,
  0x38700000, 0x38704000, 0x38708000, 0x3870C000, 0x38710000, 0x38714000, 0x38718000, 0x3871C000, 0x38720000, 0x38724000, 0x38728000, 0x3872C000, 0x38730000, 0x38734000, 0x38738000, 0x3873C000,
  0x38740000, 0x38744000, 0x38748000, 0x3874C000, 0x38750000, 0x38754000, 0x38758000, 0x3875C000, 0x38760000, 0x38764000, 0x38768000, 0x3876C000, 0x38770000, 0x38774000, 0x38778000, 0x3877C000,
  0x38780000, 0x38784000, 0x38788000, 0x3878C000, 0x38790000, 0x38794000, 0x38798000, 0x3879C000, 0x387A0000, 0x387A4000, 0x387A8000, 0x387AC000, 0x387B0000, 0x387B4000, 0x387B8000, 0x387BC000,
  0x387C0000, 0x387C4000, 0x387C8000, 0x387CC000, 0x387D0000, 0x387D4000, 0x387D8000, 0x387DC000, 0x387E0000, 0x387E4000, 0x387E8000, 0x387EC000, 0x387F0000, 0x387F4000, 0x387F8000, 0x387FC000,
  0x38000000, 0x38002000, 0x38004000, 0x38006000, 0x38008000, 0x3800A000, 0x3800C000, 0x3800E000, 0x38010000, 0x38012000, 0x38014000, 0x38016000, 0x38018000, 0x3801A000, 0x3801C000, 0x3801E000,
  0x38020000, 0x38022000, 0x38024000, 0x38026000, 0x38028000, 0x3802A000, 0x3802C000, 0x3802E000, 0x38030000, 0x38032000, 0x38034000, 0x38036000, 0x38038000, 0x3803A000, 0x3803C000, 0x3803E000,
  0x38040000, 0x38042000, 0x38044000, 0x38046000, 0x38048000, 0x3804A000, 0x3804C000, 0x3804E000, 0x38050000, 0x38052000, 0x38054000, 0x38056000, 0x38058000, 0x3805A000, 0x3805C000, 0x3805E000,
  0x38060000, 0x38062000, 0x38064000, 0x38066000, 0x38068000, 0x3806A000, 0x3806C000, 0x3806E000, 0x38070000, 0x38072000, 0x38074000, 0x38076000, 0x38078000, 0x3807A000, 0x3807C000, 0x3807E000,
  0x38080000, 0x38082000, 0x38084000, 0x38086000, 0x38088000, 0x3808A000, 0x3808C000, 0x3808E000, 0x38090000, 0x38092000, 0x38094000, 0x38096000, 0x38098000, 0x3809A000, 0x3809C000, 0x3809E000,
  0x380A0000, 0x380A2000, 0x380A4000, 0x380A6000, 0x380A8000, 0x380AA000, 0x380AC000, 0x380AE000, 0x380B0000, 0x380B2000, 0x380B4000, 0x380B6000, 0x380B8000, 0x380BA000, 0x380BC000, 0x380BE000,
  0x380C0000, 0x380C2000, 0x380C4000, 0x380C6000, 0x380C8000, 0x380CA000, 0x380CC000, 0x380CE000, 0x380D0000, 0x380D2000, 0x380D4000, 0x380D6000, 0x380D8000, 0x380DA000, 0x380DC000, 0x380DE000,
  0x380E0000, 0x380E2000, 0x380E4000, 0x380E6000, 0x380E8000, 0x380EA000, 0x380EC000, 0x380EE000, 0x380F0000, 0x380F2000, 0x380F4000, 0x380F6000, 0x380F8000, 0x380FA000, 0x380FC000, 0x380FE000,
  0x38100000, 0x38102000, 0x38104000, 0x38106000, 0x38108000, 0x3810A000, 0x3810C000, 0x3810E000, 0x38110000, 0x38112000, 0x38114000, 0x38116000, 0x38118000, 0x3811A000, 0x3811C000, 0x3811E000,
  0x38120000, 0x38122000, 0x38124000, 0x38126000, 0x38128000, 0x3812A000, 0x3812C000, 0x3812E000, 0x38130000, 0x38132000, 0x38134000, 0x38136000, 0x38138000, 0x3813A000, 0x3813C000, 0x3813E000,
  0x38140000, 0x38142000, 0x38144000, 0x38146000, 0x38148000, 0x3814A000, 0x3814C000, 0x3814E000, 0x38150000, 0x38152000, 0x38154000, 0x38156000, 0x38158000, 0x3815A000, 0x3815C000, 0x3815E000,
  0x38160000, 0x38162000, 0x38164000, 0x38166000, 0x38168000, 0x3816A000, 0x3816C000, 0x3816E000, 0x38170000, 0x38172000, 0x38174000, 0x38176000, 0x38178000, 0x3817A000, 0x3817C000, 0x3817E000,
  0x38180000, 0x38182000, 0x38184000, 0x38186000, 0x38188000, 0x3818A000, 0x3818C000, 0x3818E000, 0x38190000, 0x38192000, 0x38194000, 0x38196000, 0x38198000, 0x3819A000, 0x3819C000, 0x3819E000,
  0x381A0000, 0x381A2000, 0x381A4000, 0x381A6000, 0x381A8000, 0x381AA000, 0x381AC000, 0x381AE000, 0x381B0000, 0x381B2000, 0x381B4000, 0x381B6000, 0x381B8000, 0x381BA000, 0x381BC000, 0x381BE000,
  0x381C0000, 0x381C2000, 0x381C4000, 0x381C6000, 0x381C8000, 0x381CA000, 0x381CC000, 0x381CE000, 0x381D0000, 0x381D2000, 0x381D4000, 0x381D6000, 0x381D8000, 0x381DA000, 0x381DC000, 0x381DE000,
  0x381E0000, 0x381E2000, 0x381E4000, 0x381E6000, 0x381E8000, 0x381EA000, 0x381EC000, 0x381EE000, 0x381F0000, 0x381F2000, 0x381F4000, 0x381F6000, 0x381F8000, 0x381FA000, 0x381FC000, 0x381FE000,
  0x38200000, 0x38202000, 0x38204000, 0x38206000, 0x38208000, 0x3820A000, 0x3820C000, 0x3820E000, 0x38210000, 0x38212000, 0x38214000, 0x38216000, 0x38218000, 0x3821A000, 0x3821C000, 0x3821E000,
  0x38220000, 0x38222000, 0x38224000, 0x38226000, 0x38228000, 0x3822A000, 0x3822C000, 0x3822E000, 0x38230000, 0x38232000, 0x38234000, 0x38236000, 0x38238000, 0x3823A000, 0x3823C000, 0x3823E000,
  0x38240000, 0x38242000, 0x38244000, 0x38246000, 0x38248000, 0x3824A000, 0x3824C000, 0x3824E000, 0x38250000, 0x38252000, 0x38254000, 0x38256000, 0x38258000, 0x3825A000, 0x3825C000, 0x3825E000,
  0x38260000, 0x38262000, 0x38264000, 0x38266000, 0x38268000, 0x3826A000, 0x3826C000, 0x3826E000, 0x38270000, 0x38272000, 0x38274000, 0x38276000, 0x38278000, 0x3827A000, 0x3827C000, 0x3827E000,
  0x38280000, 0x38282000, 0x38284000, 0x38286000, 0x38288000, 0x3828A000, 0x3828C000, 0x3828E000, 0x38290000, 0x38292000, 0x38294000, 0x38296000, 0x38298000, 0x3829A000, 0x3829C000, 0x3829E000,
  0x382A0000, 0x382A2000, 0x382A4000, 0x382A6000, 0x382A8000, 0x382AA000, 0x382AC000, 0x382AE000, 0x382B0000, 0x382B2000, 0x382B4000, 0x382B6000, 0x382B8000, 0x382BA000, 0x382BC000, 0x382BE000,
  0x382C0000, 0x382C2000, 0x382C4000, 0x382C6000, 0x382C8000, 0x382CA000, 0x382CC000, 0x382CE000, 0x382D0000, 0x382D2000, 0x382D4000, 0x382D6000, 0x382D8000, 0x382DA000, 0x382DC000, 0x382DE000,
  0x382E0000, 0x382E2000, 0x382E4000, 0x382E6000, 0x382E8000, 0x382EA000, 0x382EC000, 0x382EE000, 0x382F0000, 0x382F2000, 0x382F4000, 0x382F6000, 0x382F8000, 0x382FA000, 0x382FC000, 0x382FE000,
  0x38300000, 0x38302000, 0x38304000, 0x38306000, 0x38308000, 0x3830A000, 0x3830C000, 0x3830E000, 0x38310000, 0x38312000, 0x38314000, 0x38316000, 0x38318000, 0x3831A000, 0x3831C000, 0x3831E000,
  0x38320000, 0x38322000, 0x38324000, 0x38326000, 0x38328000, 0x3832A000, 0x3832C000, 0x3832E000, 0x38330000, 0x38332000, 0x38334000, 0x38336000, 0x38338000, 0x3833A000, 0x3833C000, 0x3833E000,
  0x38340000, 0x38342000, 0x38344000, 0x38346000, 0x38348000, 0x3834A000, 0x3834C000, 0x3834E000, 0x38350000, 0x38352000, 0x38354000, 0x38356000, 0x38358000, 0x3835A000, 0x3835C000, 0x3835E000,
  0x38360000, 0x38362000, 0x38364000, 0x38366000, 0x38368000, 0x3836A000, 0x3836C000, 0x3836E000, 0x38370000, 0x38372000, 0x38374000, 0x38376000, 0x38378000, 0x3837A000, 0x3837C000, 0x3837E000,
  0x38380000, 0x38382000, 0x38384000, 0x38386000, 0x38388000, 0x3838A000, 0x3838C000, 0x3838E000, 0x38390000, 0x38392000, 0x38394000, 0x38396000, 0x38398000, 0x3839A000, 0x3839C000, 0x3839E000,
  0x383A0000, 0x383A2000, 0x383A4000, 0x383A6000, 0x383A8000, 0x383AA000, 0x383AC000, 0x383AE000, 0x383B0000, 0x383B2000, 0x383B4000, 0x383B6000, 0x383B8000, 0x383BA000, 0x383BC000, 0x383BE000,
  0x383C0000, 0x383C2000, 0x383C4000, 0x383C6000, 0x383C8000, 0x383CA000, 0x383CC000, 0x383CE000, 0x383D0000, 0x383D2000, 0x383D4000, 0x383D6000, 0x383D8000, 0x383DA000, 0x383DC000, 0x383DE000,
  0x383E0000, 0x383E2000, 0x383E4000, 0x383E6000, 0x383E8000, 0x383EA000, 0x383EC000, 0x383EE000, 0x383F0000, 0x383F2000, 0x383F4000, 0x383F6000, 0x383F8000, 0x383FA000, 0x383FC000, 0x383FE000,
  0x38400000, 0x38402000, 0x38404000, 0x38406000, 0x38408000, 0x3840A000, 0x3840C000, 0x3840E000, 0x38410000, 0x38412000, 0x38414000, 0x38416000, 0x38418000, 0x3841A000, 0x3841C000, 0x3841E000,
  0x38420000, 0x38422000, 0x38424000, 0x38426000, 0x38428000, 0x3842A000, 0x3842C000, 0x3842E000, 0x38430000, 0x38432000, 0x38434000, 0x38436000, 0x38438000, 0x3843A000, 0x3843C000, 0x3843E000,
  0x38440000, 0x38442000, 0x38444000, 0x38446000, 0x38448000, 0x3844A000, 0x3844C000, 0x3844E000, 0x38450000, 0x38452000, 0x38454000, 0x38456000, 0x38458000, 0x3845A000, 0x3845C000, 0x3845E000,
  0x38460000, 0x38462000, 0x38464000, 0x38466000, 0x38468000, 0x3846A000, 0x3846C000, 0x3846E000, 0x38470000, 0x38472000, 0x38474000, 0x38476000, 0x38478000, 0x3847A000, 0x3847C000, 0x3847E000,
  0x38480000, 0x38482000, 0x38484000, 0x38486000, 0x38488000, 0x3848A000, 0x3848C000, 0x3848E000, 0x38490000, 0x38492000, 0x38494000, 0x38496000, 0x38498000, 0x3849A000, 0x3849C000, 0x3849E000,
  0x384A0000, 0x384A2000, 0x384A4000, 0x384A6000, 0x384A8000, 0x384AA000, 0x384AC000, 0x384AE000, 0x384B0000, 0x384B2000, 0x384B4000, 0x384B6000, 0x384B8000, 0x384BA000, 0x384BC000, 0x384BE000,
  0x384C0000, 0x384C2000, 0x384C4000, 0x384C6000, 0x384C8000, 0x384CA000, 0x384CC000, 0x384CE000, 0x384D0000, 0x384D2000, 0x384D4000, 0x384D6000, 0x384D8000, 0x384DA000, 0x384DC000, 0x384DE000,
  0x384E0000, 0x384E2000, 0x384E4000, 0x384E6000, 0x384E8000, 0x384EA000, 0x384EC000, 0x384EE000, 0x384F0000, 0x384F2000, 0x384F4000, 0x384F6000, 0x384F8000, 0x384FA000, 0x384FC000, 0x384FE000,
  0x38500000, 0x38502000, 0x38504000, 0x38506000, 0x38508000, 0x3850A000, 0x3850C000, 0x3850E000, 0x38510000, 0x38512000, 0x38514000, 0x38516000, 0x38518000, 0x3851A000, 0x3851C000, 0x3851E000,
  0x38520000, 0x38522000, 0x38524000, 0x38526000, 0x38528000, 0x3852A000, 0x3852C000, 0x3852E000, 0x38530000, 0x38532000, 0x38534000, 0x38536000, 0x38538000, 0x3853A000, 0x3853C000, 0x3853E000,
  0x38540000, 0x38542000, 0x38544000, 0x38546000, 0x38548000, 0x3854A000, 0x3854C000, 0x3854E000, 0x38550000, 0x38552000, 0x38554000, 0x38556000, 0x38558000, 0x3855A000, 0x3855C000, 0x3855E000,
  0x38560000, 0x38562000, 0x38564000, 0x38566000, 0x38568000, 0x3856A000, 0x3856C000, 0x3856E000, 0x38570000, 0x38572000, 0x38574000, 0x38576000, 0x38578000, 0x3857A000, 0x3857C000, 0x3857E000,
  0x38580000, 0x38582000, 0x38584000, 0x38586000, 0x38588000, 0x3858A000, 0x3858C000, 0x3858E000, 0x38590000, 0x38592000, 0x38594000, 0x38596000, 0x38598000, 0x3859A000, 0x3859C000, 0x3859E000,
  0x385A0000, 0x385A2000, 0x385A4000, 0x385A6000, 0x385A8000, 0x385AA000, 0x385AC000, 0x385AE000, 0x385B0000, 0x385B2000, 0x385B4000, 0x385B6000, 0x385B8000, 0x385BA000, 0x385BC000, 0x385BE000,
  0x385C0000, 0x385C2000, 0x385C4000, 0x385C6000, 0x385C8000, 0x385CA000, 0x385CC000, 0x385CE000, 0x385D0000, 0x385D2000, 0x385D4000, 0x385D6000, 0x385D8000, 0x385DA000, 0x385DC000, 0x385DE000,
  0x385E0000, 0x385E2000, 0x385E4000, 0x385E6000, 0x385E8000, 0x385EA000, 0x385EC000, 0x385EE000, 0x385F0000, 0x385F2000, 0x385F4000, 0x385F6000, 0x385F8000, 0x385FA000, 0x385FC000, 0x385FE000,
  0x38600000, 0x38602000, 0x38604000, 0x38606000, 0x38608000, 0x3860A000, 0x3860C000, 0x3860E000, 0x38610000, 0x38612000, 0x38614000, 0x38616000, 0x38618000, 0x3861A000, 0x3861C000, 0x3861E000,
  0x38620000, 0x38622000, 0x38624000, 0x38626000, 0x38628000, 0x3862A000, 0x3862C000, 0x3862E000, 0x38630000, 0x38632000, 0x38634000, 0x38636000, 0x38638000, 0x3863A000, 0x3863C000, 0x3863E000,
  0x38640000, 0x38642000, 0x38644000, 0x38646000, 0x38648000, 0x3864A000, 0x3864C000, 0x3864E000, 0x38650000, 0x38652000, 0x38654000, 0x38656000, 0x38658000, 0x3865A000, 0x3865C000, 0x3865E000,
  0x38660000, 0x38662000, 0x38664000, 0x38666000, 0x38668000, 0x3866A000, 0x3866C000, 0x3866E000, 0x38670000, 0x38672000, 0x38674000, 0x38676000, 0x38678000, 0x3867A000, 0x3867C000, 0x3867E000,
  0x38680000, 0x38682000, 0x38684000, 0x38686000, 0x38688000, 0x3868A000, 0x3868C000, 0x3868E000, 0x38690000, 0x38692000, 0x38694000, 0x38696000, 0x38698000, 0x3869A000, 0x3869C000, 0x3869E000,
  0x386A0000, 0x386A2000, 0x386A4000, 0x386A6000, 0x386A8000, 0x386AA000, 0x386AC000, 0x386AE000, 0x386B0000, 0x386B2000, 0x386B4000, 0x386B6000, 0x386B8000, 0x386BA000, 0x386BC000, 0x386BE000,
  0x386C0000, 0x386C2000, 0x386C4000, 0x386C6000, 0x386C8000, 0x386CA000, 0x386CC000, 0x386CE000, 0x386D0000, 0x386D2000, 0x386D4000, 0x386D6000, 0x386D8000, 0x386DA000, 0x386DC000, 0x386DE000,
  0x386E0000, 0x386E2000, 0x386E4000, 0x386E6000, 0x386E8000, 0x386EA000, 0x386EC000, 0x386EE000, 0x386F0000, 0x386F2000, 0x386F4000, 0x386F6000, 0x386F8000, 0x386FA000, 0x386FC000, 0x386FE000,
  0x38700000, 0x38702000, 0x38704000, 0x38706000, 0x38708000, 0x3870A000, 0x3870C000, 0x3870E000, 0x38710000, 0x38712000, 0x38714000, 0x38716000, 0x38718000, 0x3871A000, 0x3871C000, 0x3871E000,
  0x38720000, 0x38722000, 0x38724000, 0x38726000, 0x38728000, 0x3872A000, 0x3872C000, 0x3872E000, 0x38730000, 0x38732000, 0x38734000, 0x38736000, 0x38738000, 0x3873A000, 0x3873C000, 0x3873E000,
  0x38740000, 0x38742000, 0x38744000, 0x38746000, 0x38748000, 0x3874A000, 0x3874C000, 0x3874E000, 0x38750000, 0x38752000, 0x38754000, 0x38756000, 0x38758000, 0x3875A000, 0x3875C000, 0x3875E000,
  0x38760000, 0x38762000, 0x38764000, 0x38766000, 0x38768000, 0x3876A000, 0x3876C000, 0x3876E000, 0x38770000, 0x38772000, 0x38774000, 0x38776000, 0x38778000, 0x3877A000, 0x3877C000, 0x3877E000,
  0x38780000, 0x38782000, 0x38784000, 0x38786000, 0x38788000, 0x3878A000, 0x3878C000, 0x3878E000, 0x38790000, 0x38792000, 0x38794000, 0x38796000, 0x38798000, 0x3879A000, 0x3879C000, 0x3879E000,
  0x387A0000, 0x387A2000, 0x387A4000, 0x387A6000, 0x387A8000, 0x387AA000, 0x387AC000, 0x387AE000, 0x387B0000, 0x387B2000, 0x387B4000, 0x387B6000, 0x387B8000, 0x387BA000, 0x387BC000, 0x387BE000,
  0x387C0000, 0x387C2000, 0x387C4000, 0x387C6000, 0x387C8000, 0x387CA000, 0x387CC000, 0x387CE000, 0x387D0000, 0x387D2000, 0x387D4000, 0x387D6000, 0x387D8000, 0x387DA000, 0x387DC000, 0x387DE000,
  0x387E0000, 0x387E2000, 0x387E4000, 0x387E6000, 0x387E8000, 0x387EA000, 0x387EC000, 0x387EE000, 0x387F0000, 0x387F2000, 0x387F4000, 0x387F6000, 0x387F8000, 0x387FA000, 0x387FC000, 0x387FE000 };
__constant static const uint32_t exponent_table[64] = {
  0x00000000, 0x00800000, 0x01000000, 0x01800000, 0x02000000, 0x02800000, 0x03000000, 0x03800000, 0x04000000, 0x04800000, 0x05000000, 0x05800000, 0x06000000, 0x06800000, 0x07000000, 0x07800000,
  0x08000000, 0x08800000, 0x09000000, 0x09800000, 0x0A000000, 0x0A800000, 0x0B000000, 0x0B800000, 0x0C000000, 0x0C800000, 0x0D000000, 0x0D800000, 0x0E000000, 0x0E800000, 0x0F000000, 0x47800000,
  0x80000000, 0x80800000, 0x81000000, 0x81800000, 0x82000000, 0x82800000, 0x83000000, 0x83800000, 0x84000000, 0x84800000, 0x85000000, 0x85800000, 0x86000000, 0x86800000, 0x87000000, 0x87800000,
  0x88000000, 0x88800000, 0x89000000, 0x89800000, 0x8A000000, 0x8A800000, 0x8B000000, 0x8B800000, 0x8C000000, 0x8C800000, 0x8D000000, 0x8D800000, 0x8E000000, 0x8E800000, 0x8F000000, 0xC7800000 };
__constant static const unsigned short offset_table[64] = {
  0, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024,
  0, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024, 1024 };

SCALAR_FUN_ATTR uint16_t float2halfbits(float value) {
  union { float x; uint32_t y; } u;
  u.x = value;
  uint32_t bits = u.y;

  uint16_t hbits = base_table[bits>>23] + (uint16_t)((bits&0x7FFFFF)>>shift_table[bits>>23]);;

  return hbits;
}

SCALAR_FUN_ATTR float halfbits2float(uint16_t value) {
  uint32_t bits = mantissa_table[offset_table[value>>10]+(value&0x3FF)] + exponent_table[value>>10];

  union { uint32_t x; float y; } u;
  u.x = bits;
  return u.y;
}

SCALAR_FUN_ATTR uint16_t halfbitsnextafter(uint16_t from, uint16_t to) {
  int fabs = from & 0x7FFF, tabs = to & 0x7FFF;
  if(fabs > 0x7C00 || tabs > 0x7C00) {
    return ((from&0x7FFF)>0x7C00) ? (from|0x200) : (to|0x200);
  }
  if(from == to || !(fabs|tabs)) {
    return to;
  }
  if(!fabs) {
    return (to&0x8000)+1;
  }
  unsigned int out =
    from +
    (((from>>15)^(unsigned int)((from^(0x8000|(0x8000-(from>>15))))<(to^(0x8000|(0x8000-(to>>15))))))<<1)
    - 1;
  return out;
}

// End of half.h.
// Start of timing.h.

// The function get_wall_time() returns the wall time in microseconds
// (with an unspecified offset).

#ifdef _WIN32

#define NOGDI
#include <windows.h>

static int64_t get_wall_time(void) {
  LARGE_INTEGER time,freq;
  assert(QueryPerformanceFrequency(&freq));
  assert(QueryPerformanceCounter(&time));
  return ((double)time.QuadPart / freq.QuadPart) * 1000000;
}

static int64_t get_wall_time_ns(void) {
  return get_wall_time() * 1000;
}

#else
// Assuming POSIX

#include <time.h>
#include <sys/time.h>

static int64_t get_wall_time_ns(void) {
  struct timespec time;
  assert(clock_gettime(CLOCK_MONOTONIC, &time) == 0);
  return time.tv_sec * 1000000000 + time.tv_nsec;
}

static int64_t get_wall_time(void) {
  return get_wall_time_ns() / 1000;
}


#endif

// End of timing.h.
// Start of lock.h.

// A very simple cross-platform implementation of locks.  Uses
// pthreads on Unix and some Windows thing there.  Futhark's
// host-level code is not multithreaded, but user code may be, so we
// need some mechanism for ensuring atomic access to API functions.
// This is that mechanism.  It is not exposed to user code at all, so
// we do not have to worry about name collisions.

#ifdef _WIN32

typedef HANDLE lock_t;

static void create_lock(lock_t *lock) {
  *lock = CreateMutex(NULL,  // Default security attributes.
                      FALSE, // Initially unlocked.
                      NULL); // Unnamed.
}

static void lock_lock(lock_t *lock) {
  assert(WaitForSingleObject(*lock, INFINITE) == WAIT_OBJECT_0);
}

static void lock_unlock(lock_t *lock) {
  assert(ReleaseMutex(*lock));
}

static void free_lock(lock_t *lock) {
  CloseHandle(*lock);
}

#else
// Assuming POSIX

#include <pthread.h>

typedef pthread_mutex_t lock_t;

static void create_lock(lock_t *lock) {
  int r = pthread_mutex_init(lock, NULL);
  assert(r == 0);
}

static void lock_lock(lock_t *lock) {
  int r = pthread_mutex_lock(lock);
  assert(r == 0);
}

static void lock_unlock(lock_t *lock) {
  int r = pthread_mutex_unlock(lock);
  assert(r == 0);
}

static void free_lock(lock_t *lock) {
  // Nothing to do for pthreads.
  (void)lock;
}

#endif

// End of lock.h.
// Start of free_list.h.

typedef uintptr_t fl_mem;

// An entry in the free list.  May be invalid, to avoid having to
// deallocate entries as soon as they are removed.  There is also a
// tag, to help with memory reuse.
struct free_list_entry {
  size_t size;
  fl_mem mem;
  const char *tag;
  unsigned char valid;
};

struct free_list {
  struct free_list_entry *entries; // Pointer to entries.
  int capacity;                    // Number of entries.
  int used;                        // Number of valid entries.
  lock_t lock;                     // Thread safety.
};

static void free_list_init(struct free_list *l) {
  l->capacity = 30; // Picked arbitrarily.
  l->used = 0;
  l->entries = (struct free_list_entry*) malloc(sizeof(struct free_list_entry) * l->capacity);
  for (int i = 0; i < l->capacity; i++) {
    l->entries[i].valid = 0;
  }
  create_lock(&l->lock);
}

// Remove invalid entries from the free list.
static void free_list_pack(struct free_list *l) {
  lock_lock(&l->lock);
  int p = 0;
  for (int i = 0; i < l->capacity; i++) {
    if (l->entries[i].valid) {
      l->entries[p] = l->entries[i];
      if (i > p) {
        l->entries[i].valid = 0;
      }
      p++;
    }
  }

  // Now p is the number of used elements.  We don't want it to go
  // less than the default capacity (although in practice it's OK as
  // long as it doesn't become 1).
  if (p < 30) {
    p = 30;
  }
  l->entries = realloc(l->entries, p * sizeof(struct free_list_entry));
  l->capacity = p;
  lock_unlock(&l->lock);
}

static void free_list_destroy(struct free_list *l) {
  assert(l->used == 0);
  free(l->entries);
  free_lock(&l->lock);
}

// Not part of the interface, so no locking.
static int free_list_find_invalid(struct free_list *l) {
  int i;
  for (i = 0; i < l->capacity; i++) {
    if (!l->entries[i].valid) {
      break;
    }
  }
  return i;
}

static void free_list_insert(struct free_list *l, size_t size, fl_mem mem, const char *tag) {
  lock_lock(&l->lock);
  int i = free_list_find_invalid(l);

  if (i == l->capacity) {
    // List is full; so we have to grow it.
    int new_capacity = l->capacity * 2 * sizeof(struct free_list_entry);
    l->entries = realloc(l->entries, new_capacity);
    for (int j = 0; j < l->capacity; j++) {
      l->entries[j+l->capacity].valid = 0;
    }
    l->capacity *= 2;
  }

  // Now 'i' points to the first invalid entry.
  l->entries[i].valid = 1;
  l->entries[i].size = size;
  l->entries[i].mem = mem;
  l->entries[i].tag = tag;

  l->used++;
  lock_unlock(&l->lock);
}

// Determine whether this entry in the free list is acceptable for
// satisfying the request.  Not public, so no locking.
static bool free_list_acceptable(size_t size, const char* tag, struct free_list_entry *entry) {
  // We check not just the hard requirement (is the entry acceptable
  // and big enough?) but also put a cap on how much wasted space
  // (internal fragmentation) we allow.  This is necessarily a
  // heuristic, and a crude one.

  if (!entry->valid) {
    return false;
  }

  if (size > entry->size) {
    return false;
  }

  // We know the block fits.  Now the question is whether it is too
  // big.  Our policy is as follows:
  //
  // 1) We don't care about wasted space below 4096 bytes (to avoid
  // churn in tiny allocations).
  //
  // 2) If the tag matches, we allow _any_ amount of wasted space.
  //
  // 3) Otherwise we allow up to 50% wasted space.

  if (entry->size < 4096) {
    return true;
  }

  if (entry->tag == tag) {
    return true;
  }

  if (entry->size < size * 2) {
    return true;
  }

  return false;
}

// Find and remove a memory block of the indicated tag, or if that
// does not exist, another memory block with exactly the desired size.
// Returns 0 on success.
static int free_list_find(struct free_list *l, size_t size, const char *tag,
                          size_t *size_out, fl_mem *mem_out) {
  lock_lock(&l->lock);
  int size_match = -1;
  int i;
  int ret = 1;
  for (i = 0; i < l->capacity; i++) {
    if (free_list_acceptable(size, tag, &l->entries[i]) &&
        (size_match < 0 || l->entries[i].size < l->entries[size_match].size)) {
      // If this entry is valid, has sufficient size, and is smaller than the
      // best entry found so far, use this entry.
      size_match = i;
    }
  }

  if (size_match >= 0) {
    l->entries[size_match].valid = 0;
    *size_out = l->entries[size_match].size;
    *mem_out = l->entries[size_match].mem;
    l->used--;
    ret = 0;
  }
  lock_unlock(&l->lock);
  return ret;
}

// Remove the first block in the free list.  Returns 0 if a block was
// removed, and nonzero if the free list was already empty.
static int free_list_first(struct free_list *l, fl_mem *mem_out) {
  lock_lock(&l->lock);
  int ret = 1;
  for (int i = 0; i < l->capacity; i++) {
    if (l->entries[i].valid) {
      l->entries[i].valid = 0;
      *mem_out = l->entries[i].mem;
      l->used--;
      ret = 0;
      break;
    }
  }
  lock_unlock(&l->lock);
  return ret;
}

// End of free_list.h.
// Start of event_list.h

typedef int (*event_report_fn)(struct str_builder*, void*);

// A collection of key-value associations. Used to associate extra data with
// events.
struct kvs {
  // A buffer that contains all value data. Must be freed when the struct kvs is
  // no longer used.
  char *buf;

  // Size of buf in bytes.
  size_t buf_size;

  // Number of bytes used in buf.
  size_t buf_used;

  // Number of associations stored.
  size_t n;

  // Capacity of vals.
  size_t vals_capacity;

  // An array of keys.
  const char* *keys;

  // Indexes into 'buf' that contains the values as zero-terminated strings.
  size_t *vals;
};

static const size_t KVS_INIT_BUF_SIZE = 128;
static const size_t KVS_INIT_NUMKEYS = 8;

void kvs_init(struct kvs* kvs) {
  kvs->buf = malloc(KVS_INIT_BUF_SIZE);
  kvs->buf_size = KVS_INIT_BUF_SIZE;
  kvs->buf_used = 0;
  kvs->vals_capacity = KVS_INIT_NUMKEYS;
  kvs->keys = calloc(kvs->vals_capacity, sizeof(const char*));
  kvs->vals = calloc(kvs->vals_capacity, sizeof(size_t));
  kvs->n = 0;
}

struct kvs* kvs_new(void) {
  struct kvs *kvs = malloc(sizeof(struct kvs));
  kvs_init(kvs);
  return kvs;
}

void kvs_printf(struct kvs* kvs, const char* key, const char* fmt, ...) {
  va_list vl;
  va_start(vl, fmt);

  size_t needed = 1 + (size_t)vsnprintf(NULL, 0, fmt, vl);

  while (kvs->buf_used+needed > kvs->buf_size) {
    kvs->buf_size *= 2;
    kvs->buf = realloc(kvs->buf, kvs->buf_size * sizeof(const char*));
  }

  if (kvs->n == kvs->vals_capacity) {
    kvs->vals_capacity *= 2;
    kvs->vals = realloc(kvs->vals, kvs->vals_capacity * sizeof(size_t));
    kvs->keys = realloc(kvs->keys, kvs->vals_capacity * sizeof(char*));
  }

  kvs->keys[kvs->n] = key;
  kvs->vals[kvs->n] = kvs->buf_used;
  kvs->buf_used += needed;

  va_start(vl, fmt); // Must re-init.
  vsnprintf(&kvs->buf[kvs->vals[kvs->n]], needed, fmt, vl);

  kvs->n++;
}

void kvs_free(struct kvs* kvs) {
  free(kvs->vals);
  free(kvs->keys);
  free(kvs->buf);
}

// Assumes all of the values are valid JSON objects.
void kvs_json(const struct kvs* kvs, struct str_builder *sb) {
  str_builder_char(sb, '{');
  for (size_t i = 0; i < kvs->n; i++) {
    if (i != 0) {
      str_builder_str(sb, ",");
    }
    str_builder_json_str(sb, kvs->keys[i]);
    str_builder_str(sb, ":");
    str_builder_str(sb, &kvs->buf[kvs->vals[i]]);
  }
  str_builder_char(sb, '}');
}

void kvs_log(const struct kvs* kvs, const char* prefix, FILE* f) {
  for (size_t i = 0; i < kvs->n; i++) {
    fprintf(f, "%s%s: %s\n",
            prefix,
            kvs->keys[i],
            &kvs->buf[kvs->vals[i]]);
  }
}

struct event {
  void* data;
  event_report_fn f;
  const char* name;
  const char *provenance;
  // Key-value information that is also to be printed.
  struct kvs *kvs;
};

struct event_list {
  struct event *events;
  int num_events;
  int capacity;
};

static void event_list_init(struct event_list *l) {
  l->capacity = 100;
  l->num_events = 0;
  l->events = calloc(l->capacity, sizeof(struct event));
}

static void event_list_free(struct event_list *l) {
  free(l->events);
}

static void add_event_to_list(struct event_list *l,
                              const char* name,
                              const char* provenance,
                              struct kvs *kvs,
                              void* data,
                              event_report_fn f) {
  if (l->num_events == l->capacity) {
    l->capacity *= 2;
    l->events = realloc(l->events, l->capacity * sizeof(struct event));
  }
  l->events[l->num_events].name = name;
  l->events[l->num_events].provenance =
    provenance ? provenance : "unknown";
  l->events[l->num_events].kvs = kvs;
  l->events[l->num_events].data = data;
  l->events[l->num_events].f = f;
  l->num_events++;
}

static int report_events_in_list(struct event_list *l,
                                 struct str_builder* sb) {
  int ret = 0;
  for (int i = 0; i < l->num_events; i++) {
    if (i != 0) {
      str_builder_str(sb, ",");
    }
    str_builder_str(sb, "{\"name\":");
    str_builder_json_str(sb, l->events[i].name);
    str_builder_str(sb, ",\"provenance\":");
    str_builder_json_str(sb, l->events[i].provenance);
    if (l->events[i].f(sb, l->events[i].data) != 0) {
      ret = 1;
      break;
    }

    str_builder_str(sb, ",\"details\":");
    if (l->events[i].kvs) {
      kvs_json(l->events[i].kvs, sb);
      kvs_free(l->events[i].kvs);
    } else {
      str_builder_str(sb, "{}");
    }

    str_builder(sb, "}");
  }
  event_list_free(l);
  event_list_init(l);
  return ret;
}

// End of event_list.h
#include <getopt.h>
#include <ctype.h>
#include <inttypes.h>
#include <unistd.h>
// Start of values.h.

//// Text I/O

typedef int (*writer)(FILE*, const void*);
typedef int (*bin_reader)(void*);
typedef int (*str_reader)(char *, void*);

struct array_reader {
  char* elems;
  int64_t n_elems_space;
  int64_t elem_size;
  int64_t n_elems_used;
  int64_t *shape;
  str_reader elem_reader;
};

static void skipspaces(FILE *f) {
  int c;
  do {
    c = getc(f);
  } while (isspace(c));

  if (c != EOF) {
    ungetc(c, f);
  }
}

static int constituent(char c) {
  return isalnum(c) || c == '.' || c == '-' || c == '+' || c == '_';
}

// Produces an empty token only on EOF.
static void next_token(FILE *f, char *buf, int bufsize) {
 start:
  skipspaces(f);

  int i = 0;
  while (i < bufsize) {
    int c = getc(f);
    buf[i] = (char)c;

    if (c == EOF) {
      buf[i] = 0;
      return;
    } else if (c == '-' && i == 1 && buf[0] == '-') {
      // Line comment, so skip to end of line and start over.
      for (; c != '\n' && c != EOF; c = getc(f));
      goto start;
    } else if (!constituent((char)c)) {
      if (i == 0) {
        // We permit single-character tokens that are not
        // constituents; this lets things like ']' and ',' be
        // tokens.
        buf[i+1] = 0;
        return;
      } else {
        ungetc(c, f);
        buf[i] = 0;
        return;
      }
    }

    i++;
  }

  buf[bufsize-1] = 0;
}

static int next_token_is(FILE *f, char *buf, int bufsize, const char* expected) {
  next_token(f, buf, bufsize);
  return strcmp(buf, expected) == 0;
}

static void remove_underscores(char *buf) {
  char *w = buf;

  for (char *r = buf; *r; r++) {
    if (*r != '_') {
      *w++ = *r;
    }
  }

  *w++ = 0;
}

static int read_str_elem(char *buf, struct array_reader *reader) {
  int ret;
  if (reader->n_elems_used == reader->n_elems_space) {
    reader->n_elems_space *= 2;
    reader->elems = (char*) realloc(reader->elems,
                                    (size_t)(reader->n_elems_space * reader->elem_size));
  }

  ret = reader->elem_reader(buf, reader->elems + reader->n_elems_used * reader->elem_size);

  if (ret == 0) {
    reader->n_elems_used++;
  }

  return ret;
}

static int read_str_array_elems(FILE *f,
                                char *buf, int bufsize,
                                struct array_reader *reader, int64_t dims) {
  int ret = 1;
  int expect_elem = 1;
  char *knows_dimsize = (char*) calloc((size_t)dims, sizeof(char));
  int cur_dim = (int)dims-1;
  int64_t *elems_read_in_dim = (int64_t*) calloc((size_t)dims, sizeof(int64_t));

  while (1) {
    next_token(f, buf, bufsize);
    if (strcmp(buf, "]") == 0) {
      expect_elem = 0;
      if (knows_dimsize[cur_dim]) {
        if (reader->shape[cur_dim] != elems_read_in_dim[cur_dim]) {
          ret = 1;
          break;
        }
      } else {
        knows_dimsize[cur_dim] = 1;
        reader->shape[cur_dim] = elems_read_in_dim[cur_dim];
      }
      if (cur_dim == 0) {
        ret = 0;
        break;
      } else {
        cur_dim--;
        elems_read_in_dim[cur_dim]++;
      }
    } else if (!expect_elem && strcmp(buf, ",") == 0) {
      expect_elem = 1;
    } else if (expect_elem) {
      if (strcmp(buf, "[") == 0) {
        if (cur_dim == dims - 1) {
          ret = 1;
          break;
        }
        cur_dim++;
        elems_read_in_dim[cur_dim] = 0;
      } else if (cur_dim == dims - 1) {
        ret = read_str_elem(buf, reader);
        if (ret != 0) {
          break;
        }
        expect_elem = 0;
        elems_read_in_dim[cur_dim]++;
      } else {
        ret = 1;
        break;
      }
    } else {
      ret = 1;
      break;
    }
  }

  free(knows_dimsize);
  free(elems_read_in_dim);
  return ret;
}

static int read_str_empty_array(FILE *f, char *buf, int bufsize,
                                const char *type_name, int64_t *shape, int64_t dims) {
  if (strlen(buf) == 0) {
    // EOF
    return 1;
  }

  if (strcmp(buf, "empty") != 0) {
    return 1;
  }

  if (!next_token_is(f, buf, bufsize, "(")) {
    return 1;
  }

  for (int i = 0; i < dims; i++) {
    if (!next_token_is(f, buf, bufsize, "[")) {
      return 1;
    }

    next_token(f, buf, bufsize);

    if (sscanf(buf, "%"SCNu64, (uint64_t*)&shape[i]) != 1) {
      return 1;
    }

    if (!next_token_is(f, buf, bufsize, "]")) {
      return 1;
    }
  }

  if (!next_token_is(f, buf, bufsize, type_name)) {
    return 1;
  }


  if (!next_token_is(f, buf, bufsize, ")")) {
    return 1;
  }

  // Check whether the array really is empty.
  for (int i = 0; i < dims; i++) {
    if (shape[i] == 0) {
      return 0;
    }
  }

  // Not an empty array!
  return 1;
}

static int read_str_array(FILE *f,
                          int64_t elem_size, str_reader elem_reader,
                          const char *type_name,
                          void **data, int64_t *shape, int64_t dims) {
  int ret;
  struct array_reader reader;
  char buf[100];

  int dims_seen;
  for (dims_seen = 0; dims_seen < dims; dims_seen++) {
    if (!next_token_is(f, buf, sizeof(buf), "[")) {
      break;
    }
  }

  if (dims_seen == 0) {
    return read_str_empty_array(f, buf, sizeof(buf), type_name, shape, dims);
  }

  if (dims_seen != dims) {
    return 1;
  }

  reader.shape = shape;
  reader.n_elems_used = 0;
  reader.elem_size = elem_size;
  reader.n_elems_space = 16;
  reader.elems = (char*) realloc(*data, (size_t)(elem_size*reader.n_elems_space));
  reader.elem_reader = elem_reader;

  ret = read_str_array_elems(f, buf, sizeof(buf), &reader, dims);

  *data = reader.elems;

  return ret;
}

#define READ_STR(MACRO, PTR, SUFFIX)                                   \
  remove_underscores(buf);                                              \
  int j;                                                                \
  if (sscanf(buf, "%"MACRO"%n", (PTR*)dest, &j) == 1) {                 \
    return !(strcmp(buf+j, "") == 0 || strcmp(buf+j, SUFFIX) == 0);     \
  } else {                                                              \
    return 1;                                                           \
  }

static int read_str_i8(char *buf, void* dest) {
  // Some platforms (WINDOWS) does not support scanf %hhd or its
  // cousin, %SCNi8.  Read into int first to avoid corrupting
  // memory.
  //
  // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=63417
  remove_underscores(buf);
  int j, x;
  if (sscanf(buf, "%i%n", &x, &j) == 1) {
    *(int8_t*)dest = (int8_t)x;
    return !(strcmp(buf+j, "") == 0 || strcmp(buf+j, "i8") == 0);
  } else {
    return 1;
  }
}

static int read_str_u8(char *buf, void* dest) {
  // Some platforms (WINDOWS) does not support scanf %hhd or its
  // cousin, %SCNu8.  Read into int first to avoid corrupting
  // memory.
  //
  // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=63417
  remove_underscores(buf);
  int j, x;
  if (sscanf(buf, "%i%n", &x, &j) == 1) {
    *(uint8_t*)dest = (uint8_t)x;
    return !(strcmp(buf+j, "") == 0 || strcmp(buf+j, "u8") == 0);
  } else {
    return 1;
  }
}

static int read_str_i16(char *buf, void* dest) {
  READ_STR(SCNi16, int16_t, "i16");
}

static int read_str_u16(char *buf, void* dest) {
  READ_STR(SCNi16, int16_t, "u16");
}

static int read_str_i32(char *buf, void* dest) {
  READ_STR(SCNi32, int32_t, "i32");
}

static int read_str_u32(char *buf, void* dest) {
  READ_STR(SCNi32, int32_t, "u32");
}

static int read_str_i64(char *buf, void* dest) {
  READ_STR(SCNi64, int64_t, "i64");
}

static int read_str_u64(char *buf, void* dest) {
  // FIXME: This is not correct, as SCNu64 only permits decimal
  // literals.  However, SCNi64 does not handle very large numbers
  // correctly (it's really for signed numbers, so that's fair).
  READ_STR(SCNu64, uint64_t, "u64");
}

static int read_str_f16(char *buf, void* dest) {
  remove_underscores(buf);
  if (strcmp(buf, "f16.nan") == 0) {
    *(uint16_t*)dest = float2halfbits(NAN);
    return 0;
  } else if (strcmp(buf, "f16.inf") == 0) {
    *(uint16_t*)dest = float2halfbits(INFINITY);
    return 0;
  } else if (strcmp(buf, "-f16.inf") == 0) {
    *(uint16_t*)dest = float2halfbits(-INFINITY);
    return 0;
  } else {
    int j;
    float x;
    if (sscanf(buf, "%f%n", &x, &j) == 1) {
      if (strcmp(buf+j, "") == 0 || strcmp(buf+j, "f16") == 0) {
        *(uint16_t*)dest = float2halfbits(x);
        return 0;
      }
    }
    return 1;
  }
}

static int read_str_f32(char *buf, void* dest) {
  remove_underscores(buf);
  if (strcmp(buf, "f32.nan") == 0) {
    *(float*)dest = (float)NAN;
    return 0;
  } else if (strcmp(buf, "f32.inf") == 0) {
    *(float*)dest = (float)INFINITY;
    return 0;
  } else if (strcmp(buf, "-f32.inf") == 0) {
    *(float*)dest = (float)-INFINITY;
    return 0;
  } else {
    READ_STR("f", float, "f32");
  }
}

static int read_str_f64(char *buf, void* dest) {
  remove_underscores(buf);
  if (strcmp(buf, "f64.nan") == 0) {
    *(double*)dest = (double)NAN;
    return 0;
  } else if (strcmp(buf, "f64.inf") == 0) {
    *(double*)dest = (double)INFINITY;
    return 0;
  } else if (strcmp(buf, "-f64.inf") == 0) {
    *(double*)dest = (double)-INFINITY;
    return 0;
  } else {
    READ_STR("lf", double, "f64");
  }
}

static int read_str_bool(char *buf, void* dest) {
  if (strcmp(buf, "true") == 0) {
    *(char*)dest = 1;
    return 0;
  } else if (strcmp(buf, "false") == 0) {
    *(char*)dest = 0;
    return 0;
  } else {
    return 1;
  }
}

static int read_str_unit(char *buf, void* dest) {
  (void)dest;
  if (strcmp(buf, "()") == 0) {
    return 0;
  } else {
    return 1;
  }
}

static int write_str_i8(FILE *out, const int8_t *src) {
  return fprintf(out, "%hhdi8", *src);
}

static int write_str_u8(FILE *out, const uint8_t *src) {
  return fprintf(out, "%hhuu8", *src);
}

static int write_str_i16(FILE *out, const int16_t *src) {
  return fprintf(out, "%hdi16", *src);
}

static int write_str_u16(FILE *out, const uint16_t *src) {
  return fprintf(out, "%huu16", *src);
}

static int write_str_i32(FILE *out, const int32_t *src) {
  return fprintf(out, "%di32", *src);
}

static int write_str_u32(FILE *out, const uint32_t *src) {
  return fprintf(out, "%uu32", *src);
}

static int write_str_i64(FILE *out, const int64_t *src) {
  return fprintf(out, "%"PRIi64"i64", *src);
}

static int write_str_u64(FILE *out, const uint64_t *src) {
  return fprintf(out, "%"PRIu64"u64", *src);
}

// FLT_DECIMAL_DIG and DBL_DECIMAL_DIG are defined in C11.
// If we want C99 compatibility, we must define them ourselves.
// We choose the standard values on platforms that use the IEEE754 defaults, with fallback to an overestimate.
#ifndef FLT_DECIMAL_DIG
  #if FLT_RADIX == 2 && FLT_MANT_DIG <= 24 && 9 < DECIMAL_DIG
    #define FLT_DECIMAL_DIG 9
  #else
    #define FLT_DECIMAL_DIG DECIMAL_DIG
  #endif
#endif
#ifndef DBL_DECIMAL_DIG
  #if FLT_RADIX == 2 && DBL_MANT_DIG <= 53 && 17 < DECIMAL_DIG
    #define DBL_DECIMAL_DIG 17
  #else
    #define DBL_DECIMAL_DIG DECIMAL_DIG
  #endif
#endif

static int write_str_f16(FILE *out, const uint16_t *src) {
  float x = halfbits2float(*src);
  if (isnan(x)) {
    return fprintf(out, "f16.nan");
  } else if (isinf(x) && x >= 0) {
    return fprintf(out, "f16.inf");
  } else if (isinf(x)) {
    return fprintf(out, "-f16.inf");
  } else {
    return fprintf(out, "%.*gf16", FLT_DECIMAL_DIG, x);
  }
}

static int write_str_f32(FILE *out, const float *src) {
  float x = *src;
  if (isnan(x)) {
    return fprintf(out, "f32.nan");
  } else if (isinf(x) && x >= 0) {
    return fprintf(out, "f32.inf");
  } else if (isinf(x)) {
    return fprintf(out, "-f32.inf");
  } else {
    return fprintf(out, "%.*gf32", FLT_DECIMAL_DIG, x);
  }
}

static int write_str_f64(FILE *out, const double *src) {
  double x = *src;
  if (isnan(x)) {
    return fprintf(out, "f64.nan");
  } else if (isinf(x) && x >= 0) {
    return fprintf(out, "f64.inf");
  } else if (isinf(x)) {
    return fprintf(out, "-f64.inf");
  } else {
    return fprintf(out, "%.*gf64", DBL_DECIMAL_DIG, x);
  }
}

static int write_str_bool(FILE *out, const void *src) {
  return fprintf(out, *(char*)src ? "true" : "false");
}

static int write_str_unit(FILE *out, const void *src) {
  (void)src;
  return fprintf(out, "()");
}

//// Binary I/O

#define BINARY_FORMAT_VERSION 2
#define IS_BIG_ENDIAN (!*(unsigned char *)&(uint16_t){1})

static void flip_bytes(size_t elem_size, unsigned char *elem) {
  for (size_t j=0; j<elem_size/2; j++) {
    unsigned char head = elem[j];
    size_t tail_index = elem_size-1-j;
    elem[j] = elem[tail_index];
    elem[tail_index] = head;
  }
}

// On Windows we need to explicitly set the file mode to not mangle
// newline characters.  On *nix there is no difference.
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
static void set_binary_mode(FILE *f) {
  setmode(fileno(f), O_BINARY);
}
#else
static void set_binary_mode(FILE *f) {
  (void)f;
}
#endif

static int read_byte(FILE *f, void* dest) {
  size_t num_elems_read = fread(dest, 1, 1, f);
  return num_elems_read == 1 ? 0 : 1;
}

//// Types

struct primtype_info_t {
  const char binname[5]; // Used for parsing binary data.
  const char* type_name; // Same name as in Futhark.
  const int64_t size; // in bytes
  const writer write_str; // Write in text format.
  const str_reader read_str; // Read in text format.
};

static const struct primtype_info_t i8_info =
  {.binname = "  i8", .type_name = "i8",   .size = 1,
   .write_str = (writer)write_str_i8, .read_str = (str_reader)read_str_i8};
static const struct primtype_info_t i16_info =
  {.binname = " i16", .type_name = "i16",  .size = 2,
   .write_str = (writer)write_str_i16, .read_str = (str_reader)read_str_i16};
static const struct primtype_info_t i32_info =
  {.binname = " i32", .type_name = "i32",  .size = 4,
   .write_str = (writer)write_str_i32, .read_str = (str_reader)read_str_i32};
static const struct primtype_info_t i64_info =
  {.binname = " i64", .type_name = "i64",  .size = 8,
   .write_str = (writer)write_str_i64, .read_str = (str_reader)read_str_i64};
static const struct primtype_info_t u8_info =
  {.binname = "  u8", .type_name = "u8",   .size = 1,
   .write_str = (writer)write_str_u8, .read_str = (str_reader)read_str_u8};
static const struct primtype_info_t u16_info =
  {.binname = " u16", .type_name = "u16",  .size = 2,
   .write_str = (writer)write_str_u16, .read_str = (str_reader)read_str_u16};
static const struct primtype_info_t u32_info =
  {.binname = " u32", .type_name = "u32",  .size = 4,
   .write_str = (writer)write_str_u32, .read_str = (str_reader)read_str_u32};
static const struct primtype_info_t u64_info =
  {.binname = " u64", .type_name = "u64",  .size = 8,
   .write_str = (writer)write_str_u64, .read_str = (str_reader)read_str_u64};
static const struct primtype_info_t f16_info =
  {.binname = " f16", .type_name = "f16",  .size = 2,
   .write_str = (writer)write_str_f16, .read_str = (str_reader)read_str_f16};
static const struct primtype_info_t f32_info =
  {.binname = " f32", .type_name = "f32",  .size = 4,
   .write_str = (writer)write_str_f32, .read_str = (str_reader)read_str_f32};
static const struct primtype_info_t f64_info =
  {.binname = " f64", .type_name = "f64",  .size = 8,
   .write_str = (writer)write_str_f64, .read_str = (str_reader)read_str_f64};
static const struct primtype_info_t bool_info =
  {.binname = "bool", .type_name = "bool", .size = 1,
   .write_str = (writer)write_str_bool, .read_str = (str_reader)read_str_bool};
static const struct primtype_info_t unit_info =
  {.binname = "bool", .type_name = "unit",   .size = 1,
   .write_str = (writer)write_str_unit, .read_str = (str_reader)read_str_unit};

static const struct primtype_info_t* primtypes[] = {
  &i8_info, &i16_info, &i32_info, &i64_info,
  &u8_info, &u16_info, &u32_info, &u64_info,
  &f16_info, &f32_info, &f64_info,
  &bool_info,
  NULL // NULL-terminated
};

// General value interface.  All endian business taken care of at
// lower layers.

static int read_is_binary(FILE *f) {
  skipspaces(f);
  int c = getc(f);
  if (c == 'b') {
    int8_t bin_version;
    int ret = read_byte(f, &bin_version);

    if (ret != 0) { futhark_panic(1, "binary-input: could not read version.\n"); }

    if (bin_version != BINARY_FORMAT_VERSION) {
      futhark_panic(1, "binary-input: File uses version %i, but I only understand version %i.\n",
            bin_version, BINARY_FORMAT_VERSION);
    }

    return 1;
  }
  ungetc(c, f);
  return 0;
}

static const struct primtype_info_t* read_bin_read_type_enum(FILE *f) {
  char read_binname[4];

  int num_matched = fscanf(f, "%4c", read_binname);
  if (num_matched != 1) { futhark_panic(1, "binary-input: Couldn't read element type.\n"); }

  const struct primtype_info_t **type = primtypes;

  for (; *type != NULL; type++) {
    // I compare the 4 characters manually instead of using strncmp because
    // this allows any value to be used, also NULL bytes
    if (memcmp(read_binname, (*type)->binname, 4) == 0) {
      return *type;
    }
  }
  futhark_panic(1, "binary-input: Did not recognize the type '%s'.\n", read_binname);
  return NULL;
}

static void read_bin_ensure_scalar(FILE *f, const struct primtype_info_t *expected_type) {
  int8_t bin_dims;
  int ret = read_byte(f, &bin_dims);
  if (ret != 0) { futhark_panic(1, "binary-input: Couldn't get dims.\n"); }

  if (bin_dims != 0) {
    futhark_panic(1, "binary-input: Expected scalar (0 dimensions), but got array with %i dimensions.\n",
          bin_dims);
  }

  const struct primtype_info_t *bin_type = read_bin_read_type_enum(f);
  if (bin_type != expected_type) {
    futhark_panic(1, "binary-input: Expected scalar of type %s but got scalar of type %s.\n",
          expected_type->type_name,
          bin_type->type_name);
  }
}

//// High-level interface

static int read_bin_array(FILE *f,
                          const struct primtype_info_t *expected_type, void **data, int64_t *shape, int64_t dims) {
  int ret;

  int8_t bin_dims;
  ret = read_byte(f, &bin_dims);
  if (ret != 0) { futhark_panic(1, "binary-input: Couldn't get dims.\n"); }

  if (bin_dims != dims) {
    futhark_panic(1, "binary-input: Expected %i dimensions, but got array with %i dimensions.\n",
          dims, bin_dims);
  }

  const struct primtype_info_t *bin_primtype = read_bin_read_type_enum(f);
  if (expected_type != bin_primtype) {
    futhark_panic(1, "binary-input: Expected %iD-array with element type '%s' but got %iD-array with element type '%s'.\n",
          dims, expected_type->type_name, dims, bin_primtype->type_name);
  }

  int64_t elem_count = 1;
  for (int i=0; i<dims; i++) {
    int64_t bin_shape;
    ret = (int)fread(&bin_shape, sizeof(bin_shape), 1, f);
    if (ret != 1) {
      futhark_panic(1, "binary-input: Couldn't read size for dimension %i of array.\n", i);
    }
    if (IS_BIG_ENDIAN) {
      flip_bytes(sizeof(bin_shape), (unsigned char*) &bin_shape);
    }
    elem_count *= bin_shape;
    shape[i] = bin_shape;
  }

  int64_t elem_size = expected_type->size;
  void* tmp = realloc(*data, (size_t)(elem_count * elem_size));
  if (tmp == NULL) {
    futhark_panic(1, "binary-input: Failed to allocate array of size %i.\n",
          elem_count * elem_size);
  }
  *data = tmp;

  int64_t num_elems_read = (int64_t)fread(*data, (size_t)elem_size, (size_t)elem_count, f);
  if (num_elems_read != elem_count) {
    futhark_panic(1, "binary-input: tried to read %i elements of an array, but only got %i elements.\n",
          elem_count, num_elems_read);
  }

  // If we're on big endian platform we must change all multibyte elements
  // from using little endian to big endian
  if (IS_BIG_ENDIAN && elem_size != 1) {
    flip_bytes((size_t)elem_size, (unsigned char*) *data);
  }

  return 0;
}

static int read_array(FILE *f, const struct primtype_info_t *expected_type, void **data, int64_t *shape, int64_t dims) {
  if (!read_is_binary(f)) {
    return read_str_array(f, expected_type->size, (str_reader)expected_type->read_str, expected_type->type_name, data, shape, dims);
  } else {
    return read_bin_array(f, expected_type, data, shape, dims);
  }
}

static int end_of_input(FILE *f) {
  skipspaces(f);
  char token[2];
  next_token(f, token, sizeof(token));
  if (strcmp(token, "") == 0) {
    return 0;
  } else {
    return 1;
  }
}

static int write_str_array(FILE *out,
                           const struct primtype_info_t *elem_type,
                           const unsigned char *data,
                           const int64_t *shape,
                           int8_t rank) {
  if (rank==0) {
    elem_type->write_str(out, (const void*)data);
  } else {
    int64_t len = (int64_t)shape[0];
    int64_t slice_size = 1;

    int64_t elem_size = elem_type->size;
    for (int8_t i = 1; i < rank; i++) {
      slice_size *= shape[i];
    }

    if (len*slice_size == 0) {
      fprintf(out, "empty(");
      for (int64_t i = 0; i < rank; i++) {
        fprintf(out, "[%"PRIi64"]", shape[i]);
      }
      fprintf(out, "%s", elem_type->type_name);
      fprintf(out, ")");
    } else if (rank==1) {
      fputc('[', out);
      for (int64_t i = 0; i < len; i++) {
        elem_type->write_str(out, (const void*) (data + i * elem_size));
        if (i != len-1) {
          fprintf(out, ", ");
        }
      }
      fputc(']', out);
    } else {
      fputc('[', out);
      for (int64_t i = 0; i < len; i++) {
        write_str_array(out, elem_type, data + i * slice_size * elem_size, shape+1, rank-1);
        if (i != len-1) {
          fprintf(out, ", ");
        }
      }
      fputc(']', out);
    }
  }
  return 0;
}

static int write_bin_array(FILE *out,
                           const struct primtype_info_t *elem_type,
                           const unsigned char *data,
                           const int64_t *shape,
                           int8_t rank) {
  int64_t num_elems = 1;
  for (int64_t i = 0; i < rank; i++) {
    num_elems *= shape[i];
  }

  fputc('b', out);
  fputc((char)BINARY_FORMAT_VERSION, out);
  fwrite(&rank, sizeof(int8_t), 1, out);
  fwrite(elem_type->binname, 4, 1, out);
  if (shape != NULL) {
    fwrite(shape, sizeof(int64_t), (size_t)rank, out);
  }

  if (IS_BIG_ENDIAN) {
    for (int64_t i = 0; i < num_elems; i++) {
      const unsigned char *elem = data+i*elem_type->size;
      for (int64_t j = 0; j < elem_type->size; j++) {
        fwrite(&elem[elem_type->size-j], 1, 1, out);
      }
    }
  } else {
    fwrite(data, (size_t)elem_type->size, (size_t)num_elems, out);
  }

  return 0;
}

static int write_array(FILE *out, int write_binary,
                       const struct primtype_info_t *elem_type,
                       const void *data,
                       const int64_t *shape,
                       const int8_t rank) {
  if (write_binary) {
    return write_bin_array(out, elem_type, data, shape, rank);
  } else {
    return write_str_array(out, elem_type, data, shape, rank);
  }
}

static int read_scalar(FILE *f,
                       const struct primtype_info_t *expected_type, void *dest) {
  if (!read_is_binary(f)) {
    char buf[100];
    next_token(f, buf, sizeof(buf));
    return expected_type->read_str(buf, dest);
  } else {
    read_bin_ensure_scalar(f, expected_type);
    size_t elem_size = (size_t)expected_type->size;
    size_t num_elems_read = fread(dest, elem_size, 1, f);
    if (IS_BIG_ENDIAN) {
      flip_bytes(elem_size, (unsigned char*) dest);
    }
    return num_elems_read == 1 ? 0 : 1;
  }
}

static int write_scalar(FILE *out, int write_binary, const struct primtype_info_t *type, void *src) {
  if (write_binary) {
    return write_bin_array(out, type, src, NULL, 0);
  } else {
    return type->write_str(out, src);
  }
}

// End of values.h.

static int binary_output = 0;
static int print_result = 1;
static FILE *runtime_file;
static int perform_warmup = 0;
static int num_runs = 1;
static const char *entry_point = "main";
// Start of tuning.h.


int is_blank_line_or_comment(const char *s) {
  size_t i = strspn(s, " \t\n");
  return s[i] == '\0' || // Line is blank.
         strncmp(s + i, "--", 2) == 0; // Line is comment.
}

static char* load_tuning_file(const char *fname,
                              void *cfg,
                              int (*set_tuning_param)(void*, const char*, size_t)) {
  const int max_line_len = 1024;
  char* line = (char*) malloc(max_line_len);

  FILE *f = fopen(fname, "r");

  if (f == NULL) {
    snprintf(line, max_line_len, "Cannot open file: %s", strerror(errno));
    return line;
  }

  int lineno = 0;
  while (fgets(line, max_line_len, f) != NULL) {
    lineno++;
    if (is_blank_line_or_comment(line)) {
      continue;
    }
    char *eql = strstr(line, "=");
    if (eql) {
      *eql = 0;
      char *endptr;
      int value = strtol(eql+1, &endptr, 10);
      if (*endptr && *endptr != '\n') {
        snprintf(line, max_line_len, "Invalid line %d (must be of form 'name=int').",
                 lineno);
        return line;
      }
      if (set_tuning_param(cfg, line, (size_t)value) != 0) {
        char* err = (char*) malloc(max_line_len + 50);
        snprintf(err, max_line_len + 50, "Unknown name '%s' on line %d.", line, lineno);
        free(line);
        return err;
      }
    } else {
      snprintf(line, max_line_len, "Invalid line %d (must be of form 'name=int').",
               lineno);
      return line;
    }
  }

  free(line);

  return NULL;
}

// End of tuning.h.

int parse_options(struct futhark_context_config *cfg, int argc, char *const argv[])
{
    int ch;
    static struct option long_options[] = {{"write-runtime-to", required_argument, NULL, 1}, {"runs", required_argument, NULL, 2}, {"debugging", no_argument, NULL, 3}, {"log", no_argument, NULL, 4}, {"profile", no_argument, NULL, 5}, {"entry-point", required_argument, NULL, 6}, {"binary-output", no_argument, NULL, 7}, {"no-print-result", no_argument, NULL, 8}, {"help", no_argument, NULL, 9}, {"print-params", no_argument, NULL, 10}, {"param", required_argument, NULL, 11}, {"tuning", required_argument, NULL, 12}, {"cache-file", required_argument, NULL, 13}, {0, 0, 0, 0}};
    static char *option_descriptions = "  -t/--write-runtime-to FILE Print the time taken to execute the program to the indicated file, an integral number of microseconds.\n  -r/--runs INT              Perform NUM runs of the program.\n  -D/--debugging             Perform possibly expensive internal correctness checks and verbose logging.\n  -L/--log                   Print various low-overhead logging information to stderr while running.\n  -P/--profile               Enable the collection of profiling information.\n  -e/--entry-point NAME      The entry point to run. Defaults to main.\n  -b/--binary-output         Print the program result in the binary output format.\n  -n/--no-print-result       Do not print the program result.\n  -h/--help                  Print help information and exit.\n  --print-params             Print all tuning parameters that can be set with --param or --tuning.\n  --param ASSIGNMENT         Set a tuning parameter to the given value.\n  --tuning FILE              Read size=value assignments from the given file.\n  --cache-file FILE          Store program cache here.\n";
    
    while ((ch = getopt_long(argc, argv, ":t:r:DLPe:bnh", long_options, NULL)) != -1) {
        if (ch == 1 || ch == 't') {
            runtime_file = fopen(optarg, "w");
            if (runtime_file == NULL)
                futhark_panic(1, "Cannot open %s: %s\n", optarg, strerror(errno));
        }
        if (ch == 2 || ch == 'r') {
            num_runs = atoi(optarg);
            perform_warmup = 1;
            if (num_runs <= 0)
                futhark_panic(1, "Need a positive number of runs, not %s\n", optarg);
        }
        if (ch == 3 || ch == 'D')
            futhark_context_config_set_debugging(cfg, 1);
        if (ch == 4 || ch == 'L')
            futhark_context_config_set_logging(cfg, 1);
        if (ch == 5 || ch == 'P')
            futhark_context_config_set_profiling(cfg, 1);
        if (ch == 6 || ch == 'e') {
            if (entry_point != NULL)
                entry_point = optarg;
        }
        if (ch == 7 || ch == 'b')
            binary_output = 1;
        if (ch == 8 || ch == 'n')
            print_result = 0;
        if (ch == 9 || ch == 'h') {
            printf("Usage: %s [OPTION]...\nOptions:\n\n%s\nFor more information, consult the Futhark User's Guide or the man pages.\n", fut_progname, option_descriptions);
            exit(0);
        }
        if (ch == 10) {
            int n = futhark_get_tuning_param_count();
            
            for (int i = 0; i < n; i++)
                printf("%s (%s)\n", futhark_get_tuning_param_name(i), futhark_get_tuning_param_class(i));
            exit(0);
        }
        if (ch == 11) {
            char *name = optarg;
            char *equals = strstr(optarg, "=");
            char *value_str = equals != NULL ? equals + 1 : optarg;
            int value = atoi(value_str);
            
            if (equals != NULL) {
                *equals = 0;
                if (futhark_context_config_set_tuning_param(cfg, name, (size_t) value) != 0)
                    futhark_panic(1, "Unknown tuning parameter: %s\n", name);
            } else
                futhark_panic(1, "Invalid argument for --parameter option: %s\n", optarg);
        }
        if (ch == 12) {
            char *ret = load_tuning_file(optarg, cfg, (int (*)(void *, const char *, size_t)) futhark_context_config_set_tuning_param);
            
            if (ret != NULL)
                futhark_panic(1, "When loading tuning file '%s': %s\n", optarg, ret);
        }
        if (ch == 13)
            futhark_context_config_set_cache_file(cfg, optarg);
        if (ch == ':')
            futhark_panic(-1, "Missing argument for option %s\n", argv[optind - 1]);
        if (ch == '?') {
            fprintf(stderr, "Usage: %s [OPTIONS]...\nOptions:\n\n%s\n", fut_progname, "  -t/--write-runtime-to FILE Print the time taken to execute the program to the indicated file, an integral number of microseconds.\n  -r/--runs INT              Perform NUM runs of the program.\n  -D/--debugging             Perform possibly expensive internal correctness checks and verbose logging.\n  -L/--log                   Print various low-overhead logging information to stderr while running.\n  -P/--profile               Enable the collection of profiling information.\n  -e/--entry-point NAME      The entry point to run. Defaults to main.\n  -b/--binary-output         Print the program result in the binary output format.\n  -n/--no-print-result       Do not print the program result.\n  -h/--help                  Print help information and exit.\n  --print-params             Print all tuning parameters that can be set with --param or --tuning.\n  --param ASSIGNMENT         Set a tuning parameter to the given value.\n  --tuning FILE              Read size=value assignments from the given file.\n  --cache-file FILE          Store program cache here.\n");
            futhark_panic(1, "Unknown option: %s\n", argv[optind - 1]);
        }
    }
    return optind;
}
static int futrts_cli_entry_arithmetic(struct futhark_context *ctx)
{
    int64_t t_start, t_end;
    int time_runs = 0, profile_run = 0;
    int retval = 0;
    
    // We do not want to profile all the initialisation.
    futhark_context_pause_profiling(ctx);
    // Declare and read input.
    set_binary_mode(stdin);
    
    bool read_value_0;
    
    if (read_scalar(stdin, &bool_info, &read_value_0) != 0)
        futhark_panic(1, "Error when reading input #%d of type %s (errno: %s).\n", 0, "bool", strerror(errno));
    ;
    
    struct futhark_u32_1d * read_value_1;
    int64_t read_shape_1[1];
    uint32_t *read_arr_1 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u32_info, (void **) &read_arr_1, read_shape_1, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 1, "[]u32", strerror(errno));
    
    bool read_value_2;
    
    if (read_scalar(stdin, &bool_info, &read_value_2) != 0)
        futhark_panic(1, "Error when reading input #%d of type %s (errno: %s).\n", 2, "bool", strerror(errno));
    ;
    
    struct futhark_u32_1d * read_value_3;
    int64_t read_shape_3[1];
    uint32_t *read_arr_3 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u32_info, (void **) &read_arr_3, read_shape_3, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 3, "[]u32", strerror(errno));
    
    int32_t read_value_4;
    
    if (read_scalar(stdin, &u32_info, &read_value_4) != 0)
        futhark_panic(1, "Error when reading input #%d of type %s (errno: %s).\n", 4, "u32", strerror(errno));
    ;
    if (end_of_input(stdin) != 0)
        futhark_panic(1, "Expected EOF on stdin after reading input for \"%s\".\n", "arithmetic");
    
    struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 * result_0;
    
    if (perform_warmup) {
        int r;
        
        ;
        assert((read_value_1 = futhark_new_u32_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        ;
        assert((read_value_3 = futhark_new_u32_1d(ctx, read_arr_3, read_shape_3[0])) != NULL);
        ;
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_arithmetic(ctx, &result_0, read_value_0, read_value_1, read_value_2, read_value_3, read_value_4);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        ;
        assert(futhark_free_u32_1d(ctx, read_value_1) == 0);
        ;
        assert(futhark_free_u32_1d(ctx, read_value_3) == 0);
        ;
        assert(futhark_free_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(ctx, result_0) == 0);
    }
    time_runs = 1;
    // Proper run.
    for (int run = 0; run < num_runs; run++) {
        // Only profile last run.
        profile_run = run == num_runs - 1;
        
        int r;
        
        ;
        assert((read_value_1 = futhark_new_u32_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        ;
        assert((read_value_3 = futhark_new_u32_1d(ctx, read_arr_3, read_shape_3[0])) != NULL);
        ;
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_arithmetic(ctx, &result_0, read_value_0, read_value_1, read_value_2, read_value_3, read_value_4);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        ;
        assert(futhark_free_u32_1d(ctx, read_value_1) == 0);
        ;
        assert(futhark_free_u32_1d(ctx, read_value_3) == 0);
        ;
        if (run < num_runs - 1) {
            assert(futhark_free_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(ctx, result_0) == 0);
        }
    }
    ;
    free(read_arr_1);
    ;
    free(read_arr_3);
    ;
    if (print_result) {
        // Print the final result.
        if (binary_output)
            set_binary_mode(stdout);
        {
            {
                struct futhark_opaque_tup2_bool_arr1d_u32 * field;
                
                if (futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_0(ctx, &field, result_0) != FUTHARK_SUCCESS)
                    futhark_panic(1, "Failed to project field %s from result\n", "0");
                else {
                    struct futhark_opaque_tup2_bool_arr1d_u32 * field_val = field;
                    
                    {
                        {
                            bool field;
                            
                            if (futhark_project_opaque_tup2_bool_arr1d_u32_0(ctx, &field, field_val) != FUTHARK_SUCCESS)
                                futhark_panic(1, "Failed to project field %s from result\n", "0");
                            else {
                                bool field_val = field;
                                
                                write_scalar(stdout, binary_output, &bool_info, &field_val);
                            }
                        }
                        puts("");
                        {
                            struct futhark_u32_1d * field;
                            
                            if (futhark_project_opaque_tup2_bool_arr1d_u32_1(ctx, &field, field_val) != FUTHARK_SUCCESS)
                                futhark_panic(1, "Failed to project field %s from result\n", "1");
                            else {
                                struct futhark_u32_1d * field_val = field;
                                
                                {
                                    uint32_t *arr = calloc(futhark_shape_u32_1d(ctx, field_val)[0], u32_info.size);
                                    
                                    assert(arr != NULL);
                                    assert(futhark_values_u32_1d(ctx, field_val, arr) == 0);
                                    assert(futhark_context_sync(ctx) == 0);
                                    write_array(stdout, binary_output, &u32_info, arr, futhark_shape_u32_1d(ctx, field_val), 1);
                                    free(arr);
                                }
                            }
                        }
                    }
                }
            }
            puts("");
            {
                struct futhark_opaque_tup2_bool_arr1d_u32 * field;
                
                if (futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_1(ctx, &field, result_0) != FUTHARK_SUCCESS)
                    futhark_panic(1, "Failed to project field %s from result\n", "1");
                else {
                    struct futhark_opaque_tup2_bool_arr1d_u32 * field_val = field;
                    
                    {
                        {
                            bool field;
                            
                            if (futhark_project_opaque_tup2_bool_arr1d_u32_0(ctx, &field, field_val) != FUTHARK_SUCCESS)
                                futhark_panic(1, "Failed to project field %s from result\n", "0");
                            else {
                                bool field_val = field;
                                
                                write_scalar(stdout, binary_output, &bool_info, &field_val);
                            }
                        }
                        puts("");
                        {
                            struct futhark_u32_1d * field;
                            
                            if (futhark_project_opaque_tup2_bool_arr1d_u32_1(ctx, &field, field_val) != FUTHARK_SUCCESS)
                                futhark_panic(1, "Failed to project field %s from result\n", "1");
                            else {
                                struct futhark_u32_1d * field_val = field;
                                
                                {
                                    uint32_t *arr = calloc(futhark_shape_u32_1d(ctx, field_val)[0], u32_info.size);
                                    
                                    assert(arr != NULL);
                                    assert(futhark_values_u32_1d(ctx, field_val, arr) == 0);
                                    assert(futhark_context_sync(ctx) == 0);
                                    write_array(stdout, binary_output, &u32_info, arr, futhark_shape_u32_1d(ctx, field_val), 1);
                                    free(arr);
                                }
                            }
                        }
                    }
                }
            }
            puts("");
            {
                struct futhark_opaque_tup2_bool_arr1d_u32 * field;
                
                if (futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_2(ctx, &field, result_0) != FUTHARK_SUCCESS)
                    futhark_panic(1, "Failed to project field %s from result\n", "2");
                else {
                    struct futhark_opaque_tup2_bool_arr1d_u32 * field_val = field;
                    
                    {
                        {
                            bool field;
                            
                            if (futhark_project_opaque_tup2_bool_arr1d_u32_0(ctx, &field, field_val) != FUTHARK_SUCCESS)
                                futhark_panic(1, "Failed to project field %s from result\n", "0");
                            else {
                                bool field_val = field;
                                
                                write_scalar(stdout, binary_output, &bool_info, &field_val);
                            }
                        }
                        puts("");
                        {
                            struct futhark_u32_1d * field;
                            
                            if (futhark_project_opaque_tup2_bool_arr1d_u32_1(ctx, &field, field_val) != FUTHARK_SUCCESS)
                                futhark_panic(1, "Failed to project field %s from result\n", "1");
                            else {
                                struct futhark_u32_1d * field_val = field;
                                
                                {
                                    uint32_t *arr = calloc(futhark_shape_u32_1d(ctx, field_val)[0], u32_info.size);
                                    
                                    assert(arr != NULL);
                                    assert(futhark_values_u32_1d(ctx, field_val, arr) == 0);
                                    assert(futhark_context_sync(ctx) == 0);
                                    write_array(stdout, binary_output, &u32_info, arr, futhark_shape_u32_1d(ctx, field_val), 1);
                                    free(arr);
                                }
                            }
                        }
                    }
                }
            }
            puts("");
            {
                struct futhark_opaque_tup2_bool_arr1d_u32 * field;
                
                if (futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_3(ctx, &field, result_0) != FUTHARK_SUCCESS)
                    futhark_panic(1, "Failed to project field %s from result\n", "3");
                else {
                    struct futhark_opaque_tup2_bool_arr1d_u32 * field_val = field;
                    
                    {
                        {
                            bool field;
                            
                            if (futhark_project_opaque_tup2_bool_arr1d_u32_0(ctx, &field, field_val) != FUTHARK_SUCCESS)
                                futhark_panic(1, "Failed to project field %s from result\n", "0");
                            else {
                                bool field_val = field;
                                
                                write_scalar(stdout, binary_output, &bool_info, &field_val);
                            }
                        }
                        puts("");
                        {
                            struct futhark_u32_1d * field;
                            
                            if (futhark_project_opaque_tup2_bool_arr1d_u32_1(ctx, &field, field_val) != FUTHARK_SUCCESS)
                                futhark_panic(1, "Failed to project field %s from result\n", "1");
                            else {
                                struct futhark_u32_1d * field_val = field;
                                
                                {
                                    uint32_t *arr = calloc(futhark_shape_u32_1d(ctx, field_val)[0], u32_info.size);
                                    
                                    assert(arr != NULL);
                                    assert(futhark_values_u32_1d(ctx, field_val, arr) == 0);
                                    assert(futhark_context_sync(ctx) == 0);
                                    write_array(stdout, binary_output, &u32_info, arr, futhark_shape_u32_1d(ctx, field_val), 1);
                                    free(arr);
                                }
                            }
                        }
                    }
                }
            }
            puts("");
            {
                int64_t field;
                
                if (futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_4(ctx, &field, result_0) != FUTHARK_SUCCESS)
                    futhark_panic(1, "Failed to project field %s from result\n", "4");
                else {
                    int64_t field_val = field;
                    
                    write_scalar(stdout, binary_output, &i64_info, &field_val);
                }
            }
        }
        puts("");
    }
    
  print_end:
    { }
    assert(futhark_free_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(ctx, result_0) == 0);
    return retval;
}
static int futrts_cli_entry_hashmap_increment(struct futhark_context *ctx)
{
    int64_t t_start, t_end;
    int time_runs = 0, profile_run = 0;
    int retval = 0;
    
    // We do not want to profile all the initialisation.
    futhark_context_pause_profiling(ctx);
    // Declare and read input.
    set_binary_mode(stdin);
    
    struct futhark_u64_1d * read_value_0;
    int64_t read_shape_0[1];
    uint64_t *read_arr_0 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u64_info, (void **) &read_arr_0, read_shape_0, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 0, "[]u64", strerror(errno));
    
    struct futhark_i64_1d * read_value_1;
    int64_t read_shape_1[1];
    int64_t *read_arr_1 = NULL;
    
    errno = 0;
    if (read_array(stdin, &i64_info, (void **) &read_arr_1, read_shape_1, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 1, "[]i64", strerror(errno));
    if (end_of_input(stdin) != 0)
        futhark_panic(1, "Expected EOF on stdin after reading input for \"%s\".\n", "hashmap_increment");
    
    struct futhark_i64_1d * result_0;
    
    if (perform_warmup) {
        int r;
        
        assert((read_value_0 = futhark_new_u64_1d(ctx, read_arr_0, read_shape_0[0])) != NULL);
        assert((read_value_1 = futhark_new_i64_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_hashmap_increment(ctx, &result_0, read_value_0, read_value_1);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        assert(futhark_free_u64_1d(ctx, read_value_0) == 0);
        assert(futhark_free_i64_1d(ctx, read_value_1) == 0);
        assert(futhark_free_i64_1d(ctx, result_0) == 0);
    }
    time_runs = 1;
    // Proper run.
    for (int run = 0; run < num_runs; run++) {
        // Only profile last run.
        profile_run = run == num_runs - 1;
        
        int r;
        
        assert((read_value_0 = futhark_new_u64_1d(ctx, read_arr_0, read_shape_0[0])) != NULL);
        assert((read_value_1 = futhark_new_i64_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_hashmap_increment(ctx, &result_0, read_value_0, read_value_1);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        assert(futhark_free_u64_1d(ctx, read_value_0) == 0);
        assert(futhark_free_i64_1d(ctx, read_value_1) == 0);
        if (run < num_runs - 1) {
            assert(futhark_free_i64_1d(ctx, result_0) == 0);
        }
    }
    free(read_arr_0);
    free(read_arr_1);
    if (print_result) {
        // Print the final result.
        if (binary_output)
            set_binary_mode(stdout);
        {
            int64_t *arr = calloc(futhark_shape_i64_1d(ctx, result_0)[0], i64_info.size);
            
            assert(arr != NULL);
            assert(futhark_values_i64_1d(ctx, result_0, arr) == 0);
            assert(futhark_context_sync(ctx) == 0);
            write_array(stdout, binary_output, &i64_info, arr, futhark_shape_i64_1d(ctx, result_0), 1);
            free(arr);
        }
        puts("");
    }
    
  print_end:
    { }
    assert(futhark_free_i64_1d(ctx, result_0) == 0);
    return retval;
}
static int futrts_cli_entry_hashmap_reserved(struct futhark_context *ctx)
{
    int64_t t_start, t_end;
    int time_runs = 0, profile_run = 0;
    int retval = 0;
    
    // We do not want to profile all the initialisation.
    futhark_context_pause_profiling(ctx);
    // Declare and read input.
    set_binary_mode(stdin);
    
    struct futhark_u64_1d * read_value_0;
    int64_t read_shape_0[1];
    uint64_t *read_arr_0 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u64_info, (void **) &read_arr_0, read_shape_0, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 0, "[]u64", strerror(errno));
    
    struct futhark_i64_1d * read_value_1;
    int64_t read_shape_1[1];
    int64_t *read_arr_1 = NULL;
    
    errno = 0;
    if (read_array(stdin, &i64_info, (void **) &read_arr_1, read_shape_1, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 1, "[]i64", strerror(errno));
    
    int64_t read_value_2;
    
    if (read_scalar(stdin, &i64_info, &read_value_2) != 0)
        futhark_panic(1, "Error when reading input #%d of type %s (errno: %s).\n", 2, "i64", strerror(errno));
    ;
    if (end_of_input(stdin) != 0)
        futhark_panic(1, "Expected EOF on stdin after reading input for \"%s\".\n", "hashmap_reserved");
    
    struct futhark_i64_1d * result_0;
    
    if (perform_warmup) {
        int r;
        
        assert((read_value_0 = futhark_new_u64_1d(ctx, read_arr_0, read_shape_0[0])) != NULL);
        assert((read_value_1 = futhark_new_i64_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        ;
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_hashmap_reserved(ctx, &result_0, read_value_0, read_value_1, read_value_2);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        assert(futhark_free_u64_1d(ctx, read_value_0) == 0);
        assert(futhark_free_i64_1d(ctx, read_value_1) == 0);
        ;
        assert(futhark_free_i64_1d(ctx, result_0) == 0);
    }
    time_runs = 1;
    // Proper run.
    for (int run = 0; run < num_runs; run++) {
        // Only profile last run.
        profile_run = run == num_runs - 1;
        
        int r;
        
        assert((read_value_0 = futhark_new_u64_1d(ctx, read_arr_0, read_shape_0[0])) != NULL);
        assert((read_value_1 = futhark_new_i64_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        ;
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_hashmap_reserved(ctx, &result_0, read_value_0, read_value_1, read_value_2);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        assert(futhark_free_u64_1d(ctx, read_value_0) == 0);
        assert(futhark_free_i64_1d(ctx, read_value_1) == 0);
        ;
        if (run < num_runs - 1) {
            assert(futhark_free_i64_1d(ctx, result_0) == 0);
        }
    }
    free(read_arr_0);
    free(read_arr_1);
    ;
    if (print_result) {
        // Print the final result.
        if (binary_output)
            set_binary_mode(stdout);
        {
            int64_t *arr = calloc(futhark_shape_i64_1d(ctx, result_0)[0], i64_info.size);
            
            assert(arr != NULL);
            assert(futhark_values_i64_1d(ctx, result_0, arr) == 0);
            assert(futhark_context_sync(ctx) == 0);
            write_array(stdout, binary_output, &i64_info, arr, futhark_shape_i64_1d(ctx, result_0), 1);
            free(arr);
        }
        puts("");
    }
    
  print_end:
    { }
    assert(futhark_free_i64_1d(ctx, result_0) == 0);
    return retval;
}
static int futrts_cli_entry_hashmap_test(struct futhark_context *ctx)
{
    int64_t t_start, t_end;
    int time_runs = 0, profile_run = 0;
    int retval = 0;
    
    // We do not want to profile all the initialisation.
    futhark_context_pause_profiling(ctx);
    // Declare and read input.
    set_binary_mode(stdin);
    
    struct futhark_u64_1d * read_value_0;
    int64_t read_shape_0[1];
    uint64_t *read_arr_0 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u64_info, (void **) &read_arr_0, read_shape_0, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 0, "[]u64", strerror(errno));
    if (end_of_input(stdin) != 0)
        futhark_panic(1, "Expected EOF on stdin after reading input for \"%s\".\n", "hashmap_test");
    
    struct futhark_i64_1d * result_0;
    
    if (perform_warmup) {
        int r;
        
        assert((read_value_0 = futhark_new_u64_1d(ctx, read_arr_0, read_shape_0[0])) != NULL);
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_hashmap_test(ctx, &result_0, read_value_0);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        assert(futhark_free_u64_1d(ctx, read_value_0) == 0);
        assert(futhark_free_i64_1d(ctx, result_0) == 0);
    }
    time_runs = 1;
    // Proper run.
    for (int run = 0; run < num_runs; run++) {
        // Only profile last run.
        profile_run = run == num_runs - 1;
        
        int r;
        
        assert((read_value_0 = futhark_new_u64_1d(ctx, read_arr_0, read_shape_0[0])) != NULL);
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_hashmap_test(ctx, &result_0, read_value_0);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        assert(futhark_free_u64_1d(ctx, read_value_0) == 0);
        if (run < num_runs - 1) {
            assert(futhark_free_i64_1d(ctx, result_0) == 0);
        }
    }
    free(read_arr_0);
    if (print_result) {
        // Print the final result.
        if (binary_output)
            set_binary_mode(stdout);
        {
            int64_t *arr = calloc(futhark_shape_i64_1d(ctx, result_0)[0], i64_info.size);
            
            assert(arr != NULL);
            assert(futhark_values_i64_1d(ctx, result_0, arr) == 0);
            assert(futhark_context_sync(ctx) == 0);
            write_array(stdout, binary_output, &i64_info, arr, futhark_shape_i64_1d(ctx, result_0), 1);
            free(arr);
        }
        puts("");
    }
    
  print_end:
    { }
    assert(futhark_free_i64_1d(ctx, result_0) == 0);
    return retval;
}
static int futrts_cli_entry_regex_test(struct futhark_context *ctx)
{
    int64_t t_start, t_end;
    int time_runs = 0, profile_run = 0;
    int retval = 0;
    
    // We do not want to profile all the initialisation.
    futhark_context_pause_profiling(ctx);
    // Declare and read input.
    set_binary_mode(stdin);
    
    struct futhark_u8_1d * read_value_0;
    int64_t read_shape_0[1];
    uint8_t *read_arr_0 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u8_info, (void **) &read_arr_0, read_shape_0, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 0, "[]u8", strerror(errno));
    
    struct futhark_u8_1d * read_value_1;
    int64_t read_shape_1[1];
    uint8_t *read_arr_1 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u8_info, (void **) &read_arr_1, read_shape_1, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 1, "[]u8", strerror(errno));
    
    struct futhark_u8_1d * read_value_2;
    int64_t read_shape_2[1];
    uint8_t *read_arr_2 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u8_info, (void **) &read_arr_2, read_shape_2, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 2, "[]u8", strerror(errno));
    if (end_of_input(stdin) != 0)
        futhark_panic(1, "Expected EOF on stdin after reading input for \"%s\".\n", "regex_test");
    
    struct futhark_opaque_tup2_i64_arr1d_u8 * result_0;
    
    if (perform_warmup) {
        int r;
        
        assert((read_value_0 = futhark_new_u8_1d(ctx, read_arr_0, read_shape_0[0])) != NULL);
        assert((read_value_1 = futhark_new_u8_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        assert((read_value_2 = futhark_new_u8_1d(ctx, read_arr_2, read_shape_2[0])) != NULL);
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_regex_test(ctx, &result_0, read_value_0, read_value_1, read_value_2);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        assert(futhark_free_u8_1d(ctx, read_value_0) == 0);
        assert(futhark_free_u8_1d(ctx, read_value_1) == 0);
        assert(futhark_free_u8_1d(ctx, read_value_2) == 0);
        assert(futhark_free_opaque_tup2_i64_arr1d_u8(ctx, result_0) == 0);
    }
    time_runs = 1;
    // Proper run.
    for (int run = 0; run < num_runs; run++) {
        // Only profile last run.
        profile_run = run == num_runs - 1;
        
        int r;
        
        assert((read_value_0 = futhark_new_u8_1d(ctx, read_arr_0, read_shape_0[0])) != NULL);
        assert((read_value_1 = futhark_new_u8_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        assert((read_value_2 = futhark_new_u8_1d(ctx, read_arr_2, read_shape_2[0])) != NULL);
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_regex_test(ctx, &result_0, read_value_0, read_value_1, read_value_2);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        assert(futhark_free_u8_1d(ctx, read_value_0) == 0);
        assert(futhark_free_u8_1d(ctx, read_value_1) == 0);
        assert(futhark_free_u8_1d(ctx, read_value_2) == 0);
        if (run < num_runs - 1) {
            assert(futhark_free_opaque_tup2_i64_arr1d_u8(ctx, result_0) == 0);
        }
    }
    free(read_arr_0);
    free(read_arr_1);
    free(read_arr_2);
    if (print_result) {
        // Print the final result.
        if (binary_output)
            set_binary_mode(stdout);
        {
            {
                int64_t field;
                
                if (futhark_project_opaque_tup2_i64_arr1d_u8_0(ctx, &field, result_0) != FUTHARK_SUCCESS)
                    futhark_panic(1, "Failed to project field %s from result\n", "0");
                else {
                    int64_t field_val = field;
                    
                    write_scalar(stdout, binary_output, &i64_info, &field_val);
                }
            }
            puts("");
            {
                struct futhark_u8_1d * field;
                
                if (futhark_project_opaque_tup2_i64_arr1d_u8_1(ctx, &field, result_0) != FUTHARK_SUCCESS)
                    futhark_panic(1, "Failed to project field %s from result\n", "1");
                else {
                    struct futhark_u8_1d * field_val = field;
                    
                    {
                        uint8_t *arr = calloc(futhark_shape_u8_1d(ctx, field_val)[0], u8_info.size);
                        
                        assert(arr != NULL);
                        assert(futhark_values_u8_1d(ctx, field_val, arr) == 0);
                        assert(futhark_context_sync(ctx) == 0);
                        write_array(stdout, binary_output, &u8_info, arr, futhark_shape_u8_1d(ctx, field_val), 1);
                        free(arr);
                    }
                }
            }
        }
        puts("");
    }
    
  print_end:
    { }
    assert(futhark_free_opaque_tup2_i64_arr1d_u8(ctx, result_0) == 0);
    return retval;
}
static int futrts_cli_entry_wrap_test(struct futhark_context *ctx)
{
    int64_t t_start, t_end;
    int time_runs = 0, profile_run = 0;
    int retval = 0;
    
    // We do not want to profile all the initialisation.
    futhark_context_pause_profiling(ctx);
    // Declare and read input.
    set_binary_mode(stdin);
    
    int64_t read_value_0;
    
    if (read_scalar(stdin, &i64_info, &read_value_0) != 0)
        futhark_panic(1, "Error when reading input #%d of type %s (errno: %s).\n", 0, "i64", strerror(errno));
    ;
    
    struct futhark_u8_1d * read_value_1;
    int64_t read_shape_1[1];
    uint8_t *read_arr_1 = NULL;
    
    errno = 0;
    if (read_array(stdin, &u8_info, (void **) &read_arr_1, read_shape_1, 1) != 0)
        futhark_panic(1, "Cannot read input #%d of type %s (errno: %s).\n", 1, "[]u8", strerror(errno));
    if (end_of_input(stdin) != 0)
        futhark_panic(1, "Expected EOF on stdin after reading input for \"%s\".\n", "wrap_test");
    
    struct futhark_u8_1d * result_0;
    
    if (perform_warmup) {
        int r;
        
        ;
        assert((read_value_1 = futhark_new_u8_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_wrap_test(ctx, &result_0, read_value_0, read_value_1);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        ;
        assert(futhark_free_u8_1d(ctx, read_value_1) == 0);
        assert(futhark_free_u8_1d(ctx, result_0) == 0);
    }
    time_runs = 1;
    // Proper run.
    for (int run = 0; run < num_runs; run++) {
        // Only profile last run.
        profile_run = run == num_runs - 1;
        
        int r;
        
        ;
        assert((read_value_1 = futhark_new_u8_1d(ctx, read_arr_1, read_shape_1[0])) != NULL);
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        // Only profile last run.
        if (profile_run)
            futhark_context_unpause_profiling(ctx);
        t_start = get_wall_time();
        r = futhark_entry_wrap_test(ctx, &result_0, read_value_0, read_value_1);
        if (r != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        if (futhark_context_sync(ctx) != 0)
            futhark_panic(1, "%s", futhark_context_get_error(ctx));
        ;
        if (profile_run)
            futhark_context_pause_profiling(ctx);
        t_end = get_wall_time();
        
        long elapsed_usec = t_end - t_start;
        
        if (time_runs && runtime_file != NULL) {
            fprintf(runtime_file, "%lld\n", (long long) elapsed_usec);
            fflush(runtime_file);
        }
        ;
        assert(futhark_free_u8_1d(ctx, read_value_1) == 0);
        if (run < num_runs - 1) {
            assert(futhark_free_u8_1d(ctx, result_0) == 0);
        }
    }
    ;
    free(read_arr_1);
    if (print_result) {
        // Print the final result.
        if (binary_output)
            set_binary_mode(stdout);
        {
            uint8_t *arr = calloc(futhark_shape_u8_1d(ctx, result_0)[0], u8_info.size);
            
            assert(arr != NULL);
            assert(futhark_values_u8_1d(ctx, result_0, arr) == 0);
            assert(futhark_context_sync(ctx) == 0);
            write_array(stdout, binary_output, &u8_info, arr, futhark_shape_u8_1d(ctx, result_0), 1);
            free(arr);
        }
        puts("");
    }
    
  print_end:
    { }
    assert(futhark_free_u8_1d(ctx, result_0) == 0);
    return retval;
}
typedef int entry_point_fun(struct futhark_context *);
struct entry_point_entry {
    const char *name;
    entry_point_fun *fun;
};
int main(int argc, char **argv)
{
    int retval = 0;
    
    fut_progname = argv[0];
    
    struct futhark_context_config *cfg = futhark_context_config_new();
    
    assert(cfg != NULL);
    
    int parsed_options = parse_options(cfg, argc, argv);
    
    argc -= parsed_options;
    argv += parsed_options;
    if (argc != 0)
        futhark_panic(1, "Excess non-option: %s\n", argv[0]);
    
    struct futhark_context *ctx = futhark_context_new(cfg);
    
    assert(ctx != NULL);
    
    char *error = futhark_context_get_error(ctx);
    
    if (error != NULL)
        futhark_panic(1, "%s", error);
    
    struct entry_point_entry entry_points[] = {{.name ="arithmetic", .fun =futrts_cli_entry_arithmetic}, {.name ="hashmap_increment", .fun =futrts_cli_entry_hashmap_increment}, {.name ="hashmap_reserved", .fun =futrts_cli_entry_hashmap_reserved}, {.name ="hashmap_test", .fun =futrts_cli_entry_hashmap_test}, {.name ="regex_test", .fun =futrts_cli_entry_regex_test}, {.name ="wrap_test", .fun =futrts_cli_entry_wrap_test}};
    
    if (entry_point != NULL) {
        int num_entry_points = sizeof(entry_points) / sizeof(entry_points[0]);
        entry_point_fun *entry_point_fun = NULL;
        
        for (int i = 0; i < num_entry_points; i++) {
            if (strcmp(entry_points[i].name, entry_point) == 0) {
                entry_point_fun = entry_points[i].fun;
                break;
            }
        }
        if (entry_point_fun == NULL) {
            fprintf(stderr, "No entry point '%s'.  Select another with --entry-point.  Options are:\n", entry_point);
            for (int i = 0; i < num_entry_points; i++)
                fprintf(stderr, "%s\n", entry_points[i].name);
            return 1;
        }
        if (isatty(fileno(stdin))) {
            fprintf(stderr, "Reading input from TTY.\n");
            fprintf(stderr, "Send EOF (CTRL-d) after typing all input values.\n");
        }
        retval = entry_point_fun(ctx);
        if (runtime_file != NULL)
            fclose(runtime_file);
    }
    futhark_context_free(ctx);
    futhark_context_config_free(cfg);
    return retval;
}

#ifdef _MSC_VER
#define inline __inline
#endif
#include <string.h>
#include <string.h>
#include <errno.h>
#include <assert.h>
#include <ctype.h>



#define FUTHARK_F64_ENABLED

// Start of scalar.h.

// Implementation of the primitive scalar operations.  Very
// repetitive.  This code is inserted directly into both CUDA and
// OpenCL programs, as well as the CPU code, so it has some #ifdefs to
// work everywhere.  Some operations are defined as macros because
// this allows us to use them as constant expressions in things like
// array sizes and static initialisers.

// Some of the #ifdefs are because OpenCL uses type-generic functions
// for some operations (e.g. sqrt), while C and CUDA sensibly use
// distinct functions for different precisions (e.g. sqrtf() and
// sqrt()).  This is quite annoying.  Due to C's unfortunate casting
// rules, it is also really easy to accidentally implement
// floating-point functions in the wrong precision, so be careful.

// Double-precision definitions are only included if the preprocessor
// macro FUTHARK_F64_ENABLED is set.

#ifndef M_PI
#define M_PI 3.141592653589793
#endif

SCALAR_FUN_ATTR int32_t fptobits_f32_i32(float x);
SCALAR_FUN_ATTR float bitstofp_i32_f32(int32_t x);

SCALAR_FUN_ATTR uint8_t   add8(uint8_t x, uint8_t y)   { return x + y; }
SCALAR_FUN_ATTR uint16_t add16(uint16_t x, uint16_t y) { return x + y; }
SCALAR_FUN_ATTR uint32_t add32(uint32_t x, uint32_t y) { return x + y; }
SCALAR_FUN_ATTR uint64_t add64(uint64_t x, uint64_t y) { return x + y; }

SCALAR_FUN_ATTR uint8_t   sub8(uint8_t x, uint8_t y)   { return x - y; }
SCALAR_FUN_ATTR uint16_t sub16(uint16_t x, uint16_t y) { return x - y; }
SCALAR_FUN_ATTR uint32_t sub32(uint32_t x, uint32_t y) { return x - y; }
SCALAR_FUN_ATTR uint64_t sub64(uint64_t x, uint64_t y) { return x - y; }

SCALAR_FUN_ATTR uint8_t   mul8(uint8_t x, uint8_t y)   { return x * y; }
SCALAR_FUN_ATTR uint16_t mul16(uint16_t x, uint16_t y) { return x * y; }
SCALAR_FUN_ATTR uint32_t mul32(uint32_t x, uint32_t y) { return x * y; }
SCALAR_FUN_ATTR uint64_t mul64(uint64_t x, uint64_t y) { return x * y; }

#if defined(ISPC)

SCALAR_FUN_ATTR uint8_t udiv8(uint8_t x, uint8_t y) {
  // This strange pattern is used to prevent the ISPC compiler from
  // causing SIGFPEs and bogus results on divisions where inactive lanes
  // have 0-valued divisors. It ensures that any inactive lane instead
  // has a divisor of 1. https://github.com/ispc/ispc/issues/2292
  uint8_t ys = 1;
  foreach_active(i) { ys = y; }
  return x / ys;
}

SCALAR_FUN_ATTR uint16_t udiv16(uint16_t x, uint16_t y) {
  uint16_t ys = 1;
  foreach_active(i) { ys = y; }
  return x / ys;
}

SCALAR_FUN_ATTR uint32_t udiv32(uint32_t x, uint32_t y) {
  uint32_t ys = 1;
  foreach_active(i) { ys = y; }
  return x / ys;
}

SCALAR_FUN_ATTR uint64_t udiv64(uint64_t x, uint64_t y) {
  uint64_t ys = 1;
  foreach_active(i) { ys = y; }
  return x / ys;
}

SCALAR_FUN_ATTR uint8_t udiv_up8(uint8_t x, uint8_t y) {
  uint8_t ys = 1;
  foreach_active(i) { ys = y; }
  return (x + y - 1) / ys;
}

SCALAR_FUN_ATTR uint16_t udiv_up16(uint16_t x, uint16_t y) {
  uint16_t ys = 1;
  foreach_active(i) { ys = y; }
  return (x + y - 1) / ys;
}

SCALAR_FUN_ATTR uint32_t udiv_up32(uint32_t x, uint32_t y) {
  uint32_t ys = 1;
  foreach_active(i) { ys = y; }
  return (x + y - 1) / ys;
}

SCALAR_FUN_ATTR uint64_t udiv_up64(uint64_t x, uint64_t y) {
  uint64_t ys = 1;
  foreach_active(i) { ys = y; }
  return (x + y - 1) / ys;
}

SCALAR_FUN_ATTR uint8_t umod8(uint8_t x, uint8_t y) {
  uint8_t ys = 1;
  foreach_active(i) { ys = y; }
  return x % ys;
}

SCALAR_FUN_ATTR uint16_t umod16(uint16_t x, uint16_t y) {
  uint16_t ys = 1;
  foreach_active(i) { ys = y; }
  return x % ys;
}

SCALAR_FUN_ATTR uint32_t umod32(uint32_t x, uint32_t y) {
  uint32_t ys = 1;
  foreach_active(i) { ys = y; }
  return x % ys;
}

SCALAR_FUN_ATTR uint64_t umod64(uint64_t x, uint64_t y) {
  uint64_t ys = 1;
  foreach_active(i) { ys = y; }
  return x % ys;
}

SCALAR_FUN_ATTR uint8_t udiv_safe8(uint8_t x, uint8_t y) {
  uint8_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x / ys;
}

SCALAR_FUN_ATTR uint16_t udiv_safe16(uint16_t x, uint16_t y) {
  uint16_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x / ys;
}

SCALAR_FUN_ATTR uint32_t udiv_safe32(uint32_t x, uint32_t y) {
  uint32_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x / ys;
}

SCALAR_FUN_ATTR uint64_t udiv_safe64(uint64_t x, uint64_t y) {
  uint64_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x / ys;
}

SCALAR_FUN_ATTR uint8_t udiv_up_safe8(uint8_t x, uint8_t y) {
  uint8_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : (x + y - 1) / ys;
}

SCALAR_FUN_ATTR uint16_t udiv_up_safe16(uint16_t x, uint16_t y) {
  uint16_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : (x + y - 1) / ys;
}

SCALAR_FUN_ATTR uint32_t udiv_up_safe32(uint32_t x, uint32_t y) {
  uint32_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : (x + y - 1) / ys;
}

SCALAR_FUN_ATTR uint64_t udiv_up_safe64(uint64_t x, uint64_t y) {
  uint64_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : (x + y - 1) / ys;
}

SCALAR_FUN_ATTR uint8_t umod_safe8(uint8_t x, uint8_t y) {
  uint8_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x % ys;
}

SCALAR_FUN_ATTR uint16_t umod_safe16(uint16_t x, uint16_t y) {
  uint16_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x % ys;
}

SCALAR_FUN_ATTR uint32_t umod_safe32(uint32_t x, uint32_t y) {
  uint32_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x % ys;
}

SCALAR_FUN_ATTR uint64_t umod_safe64(uint64_t x, uint64_t y) {
  uint64_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x % ys;
}

SCALAR_FUN_ATTR int8_t sdiv8(int8_t x, int8_t y) {
  int8_t ys = 1;
  foreach_active(i) { ys = y; }
  int8_t q = x / ys;
  int8_t r = x % ys;
  return q - ((r != 0 && r < 0 != y < 0) ? 1 : 0);
}

SCALAR_FUN_ATTR int16_t sdiv16(int16_t x, int16_t y) {
  int16_t ys = 1;
  foreach_active(i) { ys = y; }
  int16_t q = x / ys;
  int16_t r = x % ys;
  return q - ((r != 0 && r < 0 != y < 0) ? 1 : 0);
}

SCALAR_FUN_ATTR int32_t sdiv32(int32_t x, int32_t y) {
  int32_t ys = 1;
  foreach_active(i) { ys = y; }
  int32_t q = x / ys;
  int32_t r = x % ys;
  return q - ((r != 0 && r < 0 != y < 0) ? 1 : 0);
}

SCALAR_FUN_ATTR int64_t sdiv64(int64_t x, int64_t y) {
  int64_t ys = 1;
  foreach_active(i) { ys = y; }
  int64_t q = x / ys;
  int64_t r = x % ys;
  return q - ((r != 0 && r < 0 != y < 0) ? 1 : 0);
}

SCALAR_FUN_ATTR int8_t sdiv_up8(int8_t x, int8_t y) { return sdiv8(x + y - 1, y); }
SCALAR_FUN_ATTR int16_t sdiv_up16(int16_t x, int16_t y) { return sdiv16(x + y - 1, y); }
SCALAR_FUN_ATTR int32_t sdiv_up32(int32_t x, int32_t y) { return sdiv32(x + y - 1, y); }
SCALAR_FUN_ATTR int64_t sdiv_up64(int64_t x, int64_t y) { return sdiv64(x + y - 1, y); }

SCALAR_FUN_ATTR int8_t smod8(int8_t x, int8_t y) {
  int8_t ys = 1;
  foreach_active(i) { ys = y; }
  int8_t r = x % ys;
  return r + (r == 0 || (x > 0 && y > 0) || (x < 0 && y < 0) ? 0 : y);
}

SCALAR_FUN_ATTR int16_t smod16(int16_t x, int16_t y) {
  int16_t ys = 1;
  foreach_active(i) { ys = y; }
  int16_t r = x % ys;
  return r + (r == 0 || (x > 0 && y > 0) || (x < 0 && y < 0) ? 0 : y);
}

SCALAR_FUN_ATTR int32_t smod32(int32_t x, int32_t y) {
  int32_t ys = 1;
  foreach_active(i) { ys = y; }
  int32_t r = x % ys;
  return r + (r == 0 || (x > 0 && y > 0) || (x < 0 && y < 0) ? 0 : y);
}

SCALAR_FUN_ATTR int64_t smod64(int64_t x, int64_t y) {
  int64_t ys = 1;
  foreach_active(i) { ys = y; }
  int64_t r = x % ys;
  return r + (r == 0 || (x > 0 && y > 0) || (x < 0 && y < 0) ? 0 : y);
}

SCALAR_FUN_ATTR int8_t   sdiv_safe8(int8_t x, int8_t y)   { return y == 0 ? 0 : sdiv8(x, y); }
SCALAR_FUN_ATTR int16_t sdiv_safe16(int16_t x, int16_t y) { return y == 0 ? 0 : sdiv16(x, y); }
SCALAR_FUN_ATTR int32_t sdiv_safe32(int32_t x, int32_t y) { return y == 0 ? 0 : sdiv32(x, y); }
SCALAR_FUN_ATTR int64_t sdiv_safe64(int64_t x, int64_t y) { return y == 0 ? 0 : sdiv64(x, y); }

SCALAR_FUN_ATTR int8_t sdiv_up_safe8(int8_t x, int8_t y)     { return sdiv_safe8(x + y - 1, y); }
SCALAR_FUN_ATTR int16_t sdiv_up_safe16(int16_t x, int16_t y) { return sdiv_safe16(x + y - 1, y); }
SCALAR_FUN_ATTR int32_t sdiv_up_safe32(int32_t x, int32_t y) { return sdiv_safe32(x + y - 1, y); }
SCALAR_FUN_ATTR int64_t sdiv_up_safe64(int64_t x, int64_t y) { return sdiv_safe64(x + y - 1, y); }

SCALAR_FUN_ATTR int8_t   smod_safe8(int8_t x, int8_t y)   { return y == 0 ? 0 : smod8(x, y); }
SCALAR_FUN_ATTR int16_t smod_safe16(int16_t x, int16_t y) { return y == 0 ? 0 : smod16(x, y); }
SCALAR_FUN_ATTR int32_t smod_safe32(int32_t x, int32_t y) { return y == 0 ? 0 : smod32(x, y); }
SCALAR_FUN_ATTR int64_t smod_safe64(int64_t x, int64_t y) { return y == 0 ? 0 : smod64(x, y); }

SCALAR_FUN_ATTR int8_t squot8(int8_t x, int8_t y) {
  int8_t ys = 1;
  foreach_active(i) { ys = y; }
  return x / ys;
}

SCALAR_FUN_ATTR int16_t squot16(int16_t x, int16_t y) {
  int16_t ys = 1;
  foreach_active(i) { ys = y; }
  return x / ys;
}

SCALAR_FUN_ATTR int32_t squot32(int32_t x, int32_t y) {
  int32_t ys = 1;
  foreach_active(i) { ys = y; }
  return x / ys;
}

SCALAR_FUN_ATTR int64_t squot64(int64_t x, int64_t y) {
  int64_t ys = 1;
  foreach_active(i) { ys = y; }
  return x / ys;
}

SCALAR_FUN_ATTR int8_t srem8(int8_t x, int8_t y) {
  int8_t ys = 1;
  foreach_active(i) { ys = y; }
  return x % ys;
}

SCALAR_FUN_ATTR int16_t srem16(int16_t x, int16_t y) {
  int16_t ys = 1;
  foreach_active(i) { ys = y; }
  return x % ys;
}

SCALAR_FUN_ATTR int32_t srem32(int32_t x, int32_t y) {
  int32_t ys = 1;
  foreach_active(i) { ys = y; }
  return x % ys;
}

SCALAR_FUN_ATTR int64_t srem64(int64_t x, int64_t y) {
  int8_t ys = 1;
  foreach_active(i) { ys = y; }
  return x % ys;
}

SCALAR_FUN_ATTR int8_t squot_safe8(int8_t x, int8_t y) {
  int8_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x / ys;
}

SCALAR_FUN_ATTR int16_t squot_safe16(int16_t x, int16_t y) {
  int16_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x / ys;
}

SCALAR_FUN_ATTR int32_t squot_safe32(int32_t x, int32_t y) {
  int32_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x / ys;
}

SCALAR_FUN_ATTR int64_t squot_safe64(int64_t x, int64_t y) {
  int64_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x / ys;
}

SCALAR_FUN_ATTR int8_t srem_safe8(int8_t x, int8_t y) {
  int8_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x % ys;
}

SCALAR_FUN_ATTR int16_t srem_safe16(int16_t x, int16_t y) {
  int16_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x % ys;
}

SCALAR_FUN_ATTR int32_t srem_safe32(int32_t x, int32_t y) {
  int32_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x % ys;
}

SCALAR_FUN_ATTR int64_t srem_safe64(int64_t x, int64_t y) {
  int64_t ys = 1;
  foreach_active(i) { ys = y; }
  return y == 0 ? 0 : x % ys;
}

#else

SCALAR_FUN_ATTR uint8_t   udiv8(uint8_t x, uint8_t y)   { return x / y; }
SCALAR_FUN_ATTR uint16_t udiv16(uint16_t x, uint16_t y) { return x / y; }
SCALAR_FUN_ATTR uint32_t udiv32(uint32_t x, uint32_t y) { return x / y; }
SCALAR_FUN_ATTR uint64_t udiv64(uint64_t x, uint64_t y) { return x / y; }

SCALAR_FUN_ATTR uint8_t   udiv_up8(uint8_t x, uint8_t y)   { return (x + y - 1) / y; }
SCALAR_FUN_ATTR uint16_t udiv_up16(uint16_t x, uint16_t y) { return (x + y - 1) / y; }
SCALAR_FUN_ATTR uint32_t udiv_up32(uint32_t x, uint32_t y) { return (x + y - 1) / y; }
SCALAR_FUN_ATTR uint64_t udiv_up64(uint64_t x, uint64_t y) { return (x + y - 1) / y; }

SCALAR_FUN_ATTR uint8_t   umod8(uint8_t x, uint8_t y)   { return x % y; }
SCALAR_FUN_ATTR uint16_t umod16(uint16_t x, uint16_t y) { return x % y; }
SCALAR_FUN_ATTR uint32_t umod32(uint32_t x, uint32_t y) { return x % y; }
SCALAR_FUN_ATTR uint64_t umod64(uint64_t x, uint64_t y) { return x % y; }

SCALAR_FUN_ATTR uint8_t   udiv_safe8(uint8_t x, uint8_t y)   { return y == 0 ? 0 : x / y; }
SCALAR_FUN_ATTR uint16_t udiv_safe16(uint16_t x, uint16_t y) { return y == 0 ? 0 : x / y; }
SCALAR_FUN_ATTR uint32_t udiv_safe32(uint32_t x, uint32_t y) { return y == 0 ? 0 : x / y; }
SCALAR_FUN_ATTR uint64_t udiv_safe64(uint64_t x, uint64_t y) { return y == 0 ? 0 : x / y; }

SCALAR_FUN_ATTR uint8_t   udiv_up_safe8(uint8_t x, uint8_t y)   { return y == 0 ? 0 : (x + y - 1) / y; }
SCALAR_FUN_ATTR uint16_t udiv_up_safe16(uint16_t x, uint16_t y) { return y == 0 ? 0 : (x + y - 1) / y; }
SCALAR_FUN_ATTR uint32_t udiv_up_safe32(uint32_t x, uint32_t y) { return y == 0 ? 0 : (x + y - 1) / y; }
SCALAR_FUN_ATTR uint64_t udiv_up_safe64(uint64_t x, uint64_t y) { return y == 0 ? 0 : (x + y - 1) / y; }

SCALAR_FUN_ATTR uint8_t   umod_safe8(uint8_t x, uint8_t y)   { return y == 0 ? 0 : x % y; }
SCALAR_FUN_ATTR uint16_t umod_safe16(uint16_t x, uint16_t y) { return y == 0 ? 0 : x % y; }
SCALAR_FUN_ATTR uint32_t umod_safe32(uint32_t x, uint32_t y) { return y == 0 ? 0 : x % y; }
SCALAR_FUN_ATTR uint64_t umod_safe64(uint64_t x, uint64_t y) { return y == 0 ? 0 : x % y; }

SCALAR_FUN_ATTR int8_t sdiv8(int8_t x, int8_t y) {
  int8_t q = x / y;
  int8_t r = x % y;
  return q - ((r != 0 && r < 0 != y < 0) ? 1 : 0);
}

SCALAR_FUN_ATTR int16_t sdiv16(int16_t x, int16_t y) {
  int16_t q = x / y;
  int16_t r = x % y;
  return q - ((r != 0 && r < 0 != y < 0) ? 1 : 0);
}

SCALAR_FUN_ATTR int32_t sdiv32(int32_t x, int32_t y) {
  int32_t q = x / y;
  int32_t r = x % y;
  return q - ((r != 0 && r < 0 != y < 0) ? 1 : 0);
}

SCALAR_FUN_ATTR int64_t sdiv64(int64_t x, int64_t y) {
  int64_t q = x / y;
  int64_t r = x % y;
  return q - ((r != 0 && r < 0 != y < 0) ? 1 : 0);
}

SCALAR_FUN_ATTR int8_t   sdiv_up8(int8_t x, int8_t y)   { return sdiv8(x + y - 1, y); }
SCALAR_FUN_ATTR int16_t sdiv_up16(int16_t x, int16_t y) { return sdiv16(x + y - 1, y); }
SCALAR_FUN_ATTR int32_t sdiv_up32(int32_t x, int32_t y) { return sdiv32(x + y - 1, y); }
SCALAR_FUN_ATTR int64_t sdiv_up64(int64_t x, int64_t y) { return sdiv64(x + y - 1, y); }

SCALAR_FUN_ATTR int8_t smod8(int8_t x, int8_t y) {
  int8_t r = x % y;
  return r + (r == 0 || (x > 0 && y > 0) || (x < 0 && y < 0) ? 0 : y);
}

SCALAR_FUN_ATTR int16_t smod16(int16_t x, int16_t y) {
  int16_t r = x % y;
  return r + (r == 0 || (x > 0 && y > 0) || (x < 0 && y < 0) ? 0 : y);
}

SCALAR_FUN_ATTR int32_t smod32(int32_t x, int32_t y) {
  int32_t r = x % y;
  return r + (r == 0 || (x > 0 && y > 0) || (x < 0 && y < 0) ? 0 : y);
}

SCALAR_FUN_ATTR int64_t smod64(int64_t x, int64_t y) {
  int64_t r = x % y;
  return r + (r == 0 || (x > 0 && y > 0) || (x < 0 && y < 0) ? 0 : y);
}

SCALAR_FUN_ATTR int8_t   sdiv_safe8(int8_t x, int8_t y)   { return y == 0 ? 0 : sdiv8(x, y); }
SCALAR_FUN_ATTR int16_t sdiv_safe16(int16_t x, int16_t y) { return y == 0 ? 0 : sdiv16(x, y); }
SCALAR_FUN_ATTR int32_t sdiv_safe32(int32_t x, int32_t y) { return y == 0 ? 0 : sdiv32(x, y); }
SCALAR_FUN_ATTR int64_t sdiv_safe64(int64_t x, int64_t y) { return y == 0 ? 0 : sdiv64(x, y); }

SCALAR_FUN_ATTR int8_t   sdiv_up_safe8(int8_t x, int8_t y)   { return sdiv_safe8(x + y - 1, y);}
SCALAR_FUN_ATTR int16_t sdiv_up_safe16(int16_t x, int16_t y) { return sdiv_safe16(x + y - 1, y); }
SCALAR_FUN_ATTR int32_t sdiv_up_safe32(int32_t x, int32_t y) { return sdiv_safe32(x + y - 1, y); }
SCALAR_FUN_ATTR int64_t sdiv_up_safe64(int64_t x, int64_t y) { return sdiv_safe64(x + y - 1, y); }

SCALAR_FUN_ATTR int8_t   smod_safe8(int8_t x, int8_t y)   { return y == 0 ? 0 : smod8(x, y); }
SCALAR_FUN_ATTR int16_t smod_safe16(int16_t x, int16_t y) { return y == 0 ? 0 : smod16(x, y); }
SCALAR_FUN_ATTR int32_t smod_safe32(int32_t x, int32_t y) { return y == 0 ? 0 : smod32(x, y); }
SCALAR_FUN_ATTR int64_t smod_safe64(int64_t x, int64_t y) { return y == 0 ? 0 : smod64(x, y); }

SCALAR_FUN_ATTR int8_t   squot8(int8_t x, int8_t y)   { return x / y; }
SCALAR_FUN_ATTR int16_t squot16(int16_t x, int16_t y) { return x / y; }
SCALAR_FUN_ATTR int32_t squot32(int32_t x, int32_t y) { return x / y; }
SCALAR_FUN_ATTR int64_t squot64(int64_t x, int64_t y) { return x / y; }

SCALAR_FUN_ATTR int8_t   srem8(int8_t x, int8_t y)   { return x % y; }
SCALAR_FUN_ATTR int16_t srem16(int16_t x, int16_t y) { return x % y; }
SCALAR_FUN_ATTR int32_t srem32(int32_t x, int32_t y) { return x % y; }
SCALAR_FUN_ATTR int64_t srem64(int64_t x, int64_t y) { return x % y; }

SCALAR_FUN_ATTR int8_t   squot_safe8(int8_t x, int8_t y)   { return y == 0 ? 0 : x / y; }
SCALAR_FUN_ATTR int16_t squot_safe16(int16_t x, int16_t y) { return y == 0 ? 0 : x / y; }
SCALAR_FUN_ATTR int32_t squot_safe32(int32_t x, int32_t y) { return y == 0 ? 0 : x / y; }
SCALAR_FUN_ATTR int64_t squot_safe64(int64_t x, int64_t y) { return y == 0 ? 0 : x / y; }

SCALAR_FUN_ATTR int8_t   srem_safe8(int8_t x, int8_t y)   { return y == 0 ? 0 : x % y; }
SCALAR_FUN_ATTR int16_t srem_safe16(int16_t x, int16_t y) { return y == 0 ? 0 : x % y; }
SCALAR_FUN_ATTR int32_t srem_safe32(int32_t x, int32_t y) { return y == 0 ? 0 : x % y; }
SCALAR_FUN_ATTR int64_t srem_safe64(int64_t x, int64_t y) { return y == 0 ? 0 : x % y; }

#endif

SCALAR_FUN_ATTR int8_t   smin8(int8_t x, int8_t y)   { return x < y ? x : y; }
SCALAR_FUN_ATTR int16_t smin16(int16_t x, int16_t y) { return x < y ? x : y; }
SCALAR_FUN_ATTR int32_t smin32(int32_t x, int32_t y) { return x < y ? x : y; }
SCALAR_FUN_ATTR int64_t smin64(int64_t x, int64_t y) { return x < y ? x : y; }

SCALAR_FUN_ATTR uint8_t   umin8(uint8_t x, uint8_t y)   { return x < y ? x : y; }
SCALAR_FUN_ATTR uint16_t umin16(uint16_t x, uint16_t y) { return x < y ? x : y; }
SCALAR_FUN_ATTR uint32_t umin32(uint32_t x, uint32_t y) { return x < y ? x : y; }
SCALAR_FUN_ATTR uint64_t umin64(uint64_t x, uint64_t y) { return x < y ? x : y; }

SCALAR_FUN_ATTR int8_t  smax8(int8_t x, int8_t y)    { return x < y ? y : x; }
SCALAR_FUN_ATTR int16_t smax16(int16_t x, int16_t y) { return x < y ? y : x; }
SCALAR_FUN_ATTR int32_t smax32(int32_t x, int32_t y) { return x < y ? y : x; }
SCALAR_FUN_ATTR int64_t smax64(int64_t x, int64_t y) { return x < y ? y : x; }

SCALAR_FUN_ATTR uint8_t   umax8(uint8_t x, uint8_t y)   { return x < y ? y : x; }
SCALAR_FUN_ATTR uint16_t umax16(uint16_t x, uint16_t y) { return x < y ? y : x; }
SCALAR_FUN_ATTR uint32_t umax32(uint32_t x, uint32_t y) { return x < y ? y : x; }
SCALAR_FUN_ATTR uint64_t umax64(uint64_t x, uint64_t y) { return x < y ? y : x; }

SCALAR_FUN_ATTR uint8_t   shl8(uint8_t x, uint8_t y)   { return (uint8_t)(x << y); }
SCALAR_FUN_ATTR uint16_t shl16(uint16_t x, uint16_t y) { return (uint16_t)(x << y); }
SCALAR_FUN_ATTR uint32_t shl32(uint32_t x, uint32_t y) { return x << y; }
SCALAR_FUN_ATTR uint64_t shl64(uint64_t x, uint64_t y) { return x << y; }

SCALAR_FUN_ATTR uint8_t   lshr8(uint8_t x, uint8_t y)   { return x >> y; }
SCALAR_FUN_ATTR uint16_t lshr16(uint16_t x, uint16_t y) { return x >> y; }
SCALAR_FUN_ATTR uint32_t lshr32(uint32_t x, uint32_t y) { return x >> y; }
SCALAR_FUN_ATTR uint64_t lshr64(uint64_t x, uint64_t y) { return x >> y; }

SCALAR_FUN_ATTR int8_t   ashr8(int8_t x, int8_t y)   { return x >> y; }
SCALAR_FUN_ATTR int16_t ashr16(int16_t x, int16_t y) { return x >> y; }
SCALAR_FUN_ATTR int32_t ashr32(int32_t x, int32_t y) { return x >> y; }
SCALAR_FUN_ATTR int64_t ashr64(int64_t x, int64_t y) { return x >> y; }

SCALAR_FUN_ATTR uint8_t   and8(uint8_t x, uint8_t y)   { return x & y; }
SCALAR_FUN_ATTR uint16_t and16(uint16_t x, uint16_t y) { return x & y; }
SCALAR_FUN_ATTR uint32_t and32(uint32_t x, uint32_t y) { return x & y; }
SCALAR_FUN_ATTR uint64_t and64(uint64_t x, uint64_t y) { return x & y; }

SCALAR_FUN_ATTR uint8_t    or8(uint8_t x, uint8_t y)  { return x | y; }
SCALAR_FUN_ATTR uint16_t or16(uint16_t x, uint16_t y) { return x | y; }
SCALAR_FUN_ATTR uint32_t or32(uint32_t x, uint32_t y) { return x | y; }
SCALAR_FUN_ATTR uint64_t or64(uint64_t x, uint64_t y) { return x | y; }

SCALAR_FUN_ATTR uint8_t   xor8(uint8_t x, uint8_t y)   { return x ^ y; }
SCALAR_FUN_ATTR uint16_t xor16(uint16_t x, uint16_t y) { return x ^ y; }
SCALAR_FUN_ATTR uint32_t xor32(uint32_t x, uint32_t y) { return x ^ y; }
SCALAR_FUN_ATTR uint64_t xor64(uint64_t x, uint64_t y) { return x ^ y; }

SCALAR_FUN_ATTR bool ult8(uint8_t x, uint8_t y)    { return x < y; }
SCALAR_FUN_ATTR bool ult16(uint16_t x, uint16_t y) { return x < y; }
SCALAR_FUN_ATTR bool ult32(uint32_t x, uint32_t y) { return x < y; }
SCALAR_FUN_ATTR bool ult64(uint64_t x, uint64_t y) { return x < y; }

SCALAR_FUN_ATTR bool ule8(uint8_t x, uint8_t y)    { return x <= y; }
SCALAR_FUN_ATTR bool ule16(uint16_t x, uint16_t y) { return x <= y; }
SCALAR_FUN_ATTR bool ule32(uint32_t x, uint32_t y) { return x <= y; }
SCALAR_FUN_ATTR bool ule64(uint64_t x, uint64_t y) { return x <= y; }

SCALAR_FUN_ATTR bool  slt8(int8_t x, int8_t y)   { return x < y; }
SCALAR_FUN_ATTR bool slt16(int16_t x, int16_t y) { return x < y; }
SCALAR_FUN_ATTR bool slt32(int32_t x, int32_t y) { return x < y; }
SCALAR_FUN_ATTR bool slt64(int64_t x, int64_t y) { return x < y; }

SCALAR_FUN_ATTR bool  sle8(int8_t x, int8_t y)   { return x <= y; }
SCALAR_FUN_ATTR bool sle16(int16_t x, int16_t y) { return x <= y; }
SCALAR_FUN_ATTR bool sle32(int32_t x, int32_t y) { return x <= y; }
SCALAR_FUN_ATTR bool sle64(int64_t x, int64_t y) { return x <= y; }

SCALAR_FUN_ATTR uint8_t pow8(uint8_t x, uint8_t y) {
  uint8_t res = 1, rem = y;
  while (rem != 0) {
    if (rem & 1)
      res *= x;
    rem >>= 1;
    x *= x;
  }
  return res;
}

SCALAR_FUN_ATTR uint16_t pow16(uint16_t x, uint16_t y) {
  uint16_t res = 1, rem = y;
  while (rem != 0) {
    if (rem & 1)
      res *= x;
    rem >>= 1;
    x *= x;
  }
  return res;
}

SCALAR_FUN_ATTR uint32_t pow32(uint32_t x, uint32_t y) {
  uint32_t res = 1, rem = y;
  while (rem != 0) {
    if (rem & 1)
      res *= x;
    rem >>= 1;
    x *= x;
  }
  return res;
}

SCALAR_FUN_ATTR uint64_t pow64(uint64_t x, uint64_t y) {
  uint64_t res = 1, rem = y;
  while (rem != 0) {
    if (rem & 1)
      res *= x;
    rem >>= 1;
    x *= x;
  }
  return res;
}

SCALAR_FUN_ATTR bool  itob_i8_bool(int8_t x)  { return x != 0; }
SCALAR_FUN_ATTR bool itob_i16_bool(int16_t x) { return x != 0; }
SCALAR_FUN_ATTR bool itob_i32_bool(int32_t x) { return x != 0; }
SCALAR_FUN_ATTR bool itob_i64_bool(int64_t x) { return x != 0; }

SCALAR_FUN_ATTR int8_t btoi_bool_i8(bool x)   { return x; }
SCALAR_FUN_ATTR int16_t btoi_bool_i16(bool x) { return x; }
SCALAR_FUN_ATTR int32_t btoi_bool_i32(bool x) { return x; }
SCALAR_FUN_ATTR int64_t btoi_bool_i64(bool x) { return x; }

#define sext_i8_i8(x) ((int8_t) (int8_t) (x))
#define sext_i8_i16(x) ((int16_t) (int8_t) (x))
#define sext_i8_i32(x) ((int32_t) (int8_t) (x))
#define sext_i8_i64(x) ((int64_t) (int8_t) (x))
#define sext_i16_i8(x) ((int8_t) (int16_t) (x))
#define sext_i16_i16(x) ((int16_t) (int16_t) (x))
#define sext_i16_i32(x) ((int32_t) (int16_t) (x))
#define sext_i16_i64(x) ((int64_t) (int16_t) (x))
#define sext_i32_i8(x) ((int8_t) (int32_t) (x))
#define sext_i32_i16(x) ((int16_t) (int32_t) (x))
#define sext_i32_i32(x) ((int32_t) (int32_t) (x))
#define sext_i32_i64(x) ((int64_t) (int32_t) (x))
#define sext_i64_i8(x) ((int8_t) (int64_t) (x))
#define sext_i64_i16(x) ((int16_t) (int64_t) (x))
#define sext_i64_i32(x) ((int32_t) (int64_t) (x))
#define sext_i64_i64(x) ((int64_t) (int64_t) (x))
#define zext_i8_i8(x) ((int8_t) (uint8_t) (x))
#define zext_i8_i16(x) ((int16_t) (uint8_t) (x))
#define zext_i8_i32(x) ((int32_t) (uint8_t) (x))
#define zext_i8_i64(x) ((int64_t) (uint8_t) (x))
#define zext_i16_i8(x) ((int8_t) (uint16_t) (x))
#define zext_i16_i16(x) ((int16_t) (uint16_t) (x))
#define zext_i16_i32(x) ((int32_t) (uint16_t) (x))
#define zext_i16_i64(x) ((int64_t) (uint16_t) (x))
#define zext_i32_i8(x) ((int8_t) (uint32_t) (x))
#define zext_i32_i16(x) ((int16_t) (uint32_t) (x))
#define zext_i32_i32(x) ((int32_t) (uint32_t) (x))
#define zext_i32_i64(x) ((int64_t) (uint32_t) (x))
#define zext_i64_i8(x) ((int8_t) (uint64_t) (x))
#define zext_i64_i16(x) ((int16_t) (uint64_t) (x))
#define zext_i64_i32(x) ((int32_t) (uint64_t) (x))
#define zext_i64_i64(x) ((int64_t) (uint64_t) (x))

SCALAR_FUN_ATTR int8_t   abs8(int8_t x)  { return (int8_t)abs(x); }
SCALAR_FUN_ATTR int16_t abs16(int16_t x) { return (int16_t)abs(x); }
SCALAR_FUN_ATTR int32_t abs32(int32_t x) { return abs(x); }
SCALAR_FUN_ATTR int64_t abs64(int64_t x) {
#if defined(__OPENCL_VERSION__) || defined(ISPC)
  return abs(x);
#else
  return llabs(x);
#endif
}

#if defined(__OPENCL_VERSION__)

SCALAR_FUN_ATTR int32_t  futrts_popc8(int8_t x)  { return popcount(x); }
SCALAR_FUN_ATTR int32_t futrts_popc16(int16_t x) { return popcount(x); }
SCALAR_FUN_ATTR int32_t futrts_popc32(int32_t x) { return popcount(x); }
SCALAR_FUN_ATTR int32_t futrts_popc64(int64_t x) { return popcount(x); }

#elif defined(__CUDA_ARCH__)

SCALAR_FUN_ATTR int32_t  futrts_popc8(int8_t x)  { return __popc(zext_i8_i32(x)); }
SCALAR_FUN_ATTR int32_t futrts_popc16(int16_t x) { return __popc(zext_i16_i32(x)); }
SCALAR_FUN_ATTR int32_t futrts_popc32(int32_t x) { return __popc(x); }
SCALAR_FUN_ATTR int32_t futrts_popc64(int64_t x) { return __popcll(x); }

#else // Not OpenCL or CUDA, but plain C.

SCALAR_FUN_ATTR int32_t futrts_popc8(uint8_t x) {
  int c = 0;
  for (; x; ++c) { x &= x - 1; }
  return c;
}

SCALAR_FUN_ATTR int32_t futrts_popc16(uint16_t x) {
  int c = 0;
  for (; x; ++c) { x &= x - 1; }
  return c;
}

SCALAR_FUN_ATTR int32_t futrts_popc32(uint32_t x) {
  int c = 0;
  for (; x; ++c) { x &= x - 1; }
  return c;
}

SCALAR_FUN_ATTR int32_t futrts_popc64(uint64_t x) {
  int c = 0;
  for (; x; ++c) { x &= x - 1; }
  return c;
}
#endif

#if defined(__OPENCL_VERSION__)
SCALAR_FUN_ATTR uint8_t  futrts_umul_hi8 ( uint8_t a,  uint8_t b) { return mul_hi(a, b); }
SCALAR_FUN_ATTR uint16_t futrts_umul_hi16(uint16_t a, uint16_t b) { return mul_hi(a, b); }
SCALAR_FUN_ATTR uint32_t futrts_umul_hi32(uint32_t a, uint32_t b) { return mul_hi(a, b); }
SCALAR_FUN_ATTR uint64_t futrts_umul_hi64(uint64_t a, uint64_t b) { return mul_hi(a, b); }
SCALAR_FUN_ATTR uint8_t  futrts_smul_hi8 ( int8_t a,  int8_t b) { return mul_hi(a, b); }
SCALAR_FUN_ATTR uint16_t futrts_smul_hi16(int16_t a, int16_t b) { return mul_hi(a, b); }
SCALAR_FUN_ATTR uint32_t futrts_smul_hi32(int32_t a, int32_t b) { return mul_hi(a, b); }
SCALAR_FUN_ATTR uint64_t futrts_smul_hi64(int64_t a, int64_t b) { return mul_hi(a, b); }
#elif defined(__CUDA_ARCH__)
SCALAR_FUN_ATTR  uint8_t futrts_umul_hi8(uint8_t a, uint8_t b) { return ((uint16_t)a) * ((uint16_t)b) >> 8; }
SCALAR_FUN_ATTR uint16_t futrts_umul_hi16(uint16_t a, uint16_t b) { return ((uint32_t)a) * ((uint32_t)b) >> 16; }
SCALAR_FUN_ATTR uint32_t futrts_umul_hi32(uint32_t a, uint32_t b) { return __umulhi(a, b); }
SCALAR_FUN_ATTR uint64_t futrts_umul_hi64(uint64_t a, uint64_t b) { return __umul64hi(a, b); }
SCALAR_FUN_ATTR  uint8_t futrts_smul_hi8 ( int8_t a, int8_t b) { return ((int16_t)a) * ((int16_t)b) >> 8; }
SCALAR_FUN_ATTR uint16_t futrts_smul_hi16(int16_t a, int16_t b) { return ((int32_t)a) * ((int32_t)b) >> 16; }
SCALAR_FUN_ATTR uint32_t futrts_smul_hi32(int32_t a, int32_t b) { return __mulhi(a, b); }
SCALAR_FUN_ATTR uint64_t futrts_smul_hi64(int64_t a, int64_t b) { return __mul64hi(a, b); }
#elif defined(ISPC)
SCALAR_FUN_ATTR uint8_t futrts_umul_hi8(uint8_t a, uint8_t b) { return ((uint16_t)a) * ((uint16_t)b) >> 8; }
SCALAR_FUN_ATTR uint16_t futrts_umul_hi16(uint16_t a, uint16_t b) { return ((uint32_t)a) * ((uint32_t)b) >> 16; }
SCALAR_FUN_ATTR uint32_t futrts_umul_hi32(uint32_t a, uint32_t b) { return ((uint64_t)a) * ((uint64_t)b) >> 32; }
SCALAR_FUN_ATTR uint64_t futrts_umul_hi64(uint64_t a, uint64_t b) {
  uint64_t ah = a >> 32;
  uint64_t al = a & 0xffffffff;
  uint64_t bh = b >> 32;
  uint64_t bl = b & 0xffffffff;

  uint64_t p1 = al * bl;
  uint64_t p2 = al * bh;
  uint64_t p3 = ah * bl;
  uint64_t p4 = ah * bh;

  uint64_t p1h = p1 >> 32;
  uint64_t p2h = p2 >> 32;
  uint64_t p3h = p3 >> 32;
  uint64_t p2l = p2 & 0xffffffff;
  uint64_t p3l = p3 & 0xffffffff;

  uint64_t l = p1h + p2l + p3l;
  uint64_t m = (p2 >> 32) + (p3 >> 32);
  uint64_t h = (l >> 32) + m + p4;

  return h;
}
SCALAR_FUN_ATTR  int8_t futrts_smul_hi8 ( int8_t a,  int8_t b) { return ((uint16_t)a) * ((uint16_t)b) >> 8; }
SCALAR_FUN_ATTR int16_t futrts_smul_hi16(int16_t a, int16_t b) { return ((uint32_t)a) * ((uint32_t)b) >> 16; }
SCALAR_FUN_ATTR int32_t futrts_smul_hi32(int32_t a, int32_t b) { return ((uint64_t)a) * ((uint64_t)b) >> 32; }
SCALAR_FUN_ATTR int64_t futrts_smul_hi64(int64_t a, int64_t b) {
  uint64_t ah = a >> 32;
  uint64_t al = a & 0xffffffff;
  uint64_t bh = b >> 32;
  uint64_t bl = b & 0xffffffff;

  uint64_t p1 =  al * bl;
  int64_t  p2 = al * bh;
  int64_t  p3 = ah * bl;
  uint64_t p4 =  ah * bh;

  uint64_t p1h = p1 >> 32;
  uint64_t p2h = p2 >> 32;
  uint64_t p3h = p3 >> 32;
  uint64_t p2l = p2 & 0xffffffff;
  uint64_t p3l = p3 & 0xffffffff;

  uint64_t l = p1h + p2l + p3l;
  uint64_t m = (p2 >> 32) + (p3 >> 32);
  uint64_t h = (l >> 32) + m + p4;

  return h;
}

#else // Not OpenCL, ISPC, or CUDA, but plain C.
SCALAR_FUN_ATTR uint8_t futrts_umul_hi8(uint8_t a, uint8_t b) { return ((uint16_t)a) * ((uint16_t)b) >> 8; }
SCALAR_FUN_ATTR uint16_t futrts_umul_hi16(uint16_t a, uint16_t b) { return ((uint32_t)a) * ((uint32_t)b) >> 16; }
SCALAR_FUN_ATTR uint32_t futrts_umul_hi32(uint32_t a, uint32_t b) { return ((uint64_t)a) * ((uint64_t)b) >> 32; }
SCALAR_FUN_ATTR uint64_t futrts_umul_hi64(uint64_t a, uint64_t b) { return ((__uint128_t)a) * ((__uint128_t)b) >> 64; }
SCALAR_FUN_ATTR int8_t futrts_smul_hi8(int8_t a, int8_t b) { return ((int16_t)a) * ((int16_t)b) >> 8; }
SCALAR_FUN_ATTR int16_t futrts_smul_hi16(int16_t a, int16_t b) { return ((int32_t)a) * ((int32_t)b) >> 16; }
SCALAR_FUN_ATTR int32_t futrts_smul_hi32(int32_t a, int32_t b) { return ((int64_t)a) * ((int64_t)b) >> 32; }
SCALAR_FUN_ATTR int64_t futrts_smul_hi64(int64_t a, int64_t b) { return ((__int128_t)a) * ((__int128_t)b) >> 64; }
#endif

#if defined(__OPENCL_VERSION__)
SCALAR_FUN_ATTR  uint8_t futrts_umad_hi8 ( uint8_t a,  uint8_t b,  uint8_t c) { return mad_hi(a, b, c); }
SCALAR_FUN_ATTR uint16_t futrts_umad_hi16(uint16_t a, uint16_t b, uint16_t c) { return mad_hi(a, b, c); }
SCALAR_FUN_ATTR uint32_t futrts_umad_hi32(uint32_t a, uint32_t b, uint32_t c) { return mad_hi(a, b, c); }
SCALAR_FUN_ATTR uint64_t futrts_umad_hi64(uint64_t a, uint64_t b, uint64_t c) { return mad_hi(a, b, c); }
SCALAR_FUN_ATTR  uint8_t futrts_smad_hi8( int8_t a,  int8_t b,   int8_t c) { return mad_hi(a, b, c); }
SCALAR_FUN_ATTR uint16_t futrts_smad_hi16(int16_t a, int16_t b, int16_t c) { return mad_hi(a, b, c); }
SCALAR_FUN_ATTR uint32_t futrts_smad_hi32(int32_t a, int32_t b, int32_t c) { return mad_hi(a, b, c); }
SCALAR_FUN_ATTR uint64_t futrts_smad_hi64(int64_t a, int64_t b, int64_t c) { return mad_hi(a, b, c); }
#else // Not OpenCL

SCALAR_FUN_ATTR  uint8_t futrts_umad_hi8( uint8_t a,  uint8_t b,  uint8_t c) { return futrts_umul_hi8(a, b) + c; }
SCALAR_FUN_ATTR uint16_t futrts_umad_hi16(uint16_t a, uint16_t b, uint16_t c) { return futrts_umul_hi16(a, b) + c; }
SCALAR_FUN_ATTR uint32_t futrts_umad_hi32(uint32_t a, uint32_t b, uint32_t c) { return futrts_umul_hi32(a, b) + c; }
SCALAR_FUN_ATTR uint64_t futrts_umad_hi64(uint64_t a, uint64_t b, uint64_t c) { return futrts_umul_hi64(a, b) + c; }
SCALAR_FUN_ATTR  uint8_t futrts_smad_hi8 ( int8_t a,  int8_t b,  int8_t c) { return futrts_smul_hi8(a, b) + c; }
SCALAR_FUN_ATTR uint16_t futrts_smad_hi16(int16_t a, int16_t b, int16_t c) { return futrts_smul_hi16(a, b) + c; }
SCALAR_FUN_ATTR uint32_t futrts_smad_hi32(int32_t a, int32_t b, int32_t c) { return futrts_smul_hi32(a, b) + c; }
SCALAR_FUN_ATTR uint64_t futrts_smad_hi64(int64_t a, int64_t b, int64_t c) { return futrts_smul_hi64(a, b) + c; }
#endif

#if defined(__OPENCL_VERSION__)
SCALAR_FUN_ATTR int32_t  futrts_clzz8(int8_t x)  { return clz(x); }
SCALAR_FUN_ATTR int32_t futrts_clzz16(int16_t x) { return clz(x); }
SCALAR_FUN_ATTR int32_t futrts_clzz32(int32_t x) { return clz(x); }
SCALAR_FUN_ATTR int32_t futrts_clzz64(int64_t x) { return clz(x); }

#elif defined(__CUDA_ARCH__)

SCALAR_FUN_ATTR int32_t  futrts_clzz8(int8_t x)  { return __clz(zext_i8_i32(x)) - 24; }
SCALAR_FUN_ATTR int32_t futrts_clzz16(int16_t x) { return __clz(zext_i16_i32(x)) - 16; }
SCALAR_FUN_ATTR int32_t futrts_clzz32(int32_t x) { return __clz(x); }
SCALAR_FUN_ATTR int32_t futrts_clzz64(int64_t x) { return __clzll(x); }

#elif defined(ISPC)

SCALAR_FUN_ATTR int32_t  futrts_clzz8(int8_t x)  { return count_leading_zeros((int32_t)(uint8_t)x)-24; }
SCALAR_FUN_ATTR int32_t futrts_clzz16(int16_t x) { return count_leading_zeros((int32_t)(uint16_t)x)-16; }
SCALAR_FUN_ATTR int32_t futrts_clzz32(int32_t x) { return count_leading_zeros(x); }
SCALAR_FUN_ATTR int32_t futrts_clzz64(int64_t x) { return count_leading_zeros(x); }

#else // Not OpenCL, ISPC or CUDA, but plain C.

SCALAR_FUN_ATTR int32_t futrts_clzz8(int8_t x)
{ return x == 0 ? 8 : __builtin_clz((uint32_t)zext_i8_i32(x)) - 24; }
SCALAR_FUN_ATTR int32_t futrts_clzz16(int16_t x)
{ return x == 0 ? 16 : __builtin_clz((uint32_t)zext_i16_i32(x)) - 16; }
SCALAR_FUN_ATTR int32_t futrts_clzz32(int32_t x)
{ return x == 0 ? 32 : __builtin_clz((uint32_t)x); }
SCALAR_FUN_ATTR int32_t futrts_clzz64(int64_t x)
{ return x == 0 ? 64 : __builtin_clzll((uint64_t)x); }
#endif

#if defined(__OPENCL_VERSION__)
SCALAR_FUN_ATTR int32_t futrts_ctzz8(int8_t x) {
  int i = 0;
  for (; i < 8 && (x & 1) == 0; i++, x >>= 1) ;
  return i;
}

SCALAR_FUN_ATTR int32_t futrts_ctzz16(int16_t x) {
  int i = 0;
  for (; i < 16 && (x & 1) == 0; i++, x >>= 1) ;
  return i;
}

SCALAR_FUN_ATTR int32_t futrts_ctzz32(int32_t x) {
  int i = 0;
  for (; i < 32 && (x & 1) == 0; i++, x >>= 1) ;
  return i;
}

SCALAR_FUN_ATTR int32_t futrts_ctzz64(int64_t x) {
  int i = 0;
  for (; i < 64 && (x & 1) == 0; i++, x >>= 1) ;
  return i;
}

#elif defined(__CUDA_ARCH__)

SCALAR_FUN_ATTR int32_t futrts_ctzz8(int8_t x) {
  int y = __ffs(x);
  return y == 0 ? 8 : y - 1;
}

SCALAR_FUN_ATTR int32_t futrts_ctzz16(int16_t x) {
  int y = __ffs(x);
  return y == 0 ? 16 : y - 1;
}

SCALAR_FUN_ATTR int32_t futrts_ctzz32(int32_t x) {
  int y = __ffs(x);
  return y == 0 ? 32 : y - 1;
}

SCALAR_FUN_ATTR int32_t futrts_ctzz64(int64_t x) {
  int y = __ffsll(x);
  return y == 0 ? 64 : y - 1;
}

#elif defined(ISPC)

SCALAR_FUN_ATTR int32_t futrts_ctzz8(int8_t x) { return x == 0 ? 8 : count_trailing_zeros((int32_t)x); }
SCALAR_FUN_ATTR int32_t futrts_ctzz16(int16_t x) { return x == 0 ? 16 : count_trailing_zeros((int32_t)x); }
SCALAR_FUN_ATTR int32_t futrts_ctzz32(int32_t x) { return count_trailing_zeros(x); }
SCALAR_FUN_ATTR int32_t futrts_ctzz64(int64_t x) { return count_trailing_zeros(x); }

#else // Not OpenCL or CUDA, but plain C.

SCALAR_FUN_ATTR int32_t  futrts_ctzz8(int8_t x)  { return x == 0 ? 8 : __builtin_ctz((uint32_t)x); }
SCALAR_FUN_ATTR int32_t futrts_ctzz16(int16_t x) { return x == 0 ? 16 : __builtin_ctz((uint32_t)x); }
SCALAR_FUN_ATTR int32_t futrts_ctzz32(int32_t x) { return x == 0 ? 32 : __builtin_ctz((uint32_t)x); }
SCALAR_FUN_ATTR int32_t futrts_ctzz64(int64_t x) { return x == 0 ? 64 : __builtin_ctzll((uint64_t)x); }
#endif

SCALAR_FUN_ATTR float fdiv32(float x, float y) { return x / y; }
SCALAR_FUN_ATTR float fadd32(float x, float y) { return x + y; }
SCALAR_FUN_ATTR float fsub32(float x, float y) { return x - y; }
SCALAR_FUN_ATTR float fmul32(float x, float y) { return x * y; }
SCALAR_FUN_ATTR bool cmplt32(float x, float y) { return x < y; }
SCALAR_FUN_ATTR bool cmple32(float x, float y) { return x <= y; }
SCALAR_FUN_ATTR float sitofp_i8_f32(int8_t x)  { return (float) x; }

SCALAR_FUN_ATTR float sitofp_i16_f32(int16_t x) { return (float) x; }
SCALAR_FUN_ATTR float sitofp_i32_f32(int32_t x) { return (float) x; }
SCALAR_FUN_ATTR float sitofp_i64_f32(int64_t x) { return (float) x; }
SCALAR_FUN_ATTR float  uitofp_i8_f32(uint8_t x)  { return (float) x; }
SCALAR_FUN_ATTR float uitofp_i16_f32(uint16_t x) { return (float) x; }
SCALAR_FUN_ATTR float uitofp_i32_f32(uint32_t x) { return (float) x; }
SCALAR_FUN_ATTR float uitofp_i64_f32(uint64_t x) { return (float) x; }

#ifdef __OPENCL_VERSION__
SCALAR_FUN_ATTR float fabs32(float x)          { return fabs(x); }
SCALAR_FUN_ATTR float fmax32(float x, float y) { return fmax(x, y); }
SCALAR_FUN_ATTR float fmin32(float x, float y) { return fmin(x, y); }
SCALAR_FUN_ATTR float fpow32(float x, float y) { return pow(x, y); }

#elif defined(ISPC)

SCALAR_FUN_ATTR float fabs32(float x) { return abs(x); }
SCALAR_FUN_ATTR float fmax32(float x, float y) { return isnan(x) ? y : isnan(y) ? x : max(x, y); }
SCALAR_FUN_ATTR float fmin32(float x, float y) { return isnan(x) ? y : isnan(y) ? x : min(x, y); }
SCALAR_FUN_ATTR float fpow32(float a, float b) {
  float ret;
  foreach_active (i) {
      uniform float r = pow(extract(a, i), extract(b, i));
      ret = insert(ret, i, r);
  }
  return ret;
}

#else // Not OpenCL, but CUDA or plain C.

SCALAR_FUN_ATTR float fabs32(float x)          { return fabsf(x); }
SCALAR_FUN_ATTR float fmax32(float x, float y) { return fmaxf(x, y); }
SCALAR_FUN_ATTR float fmin32(float x, float y) { return fminf(x, y); }
SCALAR_FUN_ATTR float fpow32(float x, float y) { return powf(x, y); }
#endif

SCALAR_FUN_ATTR bool futrts_isnan32(float x) { return isnan(x); }

#if defined(ISPC)

SCALAR_FUN_ATTR bool futrts_isinf32(float x) { return !isnan(x) && isnan(x - x); }

SCALAR_FUN_ATTR bool futrts_isfinite32(float x) { return !isnan(x) && !futrts_isinf32(x); }

#else

SCALAR_FUN_ATTR bool futrts_isinf32(float x) { return isinf(x); }

#endif

SCALAR_FUN_ATTR int8_t fptosi_f32_i8(float x) {
  if (futrts_isnan32(x) || futrts_isinf32(x)) {
    return 0;
  } else {
    return (int8_t) x;
  }
}

SCALAR_FUN_ATTR int16_t fptosi_f32_i16(float x) {
  if (futrts_isnan32(x) || futrts_isinf32(x)) {
    return 0;
  } else {
    return (int16_t) x;
  }
}

SCALAR_FUN_ATTR int32_t fptosi_f32_i32(float x) {
  if (futrts_isnan32(x) || futrts_isinf32(x)) {
    return 0;
  } else {
    return (int32_t) x;
  }
}

SCALAR_FUN_ATTR int64_t fptosi_f32_i64(float x) {
  if (futrts_isnan32(x) || futrts_isinf32(x)) {
    return 0;
  } else {
    return (int64_t) x;
  };
}

SCALAR_FUN_ATTR uint8_t fptoui_f32_i8(float x) {
  if (futrts_isnan32(x) || futrts_isinf32(x)) {
    return 0;
  } else {
    return (uint8_t) (int8_t) x;
  }
}

SCALAR_FUN_ATTR uint16_t fptoui_f32_i16(float x) {
  if (futrts_isnan32(x) || futrts_isinf32(x)) {
    return 0;
  } else {
    return (uint16_t) (int16_t) x;
  }
}

SCALAR_FUN_ATTR uint32_t fptoui_f32_i32(float x) {
  if (futrts_isnan32(x) || futrts_isinf32(x)) {
    return 0;
  } else {
    return (uint32_t) (int32_t) x;
  }
}

SCALAR_FUN_ATTR uint64_t fptoui_f32_i64(float x) {
  if (futrts_isnan32(x) || futrts_isinf32(x)) {
    return 0;
  } else {
    return (uint64_t) (int64_t) x;
  }
}

SCALAR_FUN_ATTR bool ftob_f32_bool(float x) { return x != 0; }
SCALAR_FUN_ATTR float btof_bool_f32(bool x) { return x ? 1 : 0; }

#ifdef __OPENCL_VERSION__
SCALAR_FUN_ATTR float futrts_log32(float x) { return log(x); }
SCALAR_FUN_ATTR float futrts_log2_32(float x) { return log2(x); }
SCALAR_FUN_ATTR float futrts_log10_32(float x) { return log10(x); }
SCALAR_FUN_ATTR float futrts_log1p_32(float x) { return log1p(x); }
SCALAR_FUN_ATTR float futrts_sqrt32(float x) { return sqrt(x); }
SCALAR_FUN_ATTR float futrts_rsqrt32(float x) { return rsqrt(x); }
SCALAR_FUN_ATTR float futrts_cbrt32(float x) { return cbrt(x); }
SCALAR_FUN_ATTR float futrts_exp32(float x) { return exp(x); }
SCALAR_FUN_ATTR float futrts_cos32(float x) { return cos(x); }
SCALAR_FUN_ATTR float futrts_cospi32(float x) { return cospi(x); }
SCALAR_FUN_ATTR float futrts_sin32(float x) { return sin(x); }
SCALAR_FUN_ATTR float futrts_sinpi32(float x) { return sinpi(x); }
SCALAR_FUN_ATTR float futrts_tan32(float x) { return tan(x); }
SCALAR_FUN_ATTR float futrts_tanpi32(float x) { return tanpi(x); }
SCALAR_FUN_ATTR float futrts_acos32(float x) { return acos(x); }
SCALAR_FUN_ATTR float futrts_acospi32(float x) { return acospi(x); }
SCALAR_FUN_ATTR float futrts_asin32(float x) { return asin(x); }
SCALAR_FUN_ATTR float futrts_asinpi32(float x) { return asinpi(x); }
SCALAR_FUN_ATTR float futrts_atan32(float x) { return atan(x); }
SCALAR_FUN_ATTR float futrts_atanpi32(float x) { return atanpi(x); }
SCALAR_FUN_ATTR float futrts_cosh32(float x) { return cosh(x); }
SCALAR_FUN_ATTR float futrts_sinh32(float x) { return sinh(x); }
SCALAR_FUN_ATTR float futrts_tanh32(float x) { return tanh(x); }
SCALAR_FUN_ATTR float futrts_acosh32(float x) { return acosh(x); }
SCALAR_FUN_ATTR float futrts_asinh32(float x) { return asinh(x); }
SCALAR_FUN_ATTR float futrts_atanh32(float x) { return atanh(x); }
SCALAR_FUN_ATTR float futrts_atan2_32(float x, float y) { return atan2(x, y); }
SCALAR_FUN_ATTR float futrts_atan2pi_32(float x, float y) { return atan2pi(x, y); }
SCALAR_FUN_ATTR float futrts_hypot32(float x, float y) { return hypot(x, y); }
SCALAR_FUN_ATTR float futrts_gamma32(float x) { return tgamma(x); }
SCALAR_FUN_ATTR float futrts_lgamma32(float x) { return lgamma(x); }
SCALAR_FUN_ATTR float futrts_erf32(float x) { return erf(x); }
SCALAR_FUN_ATTR float futrts_erfc32(float x) { return erfc(x); }
SCALAR_FUN_ATTR float fmod32(float x, float y) { return fmod(x, y); }
SCALAR_FUN_ATTR float futrts_round32(float x) { return rint(x); }
SCALAR_FUN_ATTR float futrts_floor32(float x) { return floor(x); }
SCALAR_FUN_ATTR float futrts_ceil32(float x) { return ceil(x); }
SCALAR_FUN_ATTR float futrts_nextafter32(float x, float y) { return nextafter(x, y); }
SCALAR_FUN_ATTR float futrts_lerp32(float v0, float v1, float t) { return mix(v0, v1, t); }
SCALAR_FUN_ATTR float futrts_ldexp32(float x, int32_t y) { return ldexp(x, y); }
SCALAR_FUN_ATTR float futrts_copysign32(float x, float y) { return copysign(x, y); }
SCALAR_FUN_ATTR float futrts_mad32(float a, float b, float c) { return mad(a, b, c); }
SCALAR_FUN_ATTR float futrts_fma32(float a, float b, float c) { return fma(a, b, c); }

#elif defined(ISPC)

SCALAR_FUN_ATTR float futrts_log32(float x) { return futrts_isfinite32(x) || (futrts_isinf32(x) && x < 0)? log(x) : x; }
SCALAR_FUN_ATTR float futrts_log2_32(float x) { return futrts_log32(x) / log(2.0f); }
SCALAR_FUN_ATTR float futrts_log10_32(float x) { return futrts_log32(x) / log(10.0f); }

SCALAR_FUN_ATTR float futrts_log1p_32(float x) {
  if(x == -1.0f || (futrts_isinf32(x) && x > 0.0f)) return x / 0.0f;
  float y = 1.0f + x;
  float z = y - 1.0f;
  return log(y) - (z-x)/y;
}

SCALAR_FUN_ATTR float futrts_sqrt32(float x) { return sqrt(x); }
SCALAR_FUN_ATTR float futrts_rsqrt32(float x) { return 1/sqrt(x); }

extern "C" unmasked uniform float cbrtf(uniform float);
SCALAR_FUN_ATTR float futrts_cbrt32(float x) {
  float res;
  foreach_active (i) {
    uniform float r = cbrtf(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

SCALAR_FUN_ATTR float futrts_exp32(float x) { return exp(x); }
SCALAR_FUN_ATTR float futrts_cos32(float x) { return cos(x); }
SCALAR_FUN_ATTR float futrts_cospi32(float x) { return cos((float)M_PI*x); }
SCALAR_FUN_ATTR float futrts_sin32(float x) { return sin(x); }
SCALAR_FUN_ATTR float futrts_sinpi32(float x) { return sin(M_PI*x); }
SCALAR_FUN_ATTR float futrts_tan32(float x) { return tan(x); }
SCALAR_FUN_ATTR float futrts_tanpi32(float x) { return tan((float)M_PI*x); }
SCALAR_FUN_ATTR float futrts_acos32(float x) { return acos(x); }
SCALAR_FUN_ATTR float futrts_acospi32(float x) { return acos(x)/(float)M_PI; }
SCALAR_FUN_ATTR float futrts_asin32(float x) { return asin(x); }
SCALAR_FUN_ATTR float futrts_asinpi32(float x) { return asin(x)/(float)M_PI; }
SCALAR_FUN_ATTR float futrts_atan32(float x) { return atan(x); }
SCALAR_FUN_ATTR float futrts_atanpi32(float x) { return atan(x)/(float)M_PI; }
SCALAR_FUN_ATTR float futrts_cosh32(float x) { return (exp(x)+exp(-x)) / 2.0f; }
SCALAR_FUN_ATTR float futrts_sinh32(float x) { return (exp(x)-exp(-x)) / 2.0f; }
SCALAR_FUN_ATTR float futrts_tanh32(float x) { return futrts_sinh32(x)/futrts_cosh32(x); }

SCALAR_FUN_ATTR float futrts_acosh32(float x) {
  float f = x+sqrt(x*x-1);
  if (futrts_isfinite32(f)) return log(f);
  return f;
}

SCALAR_FUN_ATTR float futrts_asinh32(float x) {
  float f = x+sqrt(x*x+1);
  if (futrts_isfinite32(f)) return log(f);
  return f;
}

SCALAR_FUN_ATTR float futrts_atanh32(float x) {
  float f = (1+x)/(1-x);
  if (futrts_isfinite32(f)) return log(f)/2.0f;
  return f;
}

SCALAR_FUN_ATTR float futrts_atan2_32(float x, float y)
{ return (x == 0.0f && y == 0.0f) ? 0.0f : atan2(x, y); }
SCALAR_FUN_ATTR float futrts_atan2pi_32(float x, float y)
{ return (x == 0.0f && y == 0.0f) ? 0.0f : atan2(x, y) / (float)M_PI; }

SCALAR_FUN_ATTR float futrts_hypot32(float x, float y) {
  if (futrts_isfinite32(x) && futrts_isfinite32(y)) {
    x = abs(x);
    y = abs(y);
    float a;
    float b;
    if (x >= y){
        a = x;
        b = y;
    } else {
        a = y;
        b = x;
    }
    if(b == 0){
      return a;
    }

    int e;
    float an;
    float bn;
    an = frexp (a, &e);
    bn = ldexp (b, - e);
    float cn;
    cn = sqrt (an * an + bn * bn);
    return ldexp (cn, e);
  } else {
    if (futrts_isinf32(x) || futrts_isinf32(y)) return INFINITY;
    else return x + y;
  }

}

extern "C" unmasked uniform float tgammaf(uniform float x);
SCALAR_FUN_ATTR float futrts_gamma32(float x) {
  float res;
  foreach_active (i) {
    uniform float r = tgammaf(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

extern "C" unmasked uniform float lgammaf(uniform float x);
SCALAR_FUN_ATTR float futrts_lgamma32(float x) {
  float res;
  foreach_active (i) {
    uniform float r = lgammaf(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

extern "C" unmasked uniform float erff(uniform float x);
SCALAR_FUN_ATTR float futrts_erf32(float x) {
  float res;
  foreach_active (i) {
    uniform float r = erff(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

extern "C" unmasked uniform float erfcf(uniform float x);
SCALAR_FUN_ATTR float futrts_erfc32(float x) {
  float res;
  foreach_active (i) {
    uniform float r = erfcf(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

SCALAR_FUN_ATTR float fmod32(float x, float y) { return x - y * trunc(x/y); }
SCALAR_FUN_ATTR float futrts_round32(float x) { return round(x); }
SCALAR_FUN_ATTR float futrts_floor32(float x) { return floor(x); }
SCALAR_FUN_ATTR float futrts_ceil32(float x) { return ceil(x); }

extern "C" unmasked uniform float nextafterf(uniform float x, uniform float y);
SCALAR_FUN_ATTR float futrts_nextafter32(float x, float y) {
  float res;
  foreach_active (i) {
    uniform float r = nextafterf(extract(x, i), extract(y, i));
    res = insert(res, i, r);
  }
  return res;
}

SCALAR_FUN_ATTR float futrts_lerp32(float v0, float v1, float t) {
  return v0 + (v1 - v0) * t;
}

SCALAR_FUN_ATTR float futrts_ldexp32(float x, int32_t y) {
  return x * pow((uniform float)2.0, (float)y);
}

SCALAR_FUN_ATTR float futrts_copysign32(float x, float y) {
  int32_t xb = fptobits_f32_i32(x);
  int32_t yb = fptobits_f32_i32(y);
  return bitstofp_i32_f32((xb & ~(1<<31)) | (yb & (1<<31)));
}

SCALAR_FUN_ATTR float futrts_mad32(float a, float b, float c) {
  return a * b + c;
}

SCALAR_FUN_ATTR float futrts_fma32(float a, float b, float c) {
  return a * b + c;
}

#else // Not OpenCL or ISPC, but CUDA or plain C.

SCALAR_FUN_ATTR float futrts_log32(float x) { return logf(x); }
SCALAR_FUN_ATTR float futrts_log2_32(float x) { return log2f(x); }
SCALAR_FUN_ATTR float futrts_log10_32(float x) { return log10f(x); }
SCALAR_FUN_ATTR float futrts_log1p_32(float x) { return log1pf(x); }
SCALAR_FUN_ATTR float futrts_sqrt32(float x) { return sqrtf(x); }
SCALAR_FUN_ATTR float futrts_rsqrt32(float x) { return 1/sqrtf(x); }
SCALAR_FUN_ATTR float futrts_cbrt32(float x) { return cbrtf(x); }
SCALAR_FUN_ATTR float futrts_exp32(float x) { return expf(x); }
SCALAR_FUN_ATTR float futrts_cos32(float x) { return cosf(x); }

SCALAR_FUN_ATTR float futrts_cospi32(float x) {
#if defined(__CUDA_ARCH__)
  return cospif(x);
#else
  return cosf(((float)M_PI)*x);
#endif
}
SCALAR_FUN_ATTR float futrts_sin32(float x) { return sinf(x); }

SCALAR_FUN_ATTR float futrts_sinpi32(float x) {
#if defined(__CUDA_ARCH__)
  return sinpif(x);
#else
  return sinf((float)M_PI*x);
#endif
}

SCALAR_FUN_ATTR float futrts_tan32(float x) { return tanf(x); }
SCALAR_FUN_ATTR float futrts_tanpi32(float x) { return tanf((float)M_PI*x); }
SCALAR_FUN_ATTR float futrts_acos32(float x) { return acosf(x); }
SCALAR_FUN_ATTR float futrts_acospi32(float x) { return acosf(x)/(float)M_PI; }
SCALAR_FUN_ATTR float futrts_asin32(float x) { return asinf(x); }
SCALAR_FUN_ATTR float futrts_asinpi32(float x) { return asinf(x)/(float)M_PI; }
SCALAR_FUN_ATTR float futrts_atan32(float x) { return atanf(x); }
SCALAR_FUN_ATTR float futrts_atanpi32(float x) { return atanf(x)/(float)M_PI; }
SCALAR_FUN_ATTR float futrts_cosh32(float x) { return coshf(x); }
SCALAR_FUN_ATTR float futrts_sinh32(float x) { return sinhf(x); }
SCALAR_FUN_ATTR float futrts_tanh32(float x) { return tanhf(x); }
SCALAR_FUN_ATTR float futrts_acosh32(float x) { return acoshf(x); }
SCALAR_FUN_ATTR float futrts_asinh32(float x) { return asinhf(x); }
SCALAR_FUN_ATTR float futrts_atanh32(float x) { return atanhf(x); }
SCALAR_FUN_ATTR float futrts_atan2_32(float x, float y) { return atan2f(x, y); }
SCALAR_FUN_ATTR float futrts_atan2pi_32(float x, float y) { return atan2f(x, y) / (float)M_PI; }
SCALAR_FUN_ATTR float futrts_hypot32(float x, float y) { return hypotf(x, y); }
SCALAR_FUN_ATTR float futrts_gamma32(float x) { return tgammaf(x); }
SCALAR_FUN_ATTR float futrts_lgamma32(float x) { return lgammaf(x); }
SCALAR_FUN_ATTR float futrts_erf32(float x) { return erff(x); }
SCALAR_FUN_ATTR float futrts_erfc32(float x) { return erfcf(x); }
SCALAR_FUN_ATTR float fmod32(float x, float y) { return fmodf(x, y); }
SCALAR_FUN_ATTR float futrts_round32(float x) { return rintf(x); }
SCALAR_FUN_ATTR float futrts_floor32(float x) { return floorf(x); }
SCALAR_FUN_ATTR float futrts_ceil32(float x) { return ceilf(x); }
SCALAR_FUN_ATTR float futrts_nextafter32(float x, float y) { return nextafterf(x, y); }
SCALAR_FUN_ATTR float futrts_lerp32(float v0, float v1, float t) { return v0 + (v1 - v0) * t; }
SCALAR_FUN_ATTR float futrts_ldexp32(float x, int32_t y) { return ldexpf(x, y); }
SCALAR_FUN_ATTR float futrts_copysign32(float x, float y) { return copysignf(x, y); }
SCALAR_FUN_ATTR float futrts_mad32(float a, float b, float c) { return a * b + c; }
SCALAR_FUN_ATTR float futrts_fma32(float a, float b, float c) { return fmaf(a, b, c); }

#endif

#if defined(ISPC)

SCALAR_FUN_ATTR int32_t fptobits_f32_i32(float x) { return intbits(x); }
SCALAR_FUN_ATTR float bitstofp_i32_f32(int32_t x) { return floatbits(x); }
SCALAR_FUN_ATTR uniform int32_t fptobits_f32_i32(uniform float x) { return intbits(x); }
SCALAR_FUN_ATTR uniform float bitstofp_i32_f32(uniform int32_t x) { return floatbits(x); }

#else

SCALAR_FUN_ATTR int32_t fptobits_f32_i32(float x) {
  union {
    float f;
    int32_t t;
  } p;

  p.f = x;
  return p.t;
}

SCALAR_FUN_ATTR float bitstofp_i32_f32(int32_t x) {
  union {
    int32_t f;
    float t;
  } p;

  p.f = x;
  return p.t;
}
#endif

SCALAR_FUN_ATTR float fsignum32(float x) {
  return futrts_isnan32(x) ? x : (x > 0 ? 1 : 0) - (x < 0 ? 1 : 0);
}

#ifdef FUTHARK_F64_ENABLED

SCALAR_FUN_ATTR double bitstofp_i64_f64(int64_t x);
SCALAR_FUN_ATTR int64_t fptobits_f64_i64(double x);

#if defined(ISPC)

SCALAR_FUN_ATTR bool futrts_isinf64(double x) { return !isnan(x) && isnan(x - x); }
SCALAR_FUN_ATTR bool futrts_isfinite64(double x) { return !isnan(x) && !futrts_isinf64(x); }
SCALAR_FUN_ATTR double fdiv64(double x, double y) { return x / y; }
SCALAR_FUN_ATTR double fadd64(double x, double y) { return x + y; }
SCALAR_FUN_ATTR double fsub64(double x, double y) { return x - y; }
SCALAR_FUN_ATTR double fmul64(double x, double y) { return x * y; }
SCALAR_FUN_ATTR bool cmplt64(double x, double y) { return x < y; }
SCALAR_FUN_ATTR bool cmple64(double x, double y) { return x <= y; }
SCALAR_FUN_ATTR double sitofp_i8_f64(int8_t x) { return (double) x; }
SCALAR_FUN_ATTR double sitofp_i16_f64(int16_t x) { return (double) x; }
SCALAR_FUN_ATTR double sitofp_i32_f64(int32_t x) { return (double) x; }
SCALAR_FUN_ATTR double sitofp_i64_f64(int64_t x) { return (double) x; }
SCALAR_FUN_ATTR double uitofp_i8_f64(uint8_t x) { return (double) x; }
SCALAR_FUN_ATTR double uitofp_i16_f64(uint16_t x) { return (double) x; }
SCALAR_FUN_ATTR double uitofp_i32_f64(uint32_t x) { return (double) x; }
SCALAR_FUN_ATTR double uitofp_i64_f64(uint64_t x) { return (double) x; }
SCALAR_FUN_ATTR double fabs64(double x) { return abs(x); }
SCALAR_FUN_ATTR double fmax64(double x, double y) { return isnan(x) ? y : isnan(y) ? x : max(x, y); }
SCALAR_FUN_ATTR double fmin64(double x, double y) { return isnan(x) ? y : isnan(y) ? x : min(x, y); }

SCALAR_FUN_ATTR double fpow64(double a, double b) {
  float ret;
  foreach_active (i) {
      uniform float r = pow(extract(a, i), extract(b, i));
      ret = insert(ret, i, r);
  }
  return ret;
}
SCALAR_FUN_ATTR double futrts_log64(double x) { return futrts_isfinite64(x) || (futrts_isinf64(x) && x < 0)? log(x) : x; }
SCALAR_FUN_ATTR double futrts_log2_64(double x) { return futrts_log64(x)/log(2.0d); }
SCALAR_FUN_ATTR double futrts_log10_64(double x) { return futrts_log64(x)/log(10.0d); }

SCALAR_FUN_ATTR double futrts_log1p_64(double x) {
  if(x == -1.0d || (futrts_isinf64(x) && x > 0.0d)) return x / 0.0d;
  double y = 1.0d + x;
  double z = y - 1.0d;
  return log(y) - (z-x)/y;
}

SCALAR_FUN_ATTR double futrts_sqrt64(double x) { return sqrt(x); }
SCALAR_FUN_ATTR double futrts_rsqrt64(double x) { return 1/sqrt(x); }

SCALAR_FUN_ATTR double futrts_cbrt64(double x) {
  double res;
  foreach_active (i) {
    uniform double r = cbrtf(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}
SCALAR_FUN_ATTR double futrts_exp64(double x) { return exp(x); }
SCALAR_FUN_ATTR double futrts_cos64(double x) { return cos(x); }
SCALAR_FUN_ATTR double futrts_cospi64(double x) { return cos(M_PI*x); }
SCALAR_FUN_ATTR double futrts_sin64(double x) { return sin(x); }
SCALAR_FUN_ATTR double futrts_sinpi64(double x) { return sin(M_PI*x); }
SCALAR_FUN_ATTR double futrts_tan64(double x) { return tan(x); }
SCALAR_FUN_ATTR double futrts_tanpi64(double x) { return tan(M_PI*x); }
SCALAR_FUN_ATTR double futrts_acos64(double x) { return acos(x); }
SCALAR_FUN_ATTR double futrts_acospi64(double x) { return acos(x)/M_PI; }
SCALAR_FUN_ATTR double futrts_asin64(double x) { return asin(x); }
SCALAR_FUN_ATTR double futrts_asinpi64(double x) { return asin(x)/M_PI; }
SCALAR_FUN_ATTR double futrts_atan64(double x) { return atan(x); }
SCALAR_FUN_ATTR double futrts_atanpi64(double x) { return atan(x)/M_PI; }
SCALAR_FUN_ATTR double futrts_cosh64(double x) { return (exp(x)+exp(-x)) / 2.0d; }
SCALAR_FUN_ATTR double futrts_sinh64(double x) { return (exp(x)-exp(-x)) / 2.0d; }
SCALAR_FUN_ATTR double futrts_tanh64(double x) { return futrts_sinh64(x)/futrts_cosh64(x); }

SCALAR_FUN_ATTR double futrts_acosh64(double x) {
  double f = x+sqrt(x*x-1.0d);
  if(futrts_isfinite64(f)) return log(f);
  return f;
}

SCALAR_FUN_ATTR double futrts_asinh64(double x) {
  double f = x+sqrt(x*x+1.0d);
  if(futrts_isfinite64(f)) return log(f);
  return f;
}

SCALAR_FUN_ATTR double futrts_atanh64(double x) {
  double f = (1.0d+x)/(1.0d-x);
  if(futrts_isfinite64(f)) return log(f)/2.0d;
  return f;
}
SCALAR_FUN_ATTR double futrts_atan2_64(double x, double y) { return atan2(x, y); }

SCALAR_FUN_ATTR double futrts_atan2pi_64(double x, double y) { return atan2(x, y) / M_PI; }

extern "C" unmasked uniform double hypot(uniform double x, uniform double y);
SCALAR_FUN_ATTR double futrts_hypot64(double x, double y) {
  double res;
  foreach_active (i) {
    uniform double r = hypot(extract(x, i), extract(y, i));
    res = insert(res, i, r);
  }
  return res;
}

extern "C" unmasked uniform double tgamma(uniform double x);
SCALAR_FUN_ATTR double futrts_gamma64(double x) {
  double res;
  foreach_active (i) {
    uniform double r = tgamma(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

extern "C" unmasked uniform double lgamma(uniform double x);
SCALAR_FUN_ATTR double futrts_lgamma64(double x) {
  double res;
  foreach_active (i) {
    uniform double r = lgamma(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

extern "C" unmasked uniform double erf(uniform double x);
SCALAR_FUN_ATTR double futrts_erf64(double x) {
  double res;
  foreach_active (i) {
    uniform double r = erf(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

extern "C" unmasked uniform double erfc(uniform double x);
SCALAR_FUN_ATTR double futrts_erfc64(double x) {
  double res;
  foreach_active (i) {
    uniform double r = erfc(extract(x, i));
    res = insert(res, i, r);
  }
  return res;
}

SCALAR_FUN_ATTR double futrts_fma64(double a, double b, double c) { return a * b + c; }
SCALAR_FUN_ATTR double futrts_round64(double x) { return round(x); }
SCALAR_FUN_ATTR double futrts_ceil64(double x) { return ceil(x); }

extern "C" unmasked uniform double nextafter(uniform float x, uniform double y);
SCALAR_FUN_ATTR double futrts_nextafter64(double x, double y) {
  double res;
  foreach_active (i) {
    uniform double r = nextafter(extract(x, i), extract(y, i));
    res = insert(res, i, r);
  }
  return res;
}

SCALAR_FUN_ATTR double futrts_floor64(double x) { return floor(x); }
SCALAR_FUN_ATTR bool futrts_isnan64(double x) { return isnan(x); }

SCALAR_FUN_ATTR int8_t fptosi_f64_i8(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (int8_t) x;
  }
}

SCALAR_FUN_ATTR int16_t fptosi_f64_i16(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (int16_t) x;
  }
}

SCALAR_FUN_ATTR int32_t fptosi_f64_i32(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (int32_t) x;
  }
}

SCALAR_FUN_ATTR int64_t fptosi_f64_i64(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (int64_t) x;
  }
}

SCALAR_FUN_ATTR uint8_t fptoui_f64_i8(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (uint8_t) (int8_t) x;
  }
}

SCALAR_FUN_ATTR uint16_t fptoui_f64_i16(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (uint16_t) (int16_t) x;
  }
}

SCALAR_FUN_ATTR uint32_t fptoui_f64_i32(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (uint32_t) (int32_t) x;
  }
}

SCALAR_FUN_ATTR uint64_t fptoui_f64_i64(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (uint64_t) (int64_t) x;
  }
}

SCALAR_FUN_ATTR bool ftob_f64_bool(double x) { return x != 0.0; }
SCALAR_FUN_ATTR double btof_bool_f64(bool x) { return x ? 1.0 : 0.0; }

SCALAR_FUN_ATTR int64_t fptobits_f64_i64(double x) {
  int64_t res;
  foreach_active (i) {
    uniform double tmp = extract(x, i);
    uniform int64_t r = *((uniform int64_t* uniform)&tmp);
    res = insert(res, i, r);
  }
  return res;
}

SCALAR_FUN_ATTR double bitstofp_i64_f64(int64_t x) {
  double res;
  foreach_active (i) {
    uniform int64_t tmp = extract(x, i);
    uniform double r = *((uniform double* uniform)&tmp);
    res = insert(res, i, r);
  }
  return res;
}

SCALAR_FUN_ATTR uniform int64_t fptobits_f64_i64(uniform double x) {
  return intbits(x);
}

SCALAR_FUN_ATTR uniform double bitstofp_i64_f64(uniform int64_t x) {
  return doublebits(x);
}

SCALAR_FUN_ATTR double fmod64(double x, double y) {
  return x - y * trunc(x/y);
}

SCALAR_FUN_ATTR double fsignum64(double x) {
  return futrts_isnan64(x) ? x : (x > 0 ? 1.0d : 0.0d) - (x < 0 ? 1.0d : 0.0d);
}

SCALAR_FUN_ATTR double futrts_lerp64(double v0, double v1, double t) {
  return v0 + (v1 - v0) * t;
}

SCALAR_FUN_ATTR double futrts_ldexp64(double x, int32_t y) {
  return x * pow((uniform double)2.0, (double)y);
}

SCALAR_FUN_ATTR double futrts_copysign64(double x, double y) {
  int64_t xb = fptobits_f64_i64(x);
  int64_t yb = fptobits_f64_i64(y);
  return bitstofp_i64_f64((xb & ~(((int64_t)1)<<63)) | (yb & (((int64_t)1)<<63)));
}

SCALAR_FUN_ATTR double futrts_mad64(double a, double b, double c) { return a * b + c; }
SCALAR_FUN_ATTR float fpconv_f32_f32(float x) { return (float) x; }
SCALAR_FUN_ATTR double fpconv_f32_f64(float x) { return (double) x; }
SCALAR_FUN_ATTR float fpconv_f64_f32(double x) { return (float) x; }
SCALAR_FUN_ATTR double fpconv_f64_f64(double x) { return (double) x; }

#else

SCALAR_FUN_ATTR double fdiv64(double x, double y) { return x / y; }
SCALAR_FUN_ATTR double fadd64(double x, double y) { return x + y; }
SCALAR_FUN_ATTR double fsub64(double x, double y) { return x - y; }
SCALAR_FUN_ATTR double fmul64(double x, double y) { return x * y; }
SCALAR_FUN_ATTR bool cmplt64(double x, double y) { return x < y; }
SCALAR_FUN_ATTR bool cmple64(double x, double y) { return x <= y; }
SCALAR_FUN_ATTR double sitofp_i8_f64(int8_t x) { return (double) x; }
SCALAR_FUN_ATTR double sitofp_i16_f64(int16_t x) { return (double) x; }
SCALAR_FUN_ATTR double sitofp_i32_f64(int32_t x) { return (double) x; }
SCALAR_FUN_ATTR double sitofp_i64_f64(int64_t x) { return (double) x; }
SCALAR_FUN_ATTR double uitofp_i8_f64(uint8_t x) { return (double) x; }
SCALAR_FUN_ATTR double uitofp_i16_f64(uint16_t x) { return (double) x; }
SCALAR_FUN_ATTR double uitofp_i32_f64(uint32_t x) { return (double) x; }
SCALAR_FUN_ATTR double uitofp_i64_f64(uint64_t x) { return (double) x; }
SCALAR_FUN_ATTR double fabs64(double x) { return fabs(x); }
SCALAR_FUN_ATTR double fmax64(double x, double y) { return fmax(x, y); }
SCALAR_FUN_ATTR double fmin64(double x, double y) { return fmin(x, y); }
SCALAR_FUN_ATTR double fpow64(double x, double y) { return pow(x, y); }
SCALAR_FUN_ATTR double futrts_log64(double x) { return log(x); }
SCALAR_FUN_ATTR double futrts_log2_64(double x) { return log2(x); }
SCALAR_FUN_ATTR double futrts_log10_64(double x) { return log10(x); }
SCALAR_FUN_ATTR double futrts_log1p_64(double x) { return log1p(x); }
SCALAR_FUN_ATTR double futrts_sqrt64(double x) { return sqrt(x); }
SCALAR_FUN_ATTR double futrts_rsqrt64(double x) { return 1/sqrt(x); }
SCALAR_FUN_ATTR double futrts_cbrt64(double x) { return cbrt(x); }
SCALAR_FUN_ATTR double futrts_exp64(double x) { return exp(x); }
SCALAR_FUN_ATTR double futrts_cos64(double x) { return cos(x); }

SCALAR_FUN_ATTR double futrts_cospi64(double x) {
#ifdef __OPENCL_VERSION__
  return cospi(x);
#elif defined(__CUDA_ARCH__)
  return cospi(x);
#else
  return cos(M_PI*x);
#endif
}

SCALAR_FUN_ATTR double futrts_sin64(double x) {
  return sin(x);
}

SCALAR_FUN_ATTR double futrts_sinpi64(double x) {
#ifdef __OPENCL_VERSION__
  return sinpi(x);
#elif defined(__CUDA_ARCH__)
  return sinpi(x);
#else
  return sin(M_PI*x);
#endif
}

SCALAR_FUN_ATTR double futrts_tan64(double x) {
  return tan(x);
}

SCALAR_FUN_ATTR double futrts_tanpi64(double x) {
#ifdef __OPENCL_VERSION__
  return tanpi(x);
#else
  return tan(M_PI*x);
#endif
}

SCALAR_FUN_ATTR double futrts_acos64(double x) {
  return acos(x);
}

SCALAR_FUN_ATTR double futrts_acospi64(double x) {
#ifdef __OPENCL_VERSION__
  return acospi(x);
#else
  return acos(x) / M_PI;
#endif
}

SCALAR_FUN_ATTR double futrts_asin64(double x) {
  return asin(x);
}

SCALAR_FUN_ATTR double futrts_asinpi64(double x) {
#ifdef __OPENCL_VERSION__
  return asinpi(x);
#else
  return asin(x) / M_PI;
#endif
}

SCALAR_FUN_ATTR double futrts_atan64(double x) {
  return atan(x);
}

SCALAR_FUN_ATTR double futrts_atanpi64(double x) {
#ifdef __OPENCL_VERSION__
  return atanpi(x);
#else
  return atan(x) / M_PI;
#endif
}

SCALAR_FUN_ATTR double futrts_cosh64(double x) { return cosh(x); }
SCALAR_FUN_ATTR double futrts_sinh64(double x) { return sinh(x); }
SCALAR_FUN_ATTR double futrts_tanh64(double x) { return tanh(x); }
SCALAR_FUN_ATTR double futrts_acosh64(double x) { return acosh(x); }
SCALAR_FUN_ATTR double futrts_asinh64(double x) { return asinh(x); }
SCALAR_FUN_ATTR double futrts_atanh64(double x) { return atanh(x); }
SCALAR_FUN_ATTR double futrts_atan2_64(double x, double y) { return atan2(x, y); }

SCALAR_FUN_ATTR double futrts_atan2pi_64(double x, double y) {
#ifdef __OPENCL_VERSION__
  return atan2pi(x, y);
#else
  return atan2(x, y) / M_PI;
#endif
}

SCALAR_FUN_ATTR double futrts_hypot64(double x, double y) { return hypot(x, y); }
SCALAR_FUN_ATTR double futrts_gamma64(double x) { return tgamma(x); }
SCALAR_FUN_ATTR double futrts_lgamma64(double x) { return lgamma(x); }
SCALAR_FUN_ATTR double futrts_erf64(double x) { return erf(x); }
SCALAR_FUN_ATTR double futrts_erfc64(double x) { return erfc(x); }
SCALAR_FUN_ATTR double futrts_fma64(double a, double b, double c) { return fma(a, b, c); }
SCALAR_FUN_ATTR double futrts_round64(double x) { return rint(x); }
SCALAR_FUN_ATTR double futrts_ceil64(double x) { return ceil(x); }
SCALAR_FUN_ATTR double futrts_nextafter64(double x, double y) { return nextafter(x, y); }
SCALAR_FUN_ATTR double futrts_floor64(double x) { return floor(x); }
SCALAR_FUN_ATTR bool futrts_isnan64(double x) { return isnan(x); }
SCALAR_FUN_ATTR bool futrts_isinf64(double x) { return isinf(x); }

SCALAR_FUN_ATTR int8_t fptosi_f64_i8(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (int8_t) x;
  }
}

SCALAR_FUN_ATTR int16_t fptosi_f64_i16(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (int16_t) x;
  }
}

SCALAR_FUN_ATTR int32_t fptosi_f64_i32(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (int32_t) x;
  }
}

SCALAR_FUN_ATTR int64_t fptosi_f64_i64(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (int64_t) x;
  }
}

SCALAR_FUN_ATTR uint8_t fptoui_f64_i8(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (uint8_t) (int8_t) x;
  }
}

SCALAR_FUN_ATTR uint16_t fptoui_f64_i16(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (uint16_t) (int16_t) x;
  }
}

SCALAR_FUN_ATTR uint32_t fptoui_f64_i32(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (uint32_t) (int32_t) x;
  }
}

SCALAR_FUN_ATTR uint64_t fptoui_f64_i64(double x) {
  if (futrts_isnan64(x) || futrts_isinf64(x)) {
    return 0;
  } else {
    return (uint64_t) (int64_t) x;
  }
}

SCALAR_FUN_ATTR bool ftob_f64_bool(double x) { return x != 0; }
SCALAR_FUN_ATTR double btof_bool_f64(bool x) { return x ? 1 : 0; }

SCALAR_FUN_ATTR int64_t fptobits_f64_i64(double x) {
  union {
    double f;
    int64_t t;
  } p;

  p.f = x;
  return p.t;
}

SCALAR_FUN_ATTR double bitstofp_i64_f64(int64_t x) {
  union {
    int64_t f;
    double t;
  } p;

  p.f = x;
  return p.t;
}

SCALAR_FUN_ATTR double fmod64(double x, double y) {
  return fmod(x, y);
}

SCALAR_FUN_ATTR double fsignum64(double x) {
  return futrts_isnan64(x) ? x : (x > 0) - (x < 0);
}

SCALAR_FUN_ATTR double futrts_lerp64(double v0, double v1, double t) {
#ifdef __OPENCL_VERSION__
  return mix(v0, v1, t);
#else
  return v0 + (v1 - v0) * t;
#endif
}

SCALAR_FUN_ATTR double futrts_ldexp64(double x, int32_t y) {
  return ldexp(x, y);
}

SCALAR_FUN_ATTR double futrts_copysign64(double x, double y) {
  return copysign(x, y);
}

SCALAR_FUN_ATTR double futrts_mad64(double a, double b, double c) {
#ifdef __OPENCL_VERSION__
  return mad(a, b, c);
#else
  return a * b + c;
#endif
}

SCALAR_FUN_ATTR float fpconv_f32_f32(float x) { return (float) x; }
SCALAR_FUN_ATTR double fpconv_f32_f64(float x) { return (double) x; }
SCALAR_FUN_ATTR float fpconv_f64_f32(double x) { return (float) x; }
SCALAR_FUN_ATTR double fpconv_f64_f64(double x) { return (double) x; }

#endif

#endif

#define futrts_cond_f16(x,y,z) ((x) ? (y) : (z))
#define futrts_cond_f32(x,y,z) ((x) ? (y) : (z))
#define futrts_cond_f64(x,y,z) ((x) ? (y) : (z))

#define futrts_cond_i8(x,y,z) ((x) ? (y) : (z))
#define futrts_cond_i16(x,y,z) ((x) ? (y) : (z))
#define futrts_cond_i32(x,y,z) ((x) ? (y) : (z))
#define futrts_cond_i64(x,y,z) ((x) ? (y) : (z))

#define futrts_cond_bool(x,y,z) ((x) ? (y) : (z))
#define futrts_cond_unit(x,y,z) ((x) ? (y) : (z))

// End of scalar.h.
// Start of scalar_f16.h.

// Half-precision is emulated if needed (e.g. in straight C) with the
// native type used if possible.  The emulation works by typedef'ing
// 'float' to 'f16', and then implementing all operations on single
// precision.  To cut down on duplication, we use the same code for
// those Futhark functions that require just operators or casts.  The
// in-memory representation for arrays will still be 16 bits even
// under emulation, so the compiler will have to be careful when
// generating reads or writes.

#if !defined(cl_khr_fp16) && \
     !(defined(__CUDA_ARCH__) && __CUDA_ARCH__ >= 600) && \
     !(defined(__HIP_DEVICE_COMPILE__)) && \
     !(defined(ISPC))
#define EMULATE_F16
#endif

#if !defined(EMULATE_F16) && defined(__OPENCL_VERSION__)
#pragma OPENCL EXTENSION cl_khr_fp16 : enable
#endif

#ifdef EMULATE_F16

// Note that the half-precision storage format is still 16 bits - the
// compiler will have to be real careful!
typedef float f16;

#elif defined(ISPC)
typedef float16 f16;

#else

#ifdef __CUDA_ARCH__
#include <cuda_fp16.h>
#elif defined(__HIP_DEVICE_COMPILE__)
#include <hip/hip_fp16.h>
#endif

typedef half f16;

#endif

// Some of these functions convert to single precision because half
// precision versions are not available.
SCALAR_FUN_ATTR f16 fadd16(f16 x, f16 y) { return x + y; }
SCALAR_FUN_ATTR f16 fsub16(f16 x, f16 y) { return x - y; }
SCALAR_FUN_ATTR f16 fmul16(f16 x, f16 y) { return x * y; }
SCALAR_FUN_ATTR f16 sitofp_i8_f16(int8_t x) { return (f16) x; }
SCALAR_FUN_ATTR f16 sitofp_i16_f16(int16_t x) { return (f16) x; }
SCALAR_FUN_ATTR f16 sitofp_i32_f16(int32_t x) { return (f16) x; }
SCALAR_FUN_ATTR f16 sitofp_i64_f16(int64_t x) { return (f16) x; }
SCALAR_FUN_ATTR f16 uitofp_i8_f16(uint8_t x) { return (f16) x; }
SCALAR_FUN_ATTR f16 uitofp_i16_f16(uint16_t x) { return (f16) x; }
SCALAR_FUN_ATTR f16 uitofp_i32_f16(uint32_t x) { return (f16) x; }
SCALAR_FUN_ATTR f16 uitofp_i64_f16(uint64_t x) { return (f16) x; }
SCALAR_FUN_ATTR int8_t fptosi_f16_i8(f16 x) { return (int8_t) (float) x; }
SCALAR_FUN_ATTR int16_t fptosi_f16_i16(f16 x) { return (int16_t) x; }
SCALAR_FUN_ATTR int32_t fptosi_f16_i32(f16 x) { return (int32_t) x; }
SCALAR_FUN_ATTR int64_t fptosi_f16_i64(f16 x) { return (int64_t) x; }
SCALAR_FUN_ATTR uint8_t fptoui_f16_i8(f16 x) { return (uint8_t) (float) x; }
SCALAR_FUN_ATTR uint16_t fptoui_f16_i16(f16 x) { return (uint16_t) x; }
SCALAR_FUN_ATTR uint32_t fptoui_f16_i32(f16 x) { return (uint32_t) x; }
SCALAR_FUN_ATTR uint64_t fptoui_f16_i64(f16 x) { return (uint64_t) x; }
SCALAR_FUN_ATTR bool ftob_f16_bool(f16 x) { return x != (f16)0; }
SCALAR_FUN_ATTR f16 btof_bool_f16(bool x) { return x ? 1 : 0; }

#ifndef EMULATE_F16

SCALAR_FUN_ATTR bool futrts_isnan16(f16 x) { return isnan((float)x); }

// compare needs a custom implementation for HIP because of
// https://github.com/ROCm/clr/issues/274
#if defined(__HIP_DEVICE_COMPILE__)

SCALAR_FUN_ATTR bool cmplt16(f16 x, f16 y) { return __hlt(x, y); }
SCALAR_FUN_ATTR bool cmple16(f16 x, f16 y) { return __hle(x, y); }

#else

SCALAR_FUN_ATTR bool cmplt16(f16 x, f16 y) { return x < y; }
SCALAR_FUN_ATTR bool cmple16(f16 x, f16 y) { return x <= y; }

#endif

#ifdef __OPENCL_VERSION__

SCALAR_FUN_ATTR f16 fabs16(f16 x) { return fabs(x); }
SCALAR_FUN_ATTR f16 fmax16(f16 x, f16 y) { return fmax(x, y); }
SCALAR_FUN_ATTR f16 fmin16(f16 x, f16 y) { return fmin(x, y); }
SCALAR_FUN_ATTR f16 fpow16(f16 x, f16 y) { return pow(x, y); }

#elif defined(ISPC)

SCALAR_FUN_ATTR f16 fabs16(f16 x) { return abs(x); }
SCALAR_FUN_ATTR f16 fmax16(f16 x, f16 y) { return futrts_isnan16(x) ? y : futrts_isnan16(y) ? x : max(x, y); }
SCALAR_FUN_ATTR f16 fmin16(f16 x, f16 y) { return futrts_isnan16(x) ? y : futrts_isnan16(y) ? x : min(x, y); }
SCALAR_FUN_ATTR f16 fpow16(f16 x, f16 y) { return pow(x, y); }

#else // Assuming CUDA or HIP.

SCALAR_FUN_ATTR f16 fabs16(f16 x) { return fabsf(x); }
SCALAR_FUN_ATTR f16 fmax16(f16 x, f16 y) { return fmaxf(x, y); }
SCALAR_FUN_ATTR f16 fmin16(f16 x, f16 y) { return fminf(x, y); }
SCALAR_FUN_ATTR f16 fpow16(f16 x, f16 y) { return powf(x, y); }

#endif

#if defined(ISPC)
SCALAR_FUN_ATTR bool futrts_isinf16(float x) { return !futrts_isnan16(x) && futrts_isnan16(x - x); }
SCALAR_FUN_ATTR bool futrts_isfinite16(float x) { return !futrts_isnan16(x) && !futrts_isinf16(x); }
#else
SCALAR_FUN_ATTR bool futrts_isinf16(f16 x) { return isinf((float)x); }
#endif

#ifdef __OPENCL_VERSION__
SCALAR_FUN_ATTR f16 futrts_log16(f16 x) { return log(x); }
SCALAR_FUN_ATTR f16 futrts_log2_16(f16 x) { return log2(x); }
SCALAR_FUN_ATTR f16 futrts_log10_16(f16 x) { return log10(x); }
SCALAR_FUN_ATTR f16 futrts_log1p_16(f16 x) { return log1p(x); }
SCALAR_FUN_ATTR f16 futrts_sqrt16(f16 x) { return sqrt(x); }
SCALAR_FUN_ATTR f16 futrts_rsqrt16(f16 x) { return rsqrt(x); }
SCALAR_FUN_ATTR f16 futrts_cbrt16(f16 x) { return cbrt(x); }
SCALAR_FUN_ATTR f16 futrts_exp16(f16 x) { return exp(x); }
SCALAR_FUN_ATTR f16 futrts_cos16(f16 x) { return cos(x); }
SCALAR_FUN_ATTR f16 futrts_cospi16(f16 x) { return cospi(x); }
SCALAR_FUN_ATTR f16 futrts_sin16(f16 x) { return sin(x); }
SCALAR_FUN_ATTR f16 futrts_sinpi16(f16 x) { return sinpi(x); }
SCALAR_FUN_ATTR f16 futrts_tan16(f16 x) { return tan(x); }
SCALAR_FUN_ATTR f16 futrts_tanpi16(f16 x) { return tanpi(x); }
SCALAR_FUN_ATTR f16 futrts_acos16(f16 x) { return acos(x); }
SCALAR_FUN_ATTR f16 futrts_acospi16(f16 x) { return acospi(x); }
SCALAR_FUN_ATTR f16 futrts_asin16(f16 x) { return asin(x); }
SCALAR_FUN_ATTR f16 futrts_asinpi16(f16 x) { return asinpi(x); }
SCALAR_FUN_ATTR f16 futrts_atan16(f16 x) { return atan(x); }
SCALAR_FUN_ATTR f16 futrts_atanpi16(f16 x) { return atanpi(x); }
SCALAR_FUN_ATTR f16 futrts_cosh16(f16 x) { return cosh(x); }
SCALAR_FUN_ATTR f16 futrts_sinh16(f16 x) { return sinh(x); }
SCALAR_FUN_ATTR f16 futrts_tanh16(f16 x) { return tanh(x); }
SCALAR_FUN_ATTR f16 futrts_acosh16(f16 x) { return acosh(x); }
SCALAR_FUN_ATTR f16 futrts_asinh16(f16 x) { return asinh(x); }
SCALAR_FUN_ATTR f16 futrts_atanh16(f16 x) { return atanh(x); }
SCALAR_FUN_ATTR f16 futrts_atan2_16(f16 x, f16 y) { return atan2(x, y); }
SCALAR_FUN_ATTR f16 futrts_atan2pi_16(f16 x, f16 y) { return atan2pi(x, y); }
SCALAR_FUN_ATTR f16 futrts_hypot16(f16 x, f16 y) { return hypot(x, y); }
SCALAR_FUN_ATTR f16 futrts_gamma16(f16 x) { return tgamma(x); }
SCALAR_FUN_ATTR f16 futrts_lgamma16(f16 x) { return lgamma(x); }
SCALAR_FUN_ATTR f16 futrts_erf16(f16 x) { return erf(x); }
SCALAR_FUN_ATTR f16 futrts_erfc16(f16 x) { return erfc(x); }
SCALAR_FUN_ATTR f16 fmod16(f16 x, f16 y) { return fmod(x, y); }
SCALAR_FUN_ATTR f16 futrts_round16(f16 x) { return rint(x); }
SCALAR_FUN_ATTR f16 futrts_floor16(f16 x) { return floor(x); }
SCALAR_FUN_ATTR f16 futrts_ceil16(f16 x) { return ceil(x); }
SCALAR_FUN_ATTR f16 futrts_nextafter16(f16 x, f16 y) { return nextafter(x, y); }
SCALAR_FUN_ATTR f16 futrts_lerp16(f16 v0, f16 v1, f16 t) { return mix(v0, v1, t); }
SCALAR_FUN_ATTR f16 futrts_ldexp16(f16 x, int32_t y) { return ldexp(x, y); }
SCALAR_FUN_ATTR f16 futrts_copysign16(f16 x, f16 y) { return copysign(x, y); }
SCALAR_FUN_ATTR f16 futrts_mad16(f16 a, f16 b, f16 c) { return mad(a, b, c); }
SCALAR_FUN_ATTR f16 futrts_fma16(f16 a, f16 b, f16 c) { return fma(a, b, c); }

#elif defined(ISPC)

SCALAR_FUN_ATTR f16 futrts_log16(f16 x) { return futrts_isfinite16(x) || (futrts_isinf16(x) && x < 0) ? log(x) : x; }
SCALAR_FUN_ATTR f16 futrts_log2_16(f16 x) { return futrts_log16(x) / log(2.0f16); }
SCALAR_FUN_ATTR f16 futrts_log10_16(f16 x) { return futrts_log16(x) / log(10.0f16); }
SCALAR_FUN_ATTR f16 futrts_log1p_16(f16 x) {
  if(x == -1.0f16 || (futrts_isinf16(x) && x > 0.0f16)) return x / 0.0f16;
  f16 y = 1.0f16 + x;
  f16 z = y - 1.0f16;
  return log(y) - (z-x)/y;
}
SCALAR_FUN_ATTR f16 futrts_sqrt16(f16 x) { return (float16)sqrt((float)x); }
SCALAR_FUN_ATTR f16 futrts_rsqrt16(f16 x) { return (float16)1/sqrt((float)x); }
SCALAR_FUN_ATTR f16 futrts_exp16(f16 x) { return exp(x); }
SCALAR_FUN_ATTR f16 futrts_cos16(f16 x) { return (float16)cos((float)x); }
SCALAR_FUN_ATTR f16 futrts_cospi16(f16 x) { return (float16)cos((float)M_PI*(float)x); }
SCALAR_FUN_ATTR f16 futrts_sin16(f16 x) { return (float16)sin((float)x); }
SCALAR_FUN_ATTR f16 futrts_sinpi16(f16 x) { return (float16)sin((float)M_PI*(float)x); }
SCALAR_FUN_ATTR f16 futrts_tan16(f16 x) { return (float16)tan((float)x); }
SCALAR_FUN_ATTR f16 futrts_tanpi16(f16 x) { return (float16)(tan((float)M_PI*(float)x)); }
SCALAR_FUN_ATTR f16 futrts_acos16(f16 x) { return (float16)acos((float)x); }
SCALAR_FUN_ATTR f16 futrts_acospi16(f16 x) { return (float16)(acos((float)x)/(float)M_PI); }
SCALAR_FUN_ATTR f16 futrts_asin16(f16 x) { return (float16)asin((float)x); }
SCALAR_FUN_ATTR f16 futrts_asinpi16(f16 x) { return (float16)(asin((float)x)/(float)M_PI); }
SCALAR_FUN_ATTR f16 futrts_atan16(f16 x) { return (float16)atan((float)x); }
SCALAR_FUN_ATTR f16 futrts_atanpi16(f16 x) { return (float16)(atan((float)x)/(float)M_PI); }
SCALAR_FUN_ATTR f16 futrts_cosh16(f16 x) { return (exp(x)+exp(-x)) / 2.0f16; }
SCALAR_FUN_ATTR f16 futrts_sinh16(f16 x) { return (exp(x)-exp(-x)) / 2.0f16; }
SCALAR_FUN_ATTR f16 futrts_tanh16(f16 x) { return futrts_sinh16(x)/futrts_cosh16(x); }
SCALAR_FUN_ATTR f16 futrts_acosh16(f16 x) {
  float16 f = x+(float16)sqrt((float)(x*x-1));
  if(futrts_isfinite16(f)) return log(f);
  return f;
}
SCALAR_FUN_ATTR f16 futrts_asinh16(f16 x) {
  float16 f = x+(float16)sqrt((float)(x*x+1));
  if(futrts_isfinite16(f)) return log(f);
  return f;
}
SCALAR_FUN_ATTR f16 futrts_atanh16(f16 x) {
  float16 f = (1+x)/(1-x);
  if(futrts_isfinite16(f)) return log(f)/2.0f16;
  return f;
}
SCALAR_FUN_ATTR f16 futrts_atan2_16(f16 x, f16 y) { return (float16)atan2((float)x, (float)y); }
SCALAR_FUN_ATTR f16 futrts_atan2pi_16(f16 x, f16 y) { return (float16)(atan2((float)x, (float)y)/(float)M_PI); }
SCALAR_FUN_ATTR f16 futrts_hypot16(f16 x, f16 y) { return (float16)futrts_hypot32((float)x, (float)y); }

extern "C" unmasked uniform float tgammaf(uniform float x);
SCALAR_FUN_ATTR f16 futrts_gamma16(f16 x) {
  f16 res;
  foreach_active (i) {
    uniform f16 r = (f16)tgammaf(extract((float)x, i));
    res = insert(res, i, r);
  }
  return res;
}

extern "C" unmasked uniform float lgammaf(uniform float x);
SCALAR_FUN_ATTR f16 futrts_lgamma16(f16 x) {
  f16 res;
  foreach_active (i) {
    uniform f16 r = (f16)lgammaf(extract((float)x, i));
    res = insert(res, i, r);
  }
  return res;
}
SCALAR_FUN_ATTR f16 futrts_cbrt16(f16 x) { return (f16)futrts_cbrt32((float)x); }
SCALAR_FUN_ATTR f16 futrts_erf16(f16 x) { return (f16)futrts_erf32((float)x); }
SCALAR_FUN_ATTR f16 futrts_erfc16(f16 x) { return (f16)futrts_erfc32((float)x); }
SCALAR_FUN_ATTR f16 fmod16(f16 x, f16 y) { return x - y * (float16)trunc((float) (x/y)); }
SCALAR_FUN_ATTR f16 futrts_round16(f16 x) { return (float16)round((float)x); }
SCALAR_FUN_ATTR f16 futrts_floor16(f16 x) { return (float16)floor((float)x); }
SCALAR_FUN_ATTR f16 futrts_ceil16(f16 x) { return (float16)ceil((float)x); }
SCALAR_FUN_ATTR f16 futrts_nextafter16(f16 x, f16 y) { return (float16)futrts_nextafter32((float)x, (float) y); }
SCALAR_FUN_ATTR f16 futrts_lerp16(f16 v0, f16 v1, f16 t) { return v0 + (v1 - v0) * t; }
SCALAR_FUN_ATTR f16 futrts_ldexp16(f16 x, int32_t y) { return futrts_ldexp32((float)x, y); }
SCALAR_FUN_ATTR f16 futrts_copysign16(f16 x, f16 y) { return futrts_copysign32((float)x, y); }
SCALAR_FUN_ATTR f16 futrts_mad16(f16 a, f16 b, f16 c) { return a * b + c; }
SCALAR_FUN_ATTR f16 futrts_fma16(f16 a, f16 b, f16 c) { return a * b + c; }

#else // Assume CUDA.

SCALAR_FUN_ATTR f16 futrts_log16(f16 x) { return hlog(x); }
SCALAR_FUN_ATTR f16 futrts_log2_16(f16 x) { return hlog2(x); }
SCALAR_FUN_ATTR f16 futrts_log10_16(f16 x) { return hlog10(x); }
SCALAR_FUN_ATTR f16 futrts_log1p_16(f16 x) { return (f16)log1pf((float)x); }
SCALAR_FUN_ATTR f16 futrts_sqrt16(f16 x) { return hsqrt(x); }
SCALAR_FUN_ATTR f16 futrts_rsqrt16(f16 x) { return hrsqrt(x); }
SCALAR_FUN_ATTR f16 futrts_cbrt16(f16 x) { return cbrtf(x); }
SCALAR_FUN_ATTR f16 futrts_exp16(f16 x) { return hexp(x); }
SCALAR_FUN_ATTR f16 futrts_cos16(f16 x) { return hcos(x); }
SCALAR_FUN_ATTR f16 futrts_cospi16(f16 x) { return hcos((f16)M_PI*x); }
SCALAR_FUN_ATTR f16 futrts_sin16(f16 x) { return hsin(x); }
SCALAR_FUN_ATTR f16 futrts_sinpi16(f16 x) { return hsin((f16)M_PI*x); }
SCALAR_FUN_ATTR f16 futrts_tan16(f16 x) { return tanf(x); }
SCALAR_FUN_ATTR f16 futrts_tanpi16(f16 x) { return tanf((f16)M_PI*x); }
SCALAR_FUN_ATTR f16 futrts_acos16(f16 x) { return acosf(x); }
SCALAR_FUN_ATTR f16 futrts_acospi16(f16 x) { return (f16)acosf(x)/(f16)M_PI; }
SCALAR_FUN_ATTR f16 futrts_asin16(f16 x) { return asinf(x); }
SCALAR_FUN_ATTR f16 futrts_asinpi16(f16 x) { return (f16)asinf(x)/(f16)M_PI; }
SCALAR_FUN_ATTR f16 futrts_atan16(f16 x) { return (f16)atanf(x); }
SCALAR_FUN_ATTR f16 futrts_atanpi16(f16 x) { return (f16)atanf(x)/(f16)M_PI; }
SCALAR_FUN_ATTR f16 futrts_cosh16(f16 x) { return coshf(x); }
SCALAR_FUN_ATTR f16 futrts_sinh16(f16 x) { return sinhf(x); }
SCALAR_FUN_ATTR f16 futrts_tanh16(f16 x) { return tanhf(x); }
SCALAR_FUN_ATTR f16 futrts_acosh16(f16 x) { return acoshf(x); }
SCALAR_FUN_ATTR f16 futrts_asinh16(f16 x) { return asinhf(x); }
SCALAR_FUN_ATTR f16 futrts_atanh16(f16 x) { return atanhf(x); }
SCALAR_FUN_ATTR f16 futrts_atan2_16(f16 x, f16 y) { return (f16)atan2f(x, y); }
SCALAR_FUN_ATTR f16 futrts_atan2pi_16(f16 x, f16 y) { return (f16)atan2f(x, y)/(f16)M_PI; }
SCALAR_FUN_ATTR f16 futrts_hypot16(f16 x, f16 y) { return hypotf(x, y); }
SCALAR_FUN_ATTR f16 futrts_gamma16(f16 x) { return tgammaf(x); }
SCALAR_FUN_ATTR f16 futrts_lgamma16(f16 x) { return lgammaf(x); }
SCALAR_FUN_ATTR f16 futrts_erf16(f16 x) { return erff(x); }
SCALAR_FUN_ATTR f16 futrts_erfc16(f16 x) { return erfcf(x); }
SCALAR_FUN_ATTR f16 fmod16(f16 x, f16 y) { return fmodf(x, y); }
SCALAR_FUN_ATTR f16 futrts_round16(f16 x) { return rintf(x); }
SCALAR_FUN_ATTR f16 futrts_floor16(f16 x) { return hfloor(x); }
SCALAR_FUN_ATTR f16 futrts_ceil16(f16 x) { return hceil(x); }
SCALAR_FUN_ATTR f16 futrts_nextafter16(f16 x, f16 y) { return __ushort_as_half(halfbitsnextafter(__half_as_ushort(x), __half_as_ushort(y))); }
SCALAR_FUN_ATTR f16 futrts_lerp16(f16 v0, f16 v1, f16 t) { return v0 + (v1 - v0) * t; }
SCALAR_FUN_ATTR f16 futrts_ldexp16(f16 x, int32_t y) { return futrts_ldexp32((float)x, y); }
SCALAR_FUN_ATTR f16 futrts_copysign16(f16 x, f16 y) { return futrts_copysign32((float)x, y); }
SCALAR_FUN_ATTR f16 futrts_mad16(f16 a, f16 b, f16 c) { return a * b + c; }
SCALAR_FUN_ATTR f16 futrts_fma16(f16 a, f16 b, f16 c) { return fmaf(a, b, c); }

#endif

// The CUDA __half type cannot be put in unions for some reason, so we
// use bespoke conversion functions instead.
#ifdef __CUDA_ARCH__
SCALAR_FUN_ATTR int16_t fptobits_f16_i16(f16 x) { return __half_as_ushort(x); }
SCALAR_FUN_ATTR f16 bitstofp_i16_f16(int16_t x) { return __ushort_as_half(x); }
#elif defined(ISPC)
SCALAR_FUN_ATTR int16_t fptobits_f16_i16(f16 x) { varying int16_t y = *((varying int16_t * uniform)&x); return y;
}
SCALAR_FUN_ATTR f16 bitstofp_i16_f16(int16_t x) { varying f16 y = *((varying f16 * uniform)&x); return y; }
#else
SCALAR_FUN_ATTR int16_t fptobits_f16_i16(f16 x) {
  union {
    f16 f;
    int16_t t;
  } p;

  p.f = x;
  return p.t;
}

SCALAR_FUN_ATTR f16 bitstofp_i16_f16(int16_t x) {
  union {
    int16_t f;
    f16 t;
  } p;

  p.f = x;
  return p.t;
}
#endif

#else // No native f16 - emulate.

SCALAR_FUN_ATTR bool cmplt16(f16 x, f16 y) { return x < y; }
SCALAR_FUN_ATTR bool cmple16(f16 x, f16 y) { return x <= y; }
SCALAR_FUN_ATTR f16 fabs16(f16 x) { return fabs32(x); }
SCALAR_FUN_ATTR f16 fmax16(f16 x, f16 y) { return fmax32(x, y); }
SCALAR_FUN_ATTR f16 fmin16(f16 x, f16 y) { return fmin32(x, y); }
SCALAR_FUN_ATTR f16 fpow16(f16 x, f16 y) { return fpow32(x, y); }
SCALAR_FUN_ATTR bool futrts_isnan16(f16 x) { return futrts_isnan32(x); }
SCALAR_FUN_ATTR bool futrts_isinf16(f16 x) { return futrts_isinf32(x); }
SCALAR_FUN_ATTR f16 futrts_log16(f16 x) { return futrts_log32(x); }
SCALAR_FUN_ATTR f16 futrts_log2_16(f16 x) { return futrts_log2_32(x); }
SCALAR_FUN_ATTR f16 futrts_log10_16(f16 x) { return futrts_log10_32(x); }
SCALAR_FUN_ATTR f16 futrts_log1p_16(f16 x) { return futrts_log1p_32(x); }
SCALAR_FUN_ATTR f16 futrts_sqrt16(f16 x) { return futrts_sqrt32(x); }
SCALAR_FUN_ATTR f16 futrts_rsqrt16(f16 x) { return futrts_rsqrt32(x); }
SCALAR_FUN_ATTR f16 futrts_cbrt16(f16 x) { return futrts_cbrt32(x); }
SCALAR_FUN_ATTR f16 futrts_exp16(f16 x) { return futrts_exp32(x); }
SCALAR_FUN_ATTR f16 futrts_cos16(f16 x) { return futrts_cos32(x); }
SCALAR_FUN_ATTR f16 futrts_cospi16(f16 x) { return futrts_cospi32(x); }
SCALAR_FUN_ATTR f16 futrts_sin16(f16 x) { return futrts_sin32(x); }
SCALAR_FUN_ATTR f16 futrts_sinpi16(f16 x) { return futrts_sinpi32(x); }
SCALAR_FUN_ATTR f16 futrts_tan16(f16 x) { return futrts_tan32(x); }
SCALAR_FUN_ATTR f16 futrts_tanpi16(f16 x) { return futrts_tanpi32(x); }
SCALAR_FUN_ATTR f16 futrts_acos16(f16 x) { return futrts_acos32(x); }
SCALAR_FUN_ATTR f16 futrts_acospi16(f16 x) { return futrts_acospi32(x); }
SCALAR_FUN_ATTR f16 futrts_asin16(f16 x) { return futrts_asin32(x); }
SCALAR_FUN_ATTR f16 futrts_asinpi16(f16 x) { return futrts_asinpi32(x); }
SCALAR_FUN_ATTR f16 futrts_atan16(f16 x) { return futrts_atan32(x); }
SCALAR_FUN_ATTR f16 futrts_atanpi16(f16 x) { return futrts_atanpi32(x); }
SCALAR_FUN_ATTR f16 futrts_cosh16(f16 x) { return futrts_cosh32(x); }
SCALAR_FUN_ATTR f16 futrts_sinh16(f16 x) { return futrts_sinh32(x); }
SCALAR_FUN_ATTR f16 futrts_tanh16(f16 x) { return futrts_tanh32(x); }
SCALAR_FUN_ATTR f16 futrts_acosh16(f16 x) { return futrts_acosh32(x); }
SCALAR_FUN_ATTR f16 futrts_asinh16(f16 x) { return futrts_asinh32(x); }
SCALAR_FUN_ATTR f16 futrts_atanh16(f16 x) { return futrts_atanh32(x); }
SCALAR_FUN_ATTR f16 futrts_atan2_16(f16 x, f16 y) { return futrts_atan2_32(x, y); }
SCALAR_FUN_ATTR f16 futrts_atan2pi_16(f16 x, f16 y) { return futrts_atan2pi_32(x, y); }
SCALAR_FUN_ATTR f16 futrts_hypot16(f16 x, f16 y) { return futrts_hypot32(x, y); }
SCALAR_FUN_ATTR f16 futrts_gamma16(f16 x) { return futrts_gamma32(x); }
SCALAR_FUN_ATTR f16 futrts_lgamma16(f16 x) { return futrts_lgamma32(x); }
SCALAR_FUN_ATTR f16 futrts_erf16(f16 x) { return futrts_erf32(x); }
SCALAR_FUN_ATTR f16 futrts_erfc16(f16 x) { return futrts_erfc32(x); }
SCALAR_FUN_ATTR f16 fmod16(f16 x, f16 y) { return fmod32(x, y); }
SCALAR_FUN_ATTR f16 futrts_round16(f16 x) { return futrts_round32(x); }
SCALAR_FUN_ATTR f16 futrts_floor16(f16 x) { return futrts_floor32(x); }
SCALAR_FUN_ATTR f16 futrts_ceil16(f16 x) { return futrts_ceil32(x); }
SCALAR_FUN_ATTR f16 futrts_nextafter16(f16 x, f16 y) { return halfbits2float(halfbitsnextafter(float2halfbits(x), float2halfbits(y))); }
SCALAR_FUN_ATTR f16 futrts_lerp16(f16 v0, f16 v1, f16 t) { return futrts_lerp32(v0, v1, t); }
SCALAR_FUN_ATTR f16 futrts_ldexp16(f16 x, int32_t y) { return futrts_ldexp32(x, y); }
SCALAR_FUN_ATTR f16 futrts_copysign16(f16 x, f16 y) { return futrts_copysign32((float)x, y); }
SCALAR_FUN_ATTR f16 futrts_mad16(f16 a, f16 b, f16 c) { return futrts_mad32(a, b, c); }
SCALAR_FUN_ATTR f16 futrts_fma16(f16 a, f16 b, f16 c) { return futrts_fma32(a, b, c); }

// Even when we are using an OpenCL that does not support cl_khr_fp16,
// it must still support vload_half for actually creating a
// half-precision number, which can then be efficiently converted to a
// float.  Similarly for vstore_half.
#ifdef __OPENCL_VERSION__

SCALAR_FUN_ATTR int16_t fptobits_f16_i16(f16 x) {
  int16_t y;
  // Violating strict aliasing here.
  vstore_half((float)x, 0, (half*)&y);
  return y;
}

SCALAR_FUN_ATTR f16 bitstofp_i16_f16(int16_t x) {
  return (f16)vload_half(0, (half*)&x);
}

#else
SCALAR_FUN_ATTR int16_t fptobits_f16_i16(f16 x) { return (int16_t)float2halfbits(x); }
SCALAR_FUN_ATTR f16 bitstofp_i16_f16(int16_t x) { return halfbits2float((uint16_t)x); }
SCALAR_FUN_ATTR f16 fsignum16(f16 x) { return futrts_isnan16(x) ? x : (x > 0 ? 1 : 0) - (x < 0 ? 1 : 0); }

#endif

#endif

SCALAR_FUN_ATTR float fpconv_f16_f16(f16 x) { return x; }
SCALAR_FUN_ATTR float fpconv_f16_f32(f16 x) { return x; }
SCALAR_FUN_ATTR f16 fpconv_f32_f16(float x) { return (f16) x; }

#ifdef FUTHARK_F64_ENABLED
SCALAR_FUN_ATTR double fpconv_f16_f64(f16 x) { return (double) x; }
#if defined(ISPC)
SCALAR_FUN_ATTR f16 fpconv_f64_f16(double x) { return (f16) ((float)x); }
#else
SCALAR_FUN_ATTR f16 fpconv_f64_f16(double x) { return (f16) x; }
#endif
#endif

// End of scalar_f16.h.

// Start of context_prototypes.h
//
// Prototypes for the functions in context.h, or that will be called
// from those functions, that need to be available very early.

struct futhark_context_config;
struct futhark_context;

struct tuning_param {
  const char *name;
  const char *var; // Z-encoded name.
  const char *class;
  bool set;
  int64_t val;
};

static void set_error(struct futhark_context* ctx, char *error);

// These are called in context/config new/free functions and contain
// shared setup.  They are generated by the compiler itself.
static int init_constants(struct futhark_context*);
static int free_constants(struct futhark_context*);
static void setup_program(struct futhark_context* ctx);
static void teardown_program(struct futhark_context *ctx);

// Allocate host memory.  Must be freed with host_free().
static void host_alloc(struct futhark_context* ctx, size_t size, const char* tag, size_t* size_out, void** mem_out);
// Allocate memory allocated with host_alloc().
static void host_free(struct futhark_context* ctx, size_t size, const char* tag, void* mem);

// Log that a copy has occurred. The provenance may be NULL, if we do not know
// where this came from.
static void log_copy(struct futhark_context* ctx,
                     const char *kind, const char *provenance,
                     int r,
                     int64_t dst_offset, int64_t dst_strides[r],
                     int64_t src_offset, int64_t src_strides[r],
                     int64_t shape[r]);

static void log_transpose(struct futhark_context* ctx,
                          int64_t k, int64_t m, int64_t n);

static bool lmad_map_tr(int64_t *num_arrays_out, int64_t *n_out, int64_t *m_out,
                        int r,
                        const int64_t dst_strides[r],
                        const int64_t src_strides[r],
                        const int64_t shape[r]);

static bool lmad_contiguous(int r, int64_t strides[r], int64_t shape[r]);

static bool lmad_memcpyable(int r,
                            int64_t dst_strides[r], int64_t src_strides[r], int64_t shape[r]);

static void add_event(struct futhark_context* ctx,
                      const char* name,
                      const char* provenance,
                      struct kvs *kvs,
                      void* data,
                      event_report_fn f);

// Functions that must be defined by the backend.
static void backend_context_config_setup(struct futhark_context_config* cfg);
static void backend_context_config_teardown(struct futhark_context_config* cfg);
static int backend_context_setup(struct futhark_context *ctx);
static void backend_context_teardown(struct futhark_context *ctx);

// End of of context_prototypes.h

struct memblock {
    int *references;
    unsigned char *mem;
    int64_t size;
    const char *desc;
};
struct constants {
    int dummy;
};
#define NUM_TUNING_PARAMS 0
static const char *tuning_param_names[] = {NULL};
static const char *tuning_param_vars[] = {NULL};
static const char *tuning_param_classes[] = {NULL};
static int64_t tuning_param_defaults[] = {0};
static const struct {
                 int dummy;
             } tuning_param_indexes = {0};
// Start of backends/c.h

struct futhark_context_config {
  int in_use;
  int debugging;
  int profiling;
  int logging;
  char *cache_fname;
  struct tuning_param tuning_params[NUM_TUNING_PARAMS+1];
};

static void backend_context_config_setup(struct futhark_context_config* cfg) {
  (void)cfg;
}

static void backend_context_config_teardown(struct futhark_context_config* cfg) {
  (void)cfg;
}

int futhark_context_config_set_tuning_param(struct futhark_context_config *cfg,
                                            const char *param_name,
                                            size_t new_value) {
  for (int i = 0; i < NUM_TUNING_PARAMS; i++) {
    if (strcmp(param_name, cfg->tuning_params[i].name) == 0) {
      cfg->tuning_params[i].val = new_value;
      cfg->tuning_params[i].set = true;
      return 0;
    }
  }

  return 1;
}

struct futhark_context {
  struct futhark_context_config* cfg;
  int detail_memory;
  int debugging;
  int profiling;
  int profiling_paused;
  int logging;
  lock_t lock;
  char *error;
  lock_t error_lock;
  FILE *log;
  struct constants *constants;
  struct free_list free_list;
  struct event_list event_list;
  int64_t peak_mem_usage_default;
  int64_t cur_mem_usage_default;
  struct program* program;
  bool program_initialised;
};

int backend_context_setup(struct futhark_context* ctx) {
  (void)ctx;
  return 0;
}

void backend_context_teardown(struct futhark_context* ctx) {
  (void)ctx;
}

int futhark_context_sync(struct futhark_context* ctx) {
  (void)ctx;
  return 0;
}

// End of backends/c.h

struct program {
    int dummy;
};
static void setup_program(struct futhark_context *ctx)
{
    (void) ctx;
    
    int error = 0;
    
    (void) error;
    ctx->program = malloc(sizeof(struct program));
}
static void teardown_program(struct futhark_context *ctx)
{
    (void) ctx;
    
    int error = 0;
    
    (void) error;
    free(ctx->program);
}
int memblock_unref(struct futhark_context *ctx, struct memblock *block, const char *desc)
{
    if (block->references != NULL) {
        *block->references -= 1;
        if (ctx->detail_memory)
            fprintf(ctx->log, "Unreferencing block %s (allocated as %s) in %s: %d references remaining.\n", desc, block->desc, "default space", *block->references);
        if (*block->references == 0) {
            ctx->cur_mem_usage_default -= block->size;
            host_free(ctx, (size_t) block->size, desc, (void *) block->mem);
            free(block->references);
            if (ctx->detail_memory)
                fprintf(ctx->log, "%lld bytes freed (now allocated: %lld bytes)\n", (long long) block->size, (long long) ctx->cur_mem_usage_default);
        }
        block->references = NULL;
    }
    return 0;
}
int memblock_alloc(struct futhark_context *ctx, struct memblock *block, int64_t size, const char *desc)
{
    if (size < 0)
        futhark_panic(1, "Negative allocation of %lld bytes attempted for %s in %s.\n", (long long) size, desc, "default space", ctx->cur_mem_usage_default);
    
    int ret = memblock_unref(ctx, block, desc);
    
    if (ret != FUTHARK_SUCCESS)
        return ret;
    if (ctx->detail_memory)
        fprintf(ctx->log, "Allocating %lld bytes for %s in %s (currently allocated: %lld bytes).\n", (long long) size, desc, "default space", (long long) ctx->cur_mem_usage_default);
    host_alloc(ctx, (size_t) size, desc, (size_t *) &size, (void *) &block->mem);
    if (ctx->error == NULL) {
        block->references = (int *) malloc(sizeof(int));
        *block->references = 1;
        block->size = size;
        block->desc = desc;
        
        long long new_usage = ctx->cur_mem_usage_default + size;
        
        if (ctx->detail_memory)
            fprintf(ctx->log, "Received block of %lld bytes; now allocated: %lld bytes", (long long) block->size, new_usage);
        ctx->cur_mem_usage_default = new_usage;
        if (new_usage > ctx->peak_mem_usage_default) {
            ctx->peak_mem_usage_default = new_usage;
            if (ctx->detail_memory)
                fprintf(ctx->log, " (new peak).\n");
        } else if (ctx->detail_memory)
            fprintf(ctx->log, ".\n");
        return FUTHARK_SUCCESS;
    } else {
        // We are naively assuming that any memory allocation error is due to OOM.
        lock_lock(&ctx->error_lock);
        
        char *old_error = ctx->error;
        
        ctx->error = msgprintf("Failed to allocate memory in %s.\nAttempted allocation: %12lld bytes\nCurrently allocated:  %12lld bytes\n%s", "default space", (long long) size, (long long) ctx->cur_mem_usage_default, old_error);
        free(old_error);
        lock_unlock(&ctx->error_lock);
        return FUTHARK_OUT_OF_MEMORY;
    }
}
int memblock_set(struct futhark_context *ctx, struct memblock *lhs, struct memblock *rhs, const char *lhs_desc)
{
    int ret = memblock_unref(ctx, lhs, lhs_desc);
    
    if (rhs->references != NULL)
        (*rhs->references)++;
    *lhs = *rhs;
    return ret;
}
char *futhark_context_report(struct futhark_context *ctx)
{
    if (futhark_context_sync(ctx) != 0)
        return NULL;
    
    struct str_builder builder;
    
    str_builder_init(&builder);
    str_builder_char(&builder, '{');
    str_builder_str(&builder, "\"memory\":{");
    str_builder(&builder, "\"default space\": %lld", (long long) ctx->peak_mem_usage_default);
    str_builder_str(&builder, "},\"events\":[");
    if (report_events_in_list(&ctx->event_list, &builder) != 0) {
        free(builder.str);
        return NULL;
    } else {
        str_builder_str(&builder, "]}");
        return builder.str;
    }
}
int futhark_context_clear_caches(struct futhark_context *ctx)
{
    lock_lock(&ctx->lock);
    ctx->peak_mem_usage_default = 0;
    lock_unlock(&ctx->lock);
    return ctx->error != NULL;
}

// Start of context.h

// Internal functions.

static void set_error(struct futhark_context* ctx, char *error) {
  lock_lock(&ctx->error_lock);
  if (ctx->error == NULL) {
    ctx->error = error;
  } else {
    free(error);
  }
  lock_unlock(&ctx->error_lock);
}

// XXX: should be static, but used in ispc_util.h
void lexical_realloc_error(struct futhark_context* ctx, size_t new_size) {
  set_error(ctx,
            msgprintf("Failed to allocate memory.\nAttempted allocation: %12lld bytes\n",
                      (long long) new_size));
}

static int lexical_realloc(struct futhark_context *ctx,
                           unsigned char **ptr,
                           int64_t *old_size,
                           int64_t new_size) {
  unsigned char *new = realloc(*ptr, (size_t)new_size);
  if (new == NULL) {
    lexical_realloc_error(ctx, new_size);
    return FUTHARK_OUT_OF_MEMORY;
  } else {
    *ptr = new;
    *old_size = new_size;
    return FUTHARK_SUCCESS;
  }
}

static void free_all_in_free_list(struct futhark_context* ctx) {
  fl_mem mem;
  free_list_pack(&ctx->free_list);
  while (free_list_first(&ctx->free_list, (fl_mem*)&mem) == 0) {
    free((void*)mem);
  }
}

static int is_small_alloc(size_t size) {
  return size < 1024*1024;
}

static void host_alloc(struct futhark_context* ctx,
                       size_t size, const char* tag, size_t* size_out, void** mem_out) {
  if (is_small_alloc(size) || free_list_find(&ctx->free_list, size, tag, size_out, (fl_mem*)mem_out) != 0) {
    *size_out = size;
    *mem_out = malloc(size);
  }
}

static void host_free(struct futhark_context* ctx,
                      size_t size, const char* tag, void* mem) {
  // Small allocations are handled by malloc()s own free list.  The
  // threshold here is kind of arbitrary, but seems to work OK.
  // Larger allocations are mmap()ed/munmapped() every time, which is
  // very slow, and Futhark programs tend to use a few very large
  // allocations.
  if (is_small_alloc(size)) {
    free(mem);
  } else {
    free_list_insert(&ctx->free_list, size, (fl_mem)mem, tag);
  }
}

static void add_event(struct futhark_context* ctx,
                      const char* name,
                      const char* provenance,
                      struct kvs *kvs,
                      void* data,
                      event_report_fn f) {
  if (provenance == NULL) {
    provenance = "unknown";
  }
  if (ctx->logging) {
    fprintf(ctx->log, "Event: %s\n  at: %s\n", name, provenance);
    if (kvs) {
      kvs_log(kvs, "  ", ctx->log);
    }
  }
  add_event_to_list(&ctx->event_list, name, provenance, kvs, data, f);
}

char *futhark_context_get_error(struct futhark_context *ctx) {
  char *error = ctx->error;
  ctx->error = NULL;
  return error;
}

void futhark_context_config_set_debugging(struct futhark_context_config *cfg, int flag) {
    cfg->profiling = cfg->logging = cfg->debugging = flag;
}

void futhark_context_config_set_profiling(struct futhark_context_config *cfg, int flag) {
    cfg->profiling = flag;
}

void futhark_context_config_set_logging(struct futhark_context_config *cfg, int flag) {
    cfg->logging = flag;
}

void futhark_context_config_set_cache_file(struct futhark_context_config *cfg, const char *f) {
  cfg->cache_fname = strdup(f);
}

int futhark_get_tuning_param_count(void) {
  return NUM_TUNING_PARAMS;
}

const char *futhark_get_tuning_param_name(int i) {
  return tuning_param_names[i];
}

const char *futhark_get_tuning_param_class(int i) {
    return tuning_param_classes[i];
}

void futhark_context_set_logging_file(struct futhark_context *ctx, FILE *f){
  ctx->log = f;
}

void futhark_context_pause_profiling(struct futhark_context *ctx) {
  ctx->profiling_paused = 1;
}

void futhark_context_unpause_profiling(struct futhark_context *ctx) {
  ctx->profiling_paused = 0;
}

struct futhark_context_config* futhark_context_config_new(void) {
  struct futhark_context_config* cfg = malloc(sizeof(struct futhark_context_config));
  if (cfg == NULL) {
    return NULL;
  }
  cfg->in_use = 0;
  cfg->debugging = 0;
  cfg->profiling = 0;
  cfg->logging = 0;
  cfg->cache_fname = NULL;
  for (int i = 0; i < NUM_TUNING_PARAMS; i++) {
    cfg->tuning_params[i].set = false;
    cfg->tuning_params[i].val = tuning_param_defaults[i];
    cfg->tuning_params[i].name = tuning_param_names[i];
    cfg->tuning_params[i].var = tuning_param_vars[i];
    cfg->tuning_params[i].class = tuning_param_classes[i];
  }
  backend_context_config_setup(cfg);
  return cfg;
}

void futhark_context_config_free(struct futhark_context_config* cfg) {
  assert(!cfg->in_use);
  backend_context_config_teardown(cfg);
  free(cfg->cache_fname);
  free(cfg);
}

struct futhark_context* futhark_context_new(struct futhark_context_config* cfg) {
  struct futhark_context* ctx = malloc(sizeof(struct futhark_context));
  if (ctx == NULL) {
    return NULL;
  }
  assert(!cfg->in_use);
  ctx->cfg = cfg;
  ctx->cfg->in_use = 1;
  ctx->program_initialised = false;
  create_lock(&ctx->error_lock);
  create_lock(&ctx->lock);
  free_list_init(&ctx->free_list);
  event_list_init(&ctx->event_list);
  ctx->peak_mem_usage_default = 0;
  ctx->cur_mem_usage_default = 0;
  ctx->constants = malloc(sizeof(struct constants));
  ctx->debugging = cfg->debugging;
  ctx->logging = cfg->logging;
  ctx->detail_memory = cfg->logging;
  ctx->profiling = cfg->profiling;
  ctx->profiling_paused = 0;
  ctx->error = NULL;
  ctx->log = stderr;
  if (backend_context_setup(ctx) == 0) {
    setup_program(ctx);
    init_constants(ctx);
    ctx->program_initialised = true;
    (void)futhark_context_clear_caches(ctx);
    (void)futhark_context_sync(ctx);
  }
  return ctx;
}

void futhark_context_free(struct futhark_context* ctx) {
  if (ctx->program_initialised) {
    free_constants(ctx);
    teardown_program(ctx);
  }
  backend_context_teardown(ctx);
  free_all_in_free_list(ctx);
  free_list_destroy(&ctx->free_list);
  event_list_free(&ctx->event_list);
  free(ctx->constants);
  free(ctx->error);
  free_lock(&ctx->lock);
  free_lock(&ctx->error_lock);
  ctx->cfg->in_use = 0;
  free(ctx);
}

// End of context.h

// Start of copy.h

// Cache-oblivious map-transpose function.
#define GEN_MAP_TRANSPOSE(NAME, ELEM_TYPE)                              \
  static void map_transpose_##NAME                                      \
  (ELEM_TYPE* dst, ELEM_TYPE* src,                                      \
   int64_t k, int64_t m, int64_t n,                                     \
   int64_t cb, int64_t ce, int64_t rb, int64_t re)                      \
  {                                                                     \
  int32_t r = re - rb;                                                  \
  int32_t c = ce - cb;                                                  \
  if (k == 1) {                                                         \
    if (r <= 64 && c <= 64) {                                           \
      for (int64_t j = 0; j < c; j++) {                                 \
        for (int64_t i = 0; i < r; i++) {                               \
          dst[(j + cb) * n + (i + rb)] = src[(i + rb) * m + (j + cb)];  \
        }                                                               \
      }                                                                 \
    } else if (c <= r) {                                                \
      map_transpose_##NAME(dst, src, k, m, n, cb, ce, rb, rb + r/2);    \
      map_transpose_##NAME(dst, src, k, m, n, cb, ce, rb + r/2, re);    \
    } else {                                                            \
      map_transpose_##NAME(dst, src, k, m, n, cb, cb + c/2, rb, re);    \
      map_transpose_##NAME(dst, src, k, m, n, cb + c/2, ce, rb, re);    \
    }                                                                   \
  } else {                                                              \
  for (int64_t i = 0; i < k; i++) {                                     \
    map_transpose_##NAME(dst + i * m * n, src + i * m * n, 1, m, n, cb, ce, rb, re); \
  }\
} \
}

// Straightforward LMAD copy function.
#define GEN_LMAD_COPY_ELEMENTS(NAME, ELEM_TYPE)                         \
  static void lmad_copy_elements_##NAME(int r,                          \
                                        ELEM_TYPE* dst, int64_t dst_strides[r], \
                                        ELEM_TYPE *src, int64_t src_strides[r], \
                                        int64_t shape[r]) {             \
    if (r == 1) {                                                       \
      for (int i = 0; i < shape[0]; i++) {                              \
        dst[i*dst_strides[0]] = src[i*src_strides[0]];                  \
      }                                                                 \
    } else if (r > 1) {                                                 \
      for (int i = 0; i < shape[0]; i++) {                              \
        lmad_copy_elements_##NAME(r-1,                                  \
                                  dst+i*dst_strides[0], dst_strides+1,  \
                                  src+i*src_strides[0], src_strides+1,  \
                                  shape+1);                             \
      }                                                                 \
    }                                                                   \
  }                                                                     \

// Check whether this LMAD can be seen as a transposed 2D array.  This
// is done by checking every possible splitting point.
static bool lmad_is_tr(int64_t *n_out, int64_t *m_out,
                       int r,
                       const int64_t strides[r],
                       const int64_t shape[r]) {
  for (int i = 1; i < r; i++) {
    int n = 1, m = 1;
    bool ok = true;
    int64_t expected = 1;
    // Check strides before 'i'.
    for (int j = i-1; j >= 0; j--) {
      ok = ok && strides[j] == expected;
      expected *= shape[j];
      n *= shape[j];
    }
    // Check strides after 'i'.
    for (int j = r-1; j >= i; j--) {
      ok = ok && strides[j] == expected;
      expected *= shape[j];
      m *= shape[j];
    }
    if (ok) {
      *n_out = n;
      *m_out = m;
      return true;
    }
  }
  return false;
}

// This function determines whether the a 'dst' LMAD is row-major and
// 'src' LMAD is column-major.  Both LMADs are for arrays of the same
// shape.  Both LMADs are allowed to have additional dimensions "on
// top".  Essentially, this function determines whether a copy from
// 'src' to 'dst' is a "map(transpose)" that we know how to implement
// efficiently.  The LMADs can have arbitrary rank, and the main
// challenge here is checking whether the src LMAD actually
// corresponds to a 2D column-major layout by morally collapsing
// dimensions.  There is a lot of looping here, but the actual trip
// count is going to be very low in practice.
//
// Returns true if this is indeed a map(transpose), and writes the
// number of arrays, and moral array size to appropriate output
// parameters.
static bool lmad_map_tr(int64_t *num_arrays_out, int64_t *n_out, int64_t *m_out,
                        int r,
                        const int64_t dst_strides[r],
                        const int64_t src_strides[r],
                        const int64_t shape[r]) {
  int64_t rowmajor_strides[r];
  rowmajor_strides[r-1] = 1;

  for (int i = r-2; i >= 0; i--) {
    rowmajor_strides[i] = rowmajor_strides[i+1] * shape[i+1];
  }

  // map_r will be the number of mapped dimensions on top.
  int map_r = 0;
  int64_t num_arrays = 1;
  for (int i = 0; i < r; i++) {
    if (dst_strides[i] != rowmajor_strides[i] ||
        src_strides[i] != rowmajor_strides[i]) {
      break;
    } else {
      num_arrays *= shape[i];
      map_r++;
    }
  }

  *num_arrays_out = num_arrays;

  if (r==map_r) {
    return false;
  }

  if (memcmp(&rowmajor_strides[map_r],
             &dst_strides[map_r],
             sizeof(int64_t)*(r-map_r)) == 0) {
    return lmad_is_tr(n_out, m_out, r-map_r, src_strides+map_r, shape+map_r);
  } else if (memcmp(&rowmajor_strides[map_r],
                    &src_strides[map_r],
                    sizeof(int64_t)*(r-map_r)) == 0) {
    return lmad_is_tr(m_out, n_out, r-map_r, dst_strides+map_r, shape+map_r);
  }
  return false;
}

// Check if the strides correspond to row-major strides of *any*
// permutation of the shape.  This is done by recursive search with
// backtracking.  This is worst-case exponential, but hopefully the
// arrays we encounter do not have that many dimensions.
static bool lmad_contiguous_search(int checked, int64_t expected,
                                   int r,
                                   int64_t strides[r], int64_t shape[r], bool used[r]) {
  for (int i = 0; i < r; i++) {
    for (int j = 0; j < r; j++) {
      if (!used[j] && strides[j] == expected && strides[j] >= 0) {
        used[j] = true;
        if (checked+1 == r ||
            lmad_contiguous_search(checked+1, expected * shape[j], r, strides, shape, used)) {
          return true;
        }
        used[j] = false;
      }
    }
  }
  return false;
}

// Does this LMAD correspond to an array with positive strides and no
// holes?
static bool lmad_contiguous(int r, int64_t strides[r], int64_t shape[r]) {
  bool used[r];
  for (int i = 0; i < r; i++) {
    used[i] = false;
  }
  return lmad_contiguous_search(0, 1, r, strides, shape, used);
}

// Does this copy correspond to something that could be done with a
// memcpy()-like operation?  I.e. do the LMADs actually represent the
// same in-memory layout and are they contiguous?
static bool lmad_memcpyable(int r,
                            int64_t dst_strides[r], int64_t src_strides[r], int64_t shape[r]) {
  if (!lmad_contiguous(r, dst_strides, shape)) {
    return false;
  }
  for (int i = 0; i < r; i++) {
    if (dst_strides[i] != src_strides[i] && shape[i] != 1) {
      return false;
    }
  }
  return true;
}


static void log_copy(struct futhark_context* ctx,
                     const char *kind, const char *provenance,
                     int r,
                     int64_t dst_offset, int64_t dst_strides[r],
                     int64_t src_offset, int64_t src_strides[r],
                     int64_t shape[r]) {
  if (ctx->logging) {
    fprintf(ctx->log, "\n# Copy %s\n", kind);
    if (provenance) { fprintf(ctx->log, "At: %s\n", provenance); }
    fprintf(ctx->log, "Shape: ");
    for (int i = 0; i < r; i++) { fprintf(ctx->log, "[%ld]", (long int)shape[i]); }
    fprintf(ctx->log, "\n");
    fprintf(ctx->log, "Dst offset: %ld\n", (long int)dst_offset);
    fprintf(ctx->log, "Dst strides:");
    for (int i = 0; i < r; i++) { fprintf(ctx->log, " %ld", (long int)dst_strides[i]); }
    fprintf(ctx->log, "\n");
    fprintf(ctx->log, "Src offset: %ld\n", (long int)src_offset);
    fprintf(ctx->log, "Src strides:");
    for (int i = 0; i < r; i++) { fprintf(ctx->log, " %ld", (long int)src_strides[i]); }
    fprintf(ctx->log, "\n");
  }
}

static void log_transpose(struct futhark_context* ctx,
                          int64_t k, int64_t n, int64_t m) {
  if (ctx->logging) {
    fprintf(ctx->log, "## Transpose\n");
    fprintf(ctx->log, "Arrays     : %ld\n", (long int)k);
    fprintf(ctx->log, "X elements : %ld\n", (long int)m);
    fprintf(ctx->log, "Y elements : %ld\n", (long int)n);
    fprintf(ctx->log, "\n");
  }
}

#define GEN_LMAD_COPY(NAME, ELEM_TYPE)                                  \
  static void lmad_copy_##NAME                                          \
  (struct futhark_context *ctx, int r,                                  \
   ELEM_TYPE* dst, int64_t dst_offset, int64_t dst_strides[r],          \
   ELEM_TYPE *src, int64_t src_offset, int64_t src_strides[r],          \
   int64_t shape[r]) {                                                  \
    log_copy(ctx, "CPU to CPU", NULL, r, dst_offset, dst_strides,       \
             src_offset, src_strides, shape);                           \
    int64_t size = 1;                                                   \
    for (int i = 0; i < r; i++) { size *= shape[i]; }                   \
    if (size == 0) { return; }                                          \
    int64_t k, n, m;                                                    \
    if (lmad_map_tr(&k, &n, &m,                                         \
                    r, dst_strides, src_strides, shape)) {              \
      log_transpose(ctx, k, n, m);                                      \
      map_transpose_##NAME                                              \
        (dst+dst_offset, src+src_offset, k, n, m, 0, n, 0, m);          \
    } else if (lmad_memcpyable(r, dst_strides, src_strides, shape)) {   \
      if (ctx->logging) {fprintf(ctx->log, "## Flat copy\n\n");}          \
      memcpy(dst+dst_offset, src+src_offset, size*sizeof(*dst));        \
    } else {                                                            \
      if (ctx->logging) {fprintf(ctx->log, "## General copy\n\n");}       \
      lmad_copy_elements_##NAME                                         \
        (r,                                                             \
         dst+dst_offset, dst_strides,                                   \
         src+src_offset, src_strides, shape);                           \
    }                                                                   \
  }

GEN_MAP_TRANSPOSE(1b, uint8_t)
GEN_MAP_TRANSPOSE(2b, uint16_t)
GEN_MAP_TRANSPOSE(4b, uint32_t)
GEN_MAP_TRANSPOSE(8b, uint64_t)

GEN_LMAD_COPY_ELEMENTS(1b, uint8_t)
GEN_LMAD_COPY_ELEMENTS(2b, uint16_t)
GEN_LMAD_COPY_ELEMENTS(4b, uint32_t)
GEN_LMAD_COPY_ELEMENTS(8b, uint64_t)

GEN_LMAD_COPY(1b, uint8_t)
GEN_LMAD_COPY(2b, uint16_t)
GEN_LMAD_COPY(4b, uint32_t)
GEN_LMAD_COPY(8b, uint64_t)

// End of copy.h

#define FUTHARK_FUN_ATTR static

FUTHARK_FUN_ATTR int futrts_add_abs_11626(struct futhark_context *ctx, struct memblock *mem_out_p_22939, int64_t *out_prim_out_22940, struct memblock a_mem_22579, struct memblock b_mem_22580, int64_t dz2080U_14869, int64_t dz2081U_14870);
FUTHARK_FUN_ATTR int futrts_empty_8082(struct futhark_context *ctx, struct memblock *mem_out_p_22941, struct memblock *mem_out_p_22942, struct memblock *mem_out_p_22943, int64_t *out_prim_out_22944, int64_t capacity_17577);
FUTHARK_FUN_ATTR int futrts_entry_arithmetic(struct futhark_context *ctx, struct memblock *mem_out_p_22945, struct memblock *mem_out_p_22946, struct memblock *mem_out_p_22947, struct memblock *mem_out_p_22948, int64_t *out_prim_out_22949, int64_t *out_prim_out_22950, int64_t *out_prim_out_22951, int64_t *out_prim_out_22952, bool *out_prim_out_22953, bool *out_prim_out_22954, bool *out_prim_out_22955, bool *out_prim_out_22956, int64_t *out_prim_out_22957, struct memblock a_mem_22579, struct memblock b_mem_22580, int64_t dz2080U_15866, int64_t dz2081U_15867, bool negative_a_15868, bool negative_b_15870, int32_t factor_15872);
FUTHARK_FUN_ATTR int futrts_entry_hashmap_increment(struct futhark_context *ctx, struct memblock *mem_out_p_22958, struct memblock keys_mem_22579, struct memblock deltas_mem_22580, int64_t dz2080U_18149, int64_t dz2081U_18150);
FUTHARK_FUN_ATTR int futrts_entry_hashmap_reserved(struct futhark_context *ctx, struct memblock *mem_out_p_22959, struct memblock keys_mem_22579, struct memblock deltas_mem_22580, int64_t dz2080U_18411, int64_t dz2081U_18412, int64_t batch_18415);
FUTHARK_FUN_ATTR int futrts_entry_hashmap_test(struct futhark_context *ctx, struct memblock *mem_out_p_22960, struct memblock keys_mem_22579, int64_t dz2080U_17971);
FUTHARK_FUN_ATTR int futrts_entry_regex_test(struct futhark_context *ctx, struct memblock *mem_out_p_22961, int64_t *out_prim_out_22962, int64_t *out_prim_out_22963, struct memblock pattern_mem_22579, struct memblock text_mem_22580, struct memblock replacement_mem_22581, int64_t dz2080U_17516, int64_t dz2081U_17517, int64_t dz2082U_17518);
FUTHARK_FUN_ATTR int futrts_entry_wrap_test(struct futhark_context *ctx, struct memblock *mem_out_p_22965, int64_t *out_prim_out_22966, struct memblock text_mem_22579, int64_t dz2080U_18542, int64_t width_18543);
FUTHARK_FUN_ATTR int futrts_find_11687(struct futhark_context *ctx, int64_t *out_prim_out_22967, int64_t *out_prim_out_22968, struct memblock z30U_mem_22579, struct memblock z31U_mem_22580, struct memblock z32U_mem_22581, struct memblock z33U_mem_22582, struct memblock text_mem_22583, int64_t dz2080U_16672, int64_t dz2081U_16673, int64_t dz2082U_16674, int64_t dz2083U_16675, int64_t dz2080U_16676, int64_t start_16682);
FUTHARK_FUN_ATTR int futrts_grow_11710(struct futhark_context *ctx, struct memblock *mem_out_p_22969, struct memblock *mem_out_p_22970, struct memblock *mem_out_p_22971, int64_t *out_prim_out_22972, int64_t *out_prim_out_22973, struct memblock z30U_mem_22579, struct memblock z30U_mem_22580, struct memblock z30U_mem_22581, int64_t dz2080U_17759, int64_t z31U_17763);
FUTHARK_FUN_ATTR int futrts_integer_6855(struct futhark_context *ctx, struct memblock *mem_out_p_22974, int64_t *out_prim_out_22975, bool *out_prim_out_22976, int64_t x_15288);
FUTHARK_FUN_ATTR int futrts_literal_11671(struct futhark_context *ctx, int8_t *out_prim_out_22977, int64_t *out_prim_out_22978, struct memblock pattern_mem_22579, int64_t dz2080U_15985, int64_t p_15987);
FUTHARK_FUN_ATTR int futrts_mask_set_8529(struct futhark_context *ctx, struct memblock *mem_out_p_22979, struct memblock mask_mem_22579, int8_t c_16038);
FUTHARK_FUN_ATTR int futrts_match_branch_11689(struct futhark_context *ctx, int64_t *out_prim_out_22980, struct memblock z30U_mem_22579, struct memblock z31U_mem_22580, struct memblock z32U_mem_22581, struct memblock z33U_mem_22582, struct memblock text_mem_22583, int64_t dz2080U_16580, int64_t dz2081U_16581, int64_t dz2082U_16582, int64_t dz2083U_16583, int64_t dz2080U_16584, int64_t token_16589, int64_t last_16590, int64_t position_16592);
FUTHARK_FUN_ATTR int futrts_mul_abs_11639(struct futhark_context *ctx, struct memblock *mem_out_p_22981, int64_t *out_prim_out_22982, struct memblock a_mem_22579, int64_t dz2080U_15143, int32_t b_15145);
FUTHARK_FUN_ATTR int futrts_normalise_11622(struct futhark_context *ctx, struct memblock *mem_out_p_22983, int64_t *out_prim_out_22984, bool *out_prim_out_22985, struct memblock a_mem_22579, int64_t dz2080U_14751, bool negative_14752);
FUTHARK_FUN_ATTR int futrts_put_raw_11712(struct futhark_context *ctx, struct memblock *mem_out_p_22986, struct memblock *mem_out_p_22987, struct memblock *mem_out_p_22988, int64_t *out_prim_out_22989, struct memblock z30U_mem_22579, struct memblock z30U_mem_22580, struct memblock z30U_mem_22581, int64_t dz2080U_17722, int64_t z31U_17726, int64_t key_17727, int64_t value_17728);

static int init_constants(struct futhark_context *ctx)
{
    (void) ctx;
    
    int err = 0;
    
    
  cleanup:
    return err;
}
static int free_constants(struct futhark_context *ctx)
{
    (void) ctx;
    return 0;
}
struct futhark_u8_1d {
    struct memblock mem;
    int64_t shape[1];
};
struct futhark_u8_1d *futhark_new_u8_1d(struct futhark_context *ctx, const uint8_t *data, int64_t dim0)
{
    int err = 0;
    struct futhark_u8_1d *bad = NULL;
    struct futhark_u8_1d *arr = (struct futhark_u8_1d *) malloc(sizeof(struct futhark_u8_1d));
    
    if (arr == NULL)
        return bad;
    lock_lock(&ctx->lock);
    arr->mem.references = NULL;
    arr->shape[0] = dim0;
    if (memblock_alloc(ctx, &arr->mem, arr->shape[0] * 1, "arr->mem"))
        err = 1;
    if ((size_t) dim0 * 1 > 0)
        memmove(arr->mem.mem + 0, (const unsigned char *) data + 0, (size_t) dim0 * 1);
    lock_unlock(&ctx->lock);
    if (err != 0) {
        free(arr);
        return bad;
    }
    return arr;
}
struct futhark_u8_1d *futhark_new_raw_u8_1d(struct futhark_context *ctx, unsigned char *data, int64_t dim0)
{
    int err = 0;
    struct futhark_u8_1d *bad = NULL;
    struct futhark_u8_1d *arr = (struct futhark_u8_1d *) malloc(sizeof(struct futhark_u8_1d));
    
    if (arr == NULL)
        return bad;
    lock_lock(&ctx->lock);
    arr->mem.references = NULL;
    arr->mem.mem = data;
    arr->shape[0] = dim0;
    lock_unlock(&ctx->lock);
    return arr;
}
int futhark_free_u8_1d(struct futhark_context *ctx, struct futhark_u8_1d *arr)
{
    lock_lock(&ctx->lock);
    if (memblock_unref(ctx, &arr->mem, "arr->mem") != 0)
        return 1;
    lock_unlock(&ctx->lock);
    free(arr);
    return 0;
}
int futhark_values_u8_1d(struct futhark_context *ctx, struct futhark_u8_1d *arr, uint8_t *data)
{
    int err = 0;
    
    lock_lock(&ctx->lock);
    if ((size_t) arr->shape[0] * 1 > 0)
        memmove((unsigned char *) data + 0, arr->mem.mem + 0, (size_t) arr->shape[0] * 1);
    lock_unlock(&ctx->lock);
    return err;
}
int futhark_index_u8_1d(struct futhark_context *ctx, uint8_t *out, struct futhark_u8_1d *arr, int64_t i0)
{
    int err = 0;
    
    if (i0 >= 0 && i0 < arr->shape[0]) {
        lock_lock(&ctx->lock);
        if (1 > 0)
            memmove((unsigned char *) out + 0, arr->mem.mem + 1 * (i0 * 1), 1);
        lock_unlock(&ctx->lock);
    } else {
        err = 1;
        set_error(ctx, strdup("Index out of bounds."));
    }
    return err;
}
unsigned char *futhark_values_raw_u8_1d(struct futhark_context *ctx, struct futhark_u8_1d *arr)
{
    (void) ctx;
    return arr->mem.mem;
}
const int64_t *futhark_shape_u8_1d(struct futhark_context *ctx, struct futhark_u8_1d *arr)
{
    (void) ctx;
    return arr->shape;
}
struct futhark_u32_1d {
    struct memblock mem;
    int64_t shape[1];
};
struct futhark_u32_1d *futhark_new_u32_1d(struct futhark_context *ctx, const uint32_t *data, int64_t dim0)
{
    int err = 0;
    struct futhark_u32_1d *bad = NULL;
    struct futhark_u32_1d *arr = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d));
    
    if (arr == NULL)
        return bad;
    lock_lock(&ctx->lock);
    arr->mem.references = NULL;
    arr->shape[0] = dim0;
    if (memblock_alloc(ctx, &arr->mem, arr->shape[0] * 4, "arr->mem"))
        err = 1;
    if ((size_t) dim0 * 4 > 0)
        memmove(arr->mem.mem + 0, (const unsigned char *) data + 0, (size_t) dim0 * 4);
    lock_unlock(&ctx->lock);
    if (err != 0) {
        free(arr);
        return bad;
    }
    return arr;
}
struct futhark_u32_1d *futhark_new_raw_u32_1d(struct futhark_context *ctx, unsigned char *data, int64_t dim0)
{
    int err = 0;
    struct futhark_u32_1d *bad = NULL;
    struct futhark_u32_1d *arr = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d));
    
    if (arr == NULL)
        return bad;
    lock_lock(&ctx->lock);
    arr->mem.references = NULL;
    arr->mem.mem = data;
    arr->shape[0] = dim0;
    lock_unlock(&ctx->lock);
    return arr;
}
int futhark_free_u32_1d(struct futhark_context *ctx, struct futhark_u32_1d *arr)
{
    lock_lock(&ctx->lock);
    if (memblock_unref(ctx, &arr->mem, "arr->mem") != 0)
        return 1;
    lock_unlock(&ctx->lock);
    free(arr);
    return 0;
}
int futhark_values_u32_1d(struct futhark_context *ctx, struct futhark_u32_1d *arr, uint32_t *data)
{
    int err = 0;
    
    lock_lock(&ctx->lock);
    if ((size_t) arr->shape[0] * 4 > 0)
        memmove((unsigned char *) data + 0, arr->mem.mem + 0, (size_t) arr->shape[0] * 4);
    lock_unlock(&ctx->lock);
    return err;
}
int futhark_index_u32_1d(struct futhark_context *ctx, uint32_t *out, struct futhark_u32_1d *arr, int64_t i0)
{
    int err = 0;
    
    if (i0 >= 0 && i0 < arr->shape[0]) {
        lock_lock(&ctx->lock);
        if (4 > 0)
            memmove((unsigned char *) out + 0, arr->mem.mem + 4 * (i0 * 1), 4);
        lock_unlock(&ctx->lock);
    } else {
        err = 1;
        set_error(ctx, strdup("Index out of bounds."));
    }
    return err;
}
unsigned char *futhark_values_raw_u32_1d(struct futhark_context *ctx, struct futhark_u32_1d *arr)
{
    (void) ctx;
    return arr->mem.mem;
}
const int64_t *futhark_shape_u32_1d(struct futhark_context *ctx, struct futhark_u32_1d *arr)
{
    (void) ctx;
    return arr->shape;
}
struct futhark_u64_1d {
    struct memblock mem;
    int64_t shape[1];
};
struct futhark_u64_1d *futhark_new_u64_1d(struct futhark_context *ctx, const uint64_t *data, int64_t dim0)
{
    int err = 0;
    struct futhark_u64_1d *bad = NULL;
    struct futhark_u64_1d *arr = (struct futhark_u64_1d *) malloc(sizeof(struct futhark_u64_1d));
    
    if (arr == NULL)
        return bad;
    lock_lock(&ctx->lock);
    arr->mem.references = NULL;
    arr->shape[0] = dim0;
    if (memblock_alloc(ctx, &arr->mem, arr->shape[0] * 8, "arr->mem"))
        err = 1;
    if ((size_t) dim0 * 8 > 0)
        memmove(arr->mem.mem + 0, (const unsigned char *) data + 0, (size_t) dim0 * 8);
    lock_unlock(&ctx->lock);
    if (err != 0) {
        free(arr);
        return bad;
    }
    return arr;
}
struct futhark_u64_1d *futhark_new_raw_u64_1d(struct futhark_context *ctx, unsigned char *data, int64_t dim0)
{
    int err = 0;
    struct futhark_u64_1d *bad = NULL;
    struct futhark_u64_1d *arr = (struct futhark_u64_1d *) malloc(sizeof(struct futhark_u64_1d));
    
    if (arr == NULL)
        return bad;
    lock_lock(&ctx->lock);
    arr->mem.references = NULL;
    arr->mem.mem = data;
    arr->shape[0] = dim0;
    lock_unlock(&ctx->lock);
    return arr;
}
int futhark_free_u64_1d(struct futhark_context *ctx, struct futhark_u64_1d *arr)
{
    lock_lock(&ctx->lock);
    if (memblock_unref(ctx, &arr->mem, "arr->mem") != 0)
        return 1;
    lock_unlock(&ctx->lock);
    free(arr);
    return 0;
}
int futhark_values_u64_1d(struct futhark_context *ctx, struct futhark_u64_1d *arr, uint64_t *data)
{
    int err = 0;
    
    lock_lock(&ctx->lock);
    if ((size_t) arr->shape[0] * 8 > 0)
        memmove((unsigned char *) data + 0, arr->mem.mem + 0, (size_t) arr->shape[0] * 8);
    lock_unlock(&ctx->lock);
    return err;
}
int futhark_index_u64_1d(struct futhark_context *ctx, uint64_t *out, struct futhark_u64_1d *arr, int64_t i0)
{
    int err = 0;
    
    if (i0 >= 0 && i0 < arr->shape[0]) {
        lock_lock(&ctx->lock);
        if (8 > 0)
            memmove((unsigned char *) out + 0, arr->mem.mem + 8 * (i0 * 1), 8);
        lock_unlock(&ctx->lock);
    } else {
        err = 1;
        set_error(ctx, strdup("Index out of bounds."));
    }
    return err;
}
unsigned char *futhark_values_raw_u64_1d(struct futhark_context *ctx, struct futhark_u64_1d *arr)
{
    (void) ctx;
    return arr->mem.mem;
}
const int64_t *futhark_shape_u64_1d(struct futhark_context *ctx, struct futhark_u64_1d *arr)
{
    (void) ctx;
    return arr->shape;
}
struct futhark_i64_1d {
    struct memblock mem;
    int64_t shape[1];
};
struct futhark_i64_1d *futhark_new_i64_1d(struct futhark_context *ctx, const int64_t *data, int64_t dim0)
{
    int err = 0;
    struct futhark_i64_1d *bad = NULL;
    struct futhark_i64_1d *arr = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d));
    
    if (arr == NULL)
        return bad;
    lock_lock(&ctx->lock);
    arr->mem.references = NULL;
    arr->shape[0] = dim0;
    if (memblock_alloc(ctx, &arr->mem, arr->shape[0] * 8, "arr->mem"))
        err = 1;
    if ((size_t) dim0 * 8 > 0)
        memmove(arr->mem.mem + 0, (const unsigned char *) data + 0, (size_t) dim0 * 8);
    lock_unlock(&ctx->lock);
    if (err != 0) {
        free(arr);
        return bad;
    }
    return arr;
}
struct futhark_i64_1d *futhark_new_raw_i64_1d(struct futhark_context *ctx, unsigned char *data, int64_t dim0)
{
    int err = 0;
    struct futhark_i64_1d *bad = NULL;
    struct futhark_i64_1d *arr = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d));
    
    if (arr == NULL)
        return bad;
    lock_lock(&ctx->lock);
    arr->mem.references = NULL;
    arr->mem.mem = data;
    arr->shape[0] = dim0;
    lock_unlock(&ctx->lock);
    return arr;
}
int futhark_free_i64_1d(struct futhark_context *ctx, struct futhark_i64_1d *arr)
{
    lock_lock(&ctx->lock);
    if (memblock_unref(ctx, &arr->mem, "arr->mem") != 0)
        return 1;
    lock_unlock(&ctx->lock);
    free(arr);
    return 0;
}
int futhark_values_i64_1d(struct futhark_context *ctx, struct futhark_i64_1d *arr, int64_t *data)
{
    int err = 0;
    
    lock_lock(&ctx->lock);
    if ((size_t) arr->shape[0] * 8 > 0)
        memmove((unsigned char *) data + 0, arr->mem.mem + 0, (size_t) arr->shape[0] * 8);
    lock_unlock(&ctx->lock);
    return err;
}
int futhark_index_i64_1d(struct futhark_context *ctx, int64_t *out, struct futhark_i64_1d *arr, int64_t i0)
{
    int err = 0;
    
    if (i0 >= 0 && i0 < arr->shape[0]) {
        lock_lock(&ctx->lock);
        if (8 > 0)
            memmove((unsigned char *) out + 0, arr->mem.mem + 8 * (i0 * 1), 8);
        lock_unlock(&ctx->lock);
    } else {
        err = 1;
        set_error(ctx, strdup("Index out of bounds."));
    }
    return err;
}
unsigned char *futhark_values_raw_i64_1d(struct futhark_context *ctx, struct futhark_i64_1d *arr)
{
    (void) ctx;
    return arr->mem.mem;
}
const int64_t *futhark_shape_i64_1d(struct futhark_context *ctx, struct futhark_i64_1d *arr)
{
    (void) ctx;
    return arr->shape;
}
struct futhark_opaque_tup2_bool_arr1d_u32 {
    bool v0;
    struct futhark_u32_1d *v1;
};
int futhark_project_opaque_tup2_bool_arr1d_u32_0(struct futhark_context *ctx, bool *out, const struct futhark_opaque_tup2_bool_arr1d_u32 *obj)
{
    (void) ctx;
    (void) obj;
    
    bool v;
    
    v = obj->v0;
    *out = v;
    return 0;
}
int futhark_project_opaque_tup2_bool_arr1d_u32_1(struct futhark_context *ctx, struct futhark_u32_1d **out, const struct futhark_opaque_tup2_bool_arr1d_u32 *obj)
{
    (void) ctx;
    (void) obj;
    
    struct futhark_u32_1d *v;
    
    lock_lock(&ctx->lock);
    v = malloc(sizeof(struct futhark_u32_1d));
    memcpy(v, obj->v1, sizeof(struct futhark_u32_1d));
    (void) (*v->mem.references)++;
    lock_unlock(&ctx->lock);
    *out = v;
    return 0;
}
int futhark_new_opaque_tup2_bool_arr1d_u32(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const bool f_0, const struct futhark_u32_1d *f_1)
{
    struct futhark_opaque_tup2_bool_arr1d_u32 *v = malloc(sizeof(struct futhark_opaque_tup2_bool_arr1d_u32));
    
    lock_lock(&ctx->lock);
    v->v0 = f_0;
    {
        v->v1 = malloc(sizeof(struct futhark_u32_1d));
        memcpy(v->v1, f_1, sizeof(struct futhark_u32_1d));
        (void) (*v->v1->mem.references)++;
    }
    lock_unlock(&ctx->lock);
    *out = v;
    return FUTHARK_SUCCESS;
}
int futhark_free_opaque_tup2_bool_arr1d_u32(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 *obj)
{
    (void) ctx;
    
    int ret = 0, tmp;
    
    if (obj->v1 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v1)) != 0)
        ret = tmp;
    free(obj);
    return ret;
}
int futhark_store_opaque_tup2_bool_arr1d_u32(struct futhark_context *ctx, const struct futhark_opaque_tup2_bool_arr1d_u32 *obj, void **p, size_t *n)
{
    (void) ctx;
    (void) obj;
    
    int ret = 0;
    int64_t size_0 = 7 + 0 * sizeof(int64_t) + 1 * sizeof(bool);
    int64_t size_1 = 7 + 1 * sizeof(int64_t) + futhark_shape_u32_1d(ctx, obj->v1)[0] * sizeof(int32_t);
    
    *n = size_0 + size_1;
    if (p != NULL && *p == NULL)
        *p = malloc(*n);
    if (p != NULL) {
        unsigned char *out = *p;
        
        *out++ = 'b';
        *out++ = 2;
        *out++ = 0;
        memcpy(out, "bool", 4);
        out += 4;
        memcpy(out, &obj->v0, sizeof(obj->v0));
        out += sizeof(obj->v0);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 1;
        memcpy(out, " u32", 4);
        out += 4;
        memcpy(out, futhark_shape_u32_1d(ctx, obj->v1), 1 * sizeof(int64_t));
        out += 1 * sizeof(int64_t);
        ret |= futhark_values_u32_1d(ctx, obj->v1, (void *) out);
        out += futhark_shape_u32_1d(ctx, obj->v1)[0] * sizeof(int32_t);
    }
    return ret;
}
struct futhark_opaque_tup2_bool_arr1d_u32 *futhark_restore_opaque_tup2_bool_arr1d_u32(struct futhark_context *ctx, const void *p)
{
    (void) ctx;
    
    int err = 0;
    const unsigned char *src = p;
    struct futhark_opaque_tup2_bool_arr1d_u32 *obj = malloc(sizeof(struct futhark_opaque_tup2_bool_arr1d_u32));
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 0;
    err |= memcmp(src, "bool", 4) != 0;
    src += 4;
    if (err == 0) {
        src += 0 * sizeof(int64_t);
    }
    
    const void *data_0 = src;
    
    src += sizeof(obj->v0);
    
    int64_t shape_1[1] = {0};
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 1;
    err |= memcmp(src, " u32", 4) != 0;
    src += 4;
    if (err == 0) {
        memcpy(shape_1, src, 1 * sizeof(int64_t));
        src += 1 * sizeof(int64_t);
    }
    
    const void *data_1 = src;
    
    obj->v1 = NULL;
    src += shape_1[0] * sizeof(int32_t);
    if (err == 0) {
        memcpy(&obj->v0, data_0, sizeof(obj->v0));
        obj->v1 = futhark_new_u32_1d(ctx, data_1, shape_1[0]);
        if (obj->v1 == NULL)
            err = 1;
    }
    if (err != 0) {
        int ret = 0, tmp;
        
        if (obj->v1 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v1)) != 0)
            ret = tmp;
        free(obj);
        obj = NULL;
    }
    return obj;
}
struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 {
    bool v0;
    struct futhark_u32_1d *v1;
    bool v2;
    struct futhark_u32_1d *v3;
    bool v4;
    struct futhark_u32_1d *v5;
    bool v6;
    struct futhark_u32_1d *v7;
    int64_t v8;
};
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_0(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj)
{
    (void) ctx;
    (void) obj;
    
    struct futhark_opaque_tup2_bool_arr1d_u32 *v;
    
    lock_lock(&ctx->lock);
    v = malloc(sizeof(struct futhark_opaque_tup2_bool_arr1d_u32));
    v->v0 = obj->v0;
    v->v1 = malloc(sizeof(*v->v1));
    memcpy(v->v1, obj->v1, sizeof(*obj->v1));
    (void) (*v->v1->mem.references)++;
    lock_unlock(&ctx->lock);
    *out = v;
    return 0;
}
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_1(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj)
{
    (void) ctx;
    (void) obj;
    
    struct futhark_opaque_tup2_bool_arr1d_u32 *v;
    
    lock_lock(&ctx->lock);
    v = malloc(sizeof(struct futhark_opaque_tup2_bool_arr1d_u32));
    v->v0 = obj->v2;
    v->v1 = malloc(sizeof(*v->v1));
    memcpy(v->v1, obj->v3, sizeof(*obj->v3));
    (void) (*v->v1->mem.references)++;
    lock_unlock(&ctx->lock);
    *out = v;
    return 0;
}
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_2(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj)
{
    (void) ctx;
    (void) obj;
    
    struct futhark_opaque_tup2_bool_arr1d_u32 *v;
    
    lock_lock(&ctx->lock);
    v = malloc(sizeof(struct futhark_opaque_tup2_bool_arr1d_u32));
    v->v0 = obj->v4;
    v->v1 = malloc(sizeof(*v->v1));
    memcpy(v->v1, obj->v5, sizeof(*obj->v5));
    (void) (*v->v1->mem.references)++;
    lock_unlock(&ctx->lock);
    *out = v;
    return 0;
}
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_3(struct futhark_context *ctx, struct futhark_opaque_tup2_bool_arr1d_u32 **out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj)
{
    (void) ctx;
    (void) obj;
    
    struct futhark_opaque_tup2_bool_arr1d_u32 *v;
    
    lock_lock(&ctx->lock);
    v = malloc(sizeof(struct futhark_opaque_tup2_bool_arr1d_u32));
    v->v0 = obj->v6;
    v->v1 = malloc(sizeof(*v->v1));
    memcpy(v->v1, obj->v7, sizeof(*obj->v7));
    (void) (*v->v1->mem.references)++;
    lock_unlock(&ctx->lock);
    *out = v;
    return 0;
}
int futhark_project_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64_4(struct futhark_context *ctx, int64_t *out, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj)
{
    (void) ctx;
    (void) obj;
    
    int64_t v;
    
    v = obj->v8;
    *out = v;
    return 0;
}
int futhark_new_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(struct futhark_context *ctx, struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 **out, const struct futhark_opaque_tup2_bool_arr1d_u32 *f_0, const struct futhark_opaque_tup2_bool_arr1d_u32 *f_1, const struct futhark_opaque_tup2_bool_arr1d_u32 *f_2, const struct futhark_opaque_tup2_bool_arr1d_u32 *f_3, const int64_t f_4)
{
    struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *v = malloc(sizeof(struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64));
    
    lock_lock(&ctx->lock);
    {
        v->v0 = f_0->v0;
        {
            v->v1 = malloc(sizeof(*f_0->v1));
            memcpy(v->v1, f_0->v1, sizeof(*f_0->v1));
            (void) (*v->v1->mem.references)++;
        }
    }
    {
        v->v2 = f_1->v0;
        {
            v->v3 = malloc(sizeof(*f_1->v1));
            memcpy(v->v3, f_1->v1, sizeof(*f_1->v1));
            (void) (*v->v3->mem.references)++;
        }
    }
    {
        v->v4 = f_2->v0;
        {
            v->v5 = malloc(sizeof(*f_2->v1));
            memcpy(v->v5, f_2->v1, sizeof(*f_2->v1));
            (void) (*v->v5->mem.references)++;
        }
    }
    {
        v->v6 = f_3->v0;
        {
            v->v7 = malloc(sizeof(*f_3->v1));
            memcpy(v->v7, f_3->v1, sizeof(*f_3->v1));
            (void) (*v->v7->mem.references)++;
        }
    }
    v->v8 = f_4;
    lock_unlock(&ctx->lock);
    *out = v;
    return FUTHARK_SUCCESS;
}
int futhark_free_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(struct futhark_context *ctx, struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj)
{
    (void) ctx;
    
    int ret = 0, tmp;
    
    if (obj->v1 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v1)) != 0)
        ret = tmp;
    if (obj->v3 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v3)) != 0)
        ret = tmp;
    if (obj->v5 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v5)) != 0)
        ret = tmp;
    if (obj->v7 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v7)) != 0)
        ret = tmp;
    free(obj);
    return ret;
}
int futhark_store_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(struct futhark_context *ctx, const struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj, void **p, size_t *n)
{
    (void) ctx;
    (void) obj;
    
    int ret = 0;
    int64_t size_0 = 7 + 0 * sizeof(int64_t) + 1 * sizeof(bool);
    int64_t size_1 = 7 + 1 * sizeof(int64_t) + futhark_shape_u32_1d(ctx, obj->v1)[0] * sizeof(int32_t);
    int64_t size_2 = 7 + 0 * sizeof(int64_t) + 1 * sizeof(bool);
    int64_t size_3 = 7 + 1 * sizeof(int64_t) + futhark_shape_u32_1d(ctx, obj->v3)[0] * sizeof(int32_t);
    int64_t size_4 = 7 + 0 * sizeof(int64_t) + 1 * sizeof(bool);
    int64_t size_5 = 7 + 1 * sizeof(int64_t) + futhark_shape_u32_1d(ctx, obj->v5)[0] * sizeof(int32_t);
    int64_t size_6 = 7 + 0 * sizeof(int64_t) + 1 * sizeof(bool);
    int64_t size_7 = 7 + 1 * sizeof(int64_t) + futhark_shape_u32_1d(ctx, obj->v7)[0] * sizeof(int32_t);
    int64_t size_8 = 7 + 0 * sizeof(int64_t) + 1 * sizeof(int64_t);
    
    *n = size_0 + size_1 + size_2 + size_3 + size_4 + size_5 + size_6 + size_7 + size_8;
    if (p != NULL && *p == NULL)
        *p = malloc(*n);
    if (p != NULL) {
        unsigned char *out = *p;
        
        *out++ = 'b';
        *out++ = 2;
        *out++ = 0;
        memcpy(out, "bool", 4);
        out += 4;
        memcpy(out, &obj->v0, sizeof(obj->v0));
        out += sizeof(obj->v0);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 1;
        memcpy(out, " u32", 4);
        out += 4;
        memcpy(out, futhark_shape_u32_1d(ctx, obj->v1), 1 * sizeof(int64_t));
        out += 1 * sizeof(int64_t);
        ret |= futhark_values_u32_1d(ctx, obj->v1, (void *) out);
        out += futhark_shape_u32_1d(ctx, obj->v1)[0] * sizeof(int32_t);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 0;
        memcpy(out, "bool", 4);
        out += 4;
        memcpy(out, &obj->v2, sizeof(obj->v2));
        out += sizeof(obj->v2);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 1;
        memcpy(out, " u32", 4);
        out += 4;
        memcpy(out, futhark_shape_u32_1d(ctx, obj->v3), 1 * sizeof(int64_t));
        out += 1 * sizeof(int64_t);
        ret |= futhark_values_u32_1d(ctx, obj->v3, (void *) out);
        out += futhark_shape_u32_1d(ctx, obj->v3)[0] * sizeof(int32_t);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 0;
        memcpy(out, "bool", 4);
        out += 4;
        memcpy(out, &obj->v4, sizeof(obj->v4));
        out += sizeof(obj->v4);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 1;
        memcpy(out, " u32", 4);
        out += 4;
        memcpy(out, futhark_shape_u32_1d(ctx, obj->v5), 1 * sizeof(int64_t));
        out += 1 * sizeof(int64_t);
        ret |= futhark_values_u32_1d(ctx, obj->v5, (void *) out);
        out += futhark_shape_u32_1d(ctx, obj->v5)[0] * sizeof(int32_t);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 0;
        memcpy(out, "bool", 4);
        out += 4;
        memcpy(out, &obj->v6, sizeof(obj->v6));
        out += sizeof(obj->v6);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 1;
        memcpy(out, " u32", 4);
        out += 4;
        memcpy(out, futhark_shape_u32_1d(ctx, obj->v7), 1 * sizeof(int64_t));
        out += 1 * sizeof(int64_t);
        ret |= futhark_values_u32_1d(ctx, obj->v7, (void *) out);
        out += futhark_shape_u32_1d(ctx, obj->v7)[0] * sizeof(int32_t);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 0;
        memcpy(out, " i64", 4);
        out += 4;
        memcpy(out, &obj->v8, sizeof(obj->v8));
        out += sizeof(obj->v8);
    }
    return ret;
}
struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *futhark_restore_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64(struct futhark_context *ctx, const void *p)
{
    (void) ctx;
    
    int err = 0;
    const unsigned char *src = p;
    struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *obj = malloc(sizeof(struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64));
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 0;
    err |= memcmp(src, "bool", 4) != 0;
    src += 4;
    if (err == 0) {
        src += 0 * sizeof(int64_t);
    }
    
    const void *data_0 = src;
    
    src += sizeof(obj->v0);
    
    int64_t shape_1[1] = {0};
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 1;
    err |= memcmp(src, " u32", 4) != 0;
    src += 4;
    if (err == 0) {
        memcpy(shape_1, src, 1 * sizeof(int64_t));
        src += 1 * sizeof(int64_t);
    }
    
    const void *data_1 = src;
    
    obj->v1 = NULL;
    src += shape_1[0] * sizeof(int32_t);
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 0;
    err |= memcmp(src, "bool", 4) != 0;
    src += 4;
    if (err == 0) {
        src += 0 * sizeof(int64_t);
    }
    
    const void *data_2 = src;
    
    src += sizeof(obj->v2);
    
    int64_t shape_3[1] = {0};
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 1;
    err |= memcmp(src, " u32", 4) != 0;
    src += 4;
    if (err == 0) {
        memcpy(shape_3, src, 1 * sizeof(int64_t));
        src += 1 * sizeof(int64_t);
    }
    
    const void *data_3 = src;
    
    obj->v3 = NULL;
    src += shape_3[0] * sizeof(int32_t);
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 0;
    err |= memcmp(src, "bool", 4) != 0;
    src += 4;
    if (err == 0) {
        src += 0 * sizeof(int64_t);
    }
    
    const void *data_4 = src;
    
    src += sizeof(obj->v4);
    
    int64_t shape_5[1] = {0};
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 1;
    err |= memcmp(src, " u32", 4) != 0;
    src += 4;
    if (err == 0) {
        memcpy(shape_5, src, 1 * sizeof(int64_t));
        src += 1 * sizeof(int64_t);
    }
    
    const void *data_5 = src;
    
    obj->v5 = NULL;
    src += shape_5[0] * sizeof(int32_t);
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 0;
    err |= memcmp(src, "bool", 4) != 0;
    src += 4;
    if (err == 0) {
        src += 0 * sizeof(int64_t);
    }
    
    const void *data_6 = src;
    
    src += sizeof(obj->v6);
    
    int64_t shape_7[1] = {0};
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 1;
    err |= memcmp(src, " u32", 4) != 0;
    src += 4;
    if (err == 0) {
        memcpy(shape_7, src, 1 * sizeof(int64_t));
        src += 1 * sizeof(int64_t);
    }
    
    const void *data_7 = src;
    
    obj->v7 = NULL;
    src += shape_7[0] * sizeof(int32_t);
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 0;
    err |= memcmp(src, " i64", 4) != 0;
    src += 4;
    if (err == 0) {
        src += 0 * sizeof(int64_t);
    }
    
    const void *data_8 = src;
    
    src += sizeof(obj->v8);
    if (err == 0) {
        memcpy(&obj->v0, data_0, sizeof(obj->v0));
        obj->v1 = futhark_new_u32_1d(ctx, data_1, shape_1[0]);
        if (obj->v1 == NULL)
            err = 1;
        memcpy(&obj->v2, data_2, sizeof(obj->v2));
        obj->v3 = futhark_new_u32_1d(ctx, data_3, shape_3[0]);
        if (obj->v3 == NULL)
            err = 1;
        memcpy(&obj->v4, data_4, sizeof(obj->v4));
        obj->v5 = futhark_new_u32_1d(ctx, data_5, shape_5[0]);
        if (obj->v5 == NULL)
            err = 1;
        memcpy(&obj->v6, data_6, sizeof(obj->v6));
        obj->v7 = futhark_new_u32_1d(ctx, data_7, shape_7[0]);
        if (obj->v7 == NULL)
            err = 1;
        memcpy(&obj->v8, data_8, sizeof(obj->v8));
    }
    if (err != 0) {
        int ret = 0, tmp;
        
        if (obj->v1 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v1)) != 0)
            ret = tmp;
        if (obj->v3 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v3)) != 0)
            ret = tmp;
        if (obj->v5 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v5)) != 0)
            ret = tmp;
        if (obj->v7 != NULL && (tmp = futhark_free_u32_1d(ctx, obj->v7)) != 0)
            ret = tmp;
        free(obj);
        obj = NULL;
    }
    return obj;
}
struct futhark_opaque_tup2_i64_arr1d_u8 {
    int64_t v0;
    struct futhark_u8_1d *v1;
};
int futhark_project_opaque_tup2_i64_arr1d_u8_0(struct futhark_context *ctx, int64_t *out, const struct futhark_opaque_tup2_i64_arr1d_u8 *obj)
{
    (void) ctx;
    (void) obj;
    
    int64_t v;
    
    v = obj->v0;
    *out = v;
    return 0;
}
int futhark_project_opaque_tup2_i64_arr1d_u8_1(struct futhark_context *ctx, struct futhark_u8_1d **out, const struct futhark_opaque_tup2_i64_arr1d_u8 *obj)
{
    (void) ctx;
    (void) obj;
    
    struct futhark_u8_1d *v;
    
    lock_lock(&ctx->lock);
    v = malloc(sizeof(struct futhark_u8_1d));
    memcpy(v, obj->v1, sizeof(struct futhark_u8_1d));
    (void) (*v->mem.references)++;
    lock_unlock(&ctx->lock);
    *out = v;
    return 0;
}
int futhark_new_opaque_tup2_i64_arr1d_u8(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 **out, const int64_t f_0, const struct futhark_u8_1d *f_1)
{
    struct futhark_opaque_tup2_i64_arr1d_u8 *v = malloc(sizeof(struct futhark_opaque_tup2_i64_arr1d_u8));
    
    lock_lock(&ctx->lock);
    v->v0 = f_0;
    {
        v->v1 = malloc(sizeof(struct futhark_u8_1d));
        memcpy(v->v1, f_1, sizeof(struct futhark_u8_1d));
        (void) (*v->v1->mem.references)++;
    }
    lock_unlock(&ctx->lock);
    *out = v;
    return FUTHARK_SUCCESS;
}
int futhark_free_opaque_tup2_i64_arr1d_u8(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 *obj)
{
    (void) ctx;
    
    int ret = 0, tmp;
    
    if (obj->v1 != NULL && (tmp = futhark_free_u8_1d(ctx, obj->v1)) != 0)
        ret = tmp;
    free(obj);
    return ret;
}
int futhark_store_opaque_tup2_i64_arr1d_u8(struct futhark_context *ctx, const struct futhark_opaque_tup2_i64_arr1d_u8 *obj, void **p, size_t *n)
{
    (void) ctx;
    (void) obj;
    
    int ret = 0;
    int64_t size_0 = 7 + 0 * sizeof(int64_t) + 1 * sizeof(int64_t);
    int64_t size_1 = 7 + 1 * sizeof(int64_t) + futhark_shape_u8_1d(ctx, obj->v1)[0] * sizeof(int8_t);
    
    *n = size_0 + size_1;
    if (p != NULL && *p == NULL)
        *p = malloc(*n);
    if (p != NULL) {
        unsigned char *out = *p;
        
        *out++ = 'b';
        *out++ = 2;
        *out++ = 0;
        memcpy(out, " i64", 4);
        out += 4;
        memcpy(out, &obj->v0, sizeof(obj->v0));
        out += sizeof(obj->v0);
        *out++ = 'b';
        *out++ = 2;
        *out++ = 1;
        memcpy(out, "  u8", 4);
        out += 4;
        memcpy(out, futhark_shape_u8_1d(ctx, obj->v1), 1 * sizeof(int64_t));
        out += 1 * sizeof(int64_t);
        ret |= futhark_values_u8_1d(ctx, obj->v1, (void *) out);
        out += futhark_shape_u8_1d(ctx, obj->v1)[0] * sizeof(int8_t);
    }
    return ret;
}
struct futhark_opaque_tup2_i64_arr1d_u8 *futhark_restore_opaque_tup2_i64_arr1d_u8(struct futhark_context *ctx, const void *p)
{
    (void) ctx;
    
    int err = 0;
    const unsigned char *src = p;
    struct futhark_opaque_tup2_i64_arr1d_u8 *obj = malloc(sizeof(struct futhark_opaque_tup2_i64_arr1d_u8));
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 0;
    err |= memcmp(src, " i64", 4) != 0;
    src += 4;
    if (err == 0) {
        src += 0 * sizeof(int64_t);
    }
    
    const void *data_0 = src;
    
    src += sizeof(obj->v0);
    
    int64_t shape_1[1] = {0};
    
    err |= *src++ != 'b';
    err |= *src++ != 2;
    err |= *src++ != 1;
    err |= memcmp(src, "  u8", 4) != 0;
    src += 4;
    if (err == 0) {
        memcpy(shape_1, src, 1 * sizeof(int64_t));
        src += 1 * sizeof(int64_t);
    }
    
    const void *data_1 = src;
    
    obj->v1 = NULL;
    src += shape_1[0] * sizeof(int8_t);
    if (err == 0) {
        memcpy(&obj->v0, data_0, sizeof(obj->v0));
        obj->v1 = futhark_new_u8_1d(ctx, data_1, shape_1[0]);
        if (obj->v1 == NULL)
            err = 1;
    }
    if (err != 0) {
        int ret = 0, tmp;
        
        if (obj->v1 != NULL && (tmp = futhark_free_u8_1d(ctx, obj->v1)) != 0)
            ret = tmp;
        free(obj);
        obj = NULL;
    }
    return obj;
}

FUTHARK_FUN_ATTR int futrts_add_abs_11626(struct futhark_context *ctx, struct memblock *mem_out_p_22939, int64_t *out_prim_out_22940, struct memblock a_mem_22579, struct memblock b_mem_22580, int64_t dz2080U_14869, int64_t dz2081U_14870)
{
    (void) ctx;
    
    int err = 0;
    struct memblock ext_mem_22591;
    
    ext_mem_22591.references = NULL;
    
    struct memblock mem_22582;
    
    mem_22582.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22832;
    
    // stdlib/bigint-fast.fut:21:14-42
    
    int64_t max_res_18613 = smax64(dz2080U_14869, dz2081U_14870);
    
    // stdlib/bigint-fast.fut:22:62-68
    
    int64_t dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_14879 = add64((int64_t) 1, max_res_18613);
    int64_t bytes_22581 = (int64_t) 4 * dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_14879;
    
    // stdlib/bigint-fast.fut:28:7-32
    
    bool x_14930 = sle64((int64_t) 0, max_res_18613);
    
    // stdlib/bigint-fast.fut:28:7-32
    
    bool y_14931 = slt64(max_res_18613, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_14879);
    
    // stdlib/bigint-fast.fut:28:7-32
    
    bool bounds_check_14932 = x_14930 && y_14931;
    
    // stdlib/bigint-fast.fut:28:7-32
    
    bool index_certs_14933;
    
    if (!bounds_check_14932) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) max_res_18613, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_14879, "].", "-> #0  stdlib/bigint-fast.fut:28:7-32\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    if (memblock_alloc(ctx, &mem_22582, bytes_22581, "mem_22582")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:22:51-74
    for (int64_t nest_i_22833 = 0; nest_i_22833 < dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_14879; nest_i_22833++) {
        ((int32_t *) mem_22582.mem)[nest_i_22833] = 0;
    }
    // stdlib/bigint-fast.fut:22:31-27:23
    
    int64_t add_abs_res_14887;
    int64_t carry_14890 = (int64_t) 0;
    
    for (int64_t i_14888 = 0; i_14888 < max_res_18613; i_14888++) {
        // stdlib/bigint-fast.fut:23:13-53
        
        bool cond_14893 = slt64(i_14888, dz2080U_14869);
        
        // stdlib/bigint-fast.fut:23:13-53
        
        int64_t x_14894;
        
        if (cond_14893) {
            // stdlib/bigint-fast.fut:23:42-46
            
            int32_t u32_arg0_21918 = ((int32_t *) a_mem_22579.mem)[i_14888];
            
            // stdlib/bigint-fast.fut:23:34-46
            
            int64_t u32_res_21919 = zext_i32_i64(u32_arg0_21918);
            
            x_14894 = u32_res_21919;
        } else {
            x_14894 = (int64_t) 0;
        }
        // stdlib/bigint-fast.fut:24:13-53
        
        bool cond_14904 = slt64(i_14888, dz2081U_14870);
        
        // stdlib/bigint-fast.fut:24:13-53
        
        int64_t y_14905;
        
        if (cond_14904) {
            // stdlib/bigint-fast.fut:24:42-46
            
            int32_t u32_arg0_21920 = ((int32_t *) b_mem_22580.mem)[i_14888];
            
            // stdlib/bigint-fast.fut:24:34-46
            
            int64_t u32_res_21921 = zext_i32_i64(u32_arg0_21920);
            
            y_14905 = u32_res_21921;
        } else {
            y_14905 = (int64_t) 0;
        }
        // stdlib/bigint-fast.fut:25:16-18
        
        int64_t zp_lhs_14913 = add64(x_14894, y_14905);
        
        // stdlib/bigint-fast.fut:25:18-24
        
        int64_t sum_14915 = add64(carry_14890, zp_lhs_14913);
        
        // stdlib/bigint-fast.fut:26:18-29
        
        int32_t u64_res_18736 = zext_i64_i32(sum_14915);
        
        // stdlib/bigint-fast.fut:26:9-29
        ((int32_t *) mem_22582.mem)[i_14888] = u64_res_18736;
        // stdlib/bigint-fast.fut:27:17-22
        
        int64_t tmp_14924 = lshr64(sum_14915, (int64_t) 32);
        int64_t carry_tmp_22835 = tmp_14924;
        
        carry_14890 = carry_tmp_22835;
    }
    add_abs_res_14887 = carry_14890;
    // stdlib/bigint-fast.fut:28:19-32
    
    int32_t u64_res_18738 = zext_i64_i32(add_abs_res_14887);
    
    // stdlib/bigint-fast.fut:28:7-32
    ((int32_t *) mem_22582.mem)[max_res_18613] = u64_res_18738;
    // stdlib/bigint-fast.fut:29:7-26
    
    int64_t add_abs_res_14936;
    bool add_abs_res_14937;
    
    if (futrts_normalise_11622(ctx, &ext_mem_22591, &add_abs_res_14936, &add_abs_res_14937, mem_22582, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_14879, 0) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &mem_22582, "mem_22582") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &ext_mem_22591, "ext_mem_22591") != 0)
        return 1;
    prim_out_22832 = add_abs_res_14936;
    if (memblock_set(ctx, &*mem_out_p_22939, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    *out_prim_out_22940 = prim_out_22832;
    
  cleanup:
    {
        if (memblock_unref(ctx, &ext_mem_22591, "ext_mem_22591") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22582, "mem_22582") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_empty_8082(struct futhark_context *ctx, struct memblock *mem_out_p_22941, struct memblock *mem_out_p_22942, struct memblock *mem_out_p_22943, int64_t *out_prim_out_22944, int64_t capacity_17577)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_22583;
    
    mem_22583.references = NULL;
    
    struct memblock mem_22582;
    
    mem_22582.references = NULL;
    
    struct memblock mem_22580;
    
    mem_22580.references = NULL;
    
    struct memblock mem_out_22833;
    
    mem_out_22833.references = NULL;
    
    struct memblock mem_out_22832;
    
    mem_out_22832.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22834;
    
    // stdlib/hashmap.fut:4:11-55
    
    bool cond_17578 = slt64((int64_t) 0, capacity_17577);
    
    // stdlib/hashmap.fut:4:47-49
    
    int64_t za_rhs_17580 = sub64(capacity_17577, (int64_t) 1);
    
    // stdlib/hashmap.fut:4:36-49
    
    int64_t zeze_lhs_17582 = capacity_17577 & za_rhs_17580;
    
    // stdlib/hashmap.fut:4:51-55
    
    bool tmp_t_res_17584 = zeze_lhs_17582 == (int64_t) 0;
    bool x_18547 = cond_17578 && tmp_t_res_17584;
    int64_t bytes_22579 = (int64_t) 8 * capacity_17577;
    
    // stdlib/hashmap.fut:4:3-5:46
    
    bool assert_c_17592;
    
    if (!x_18547) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "Assertion is false: (capacity > 0 && (capacity & (capacity - 1)) == 0)", "-> #0  stdlib/hashmap.fut:4:3-5:46\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    if (memblock_alloc(ctx, &mem_22580, bytes_22579, "mem_22580")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:5:4-40
    for (int64_t nest_i_22835 = 0; nest_i_22835 < capacity_17577; nest_i_22835++) {
        ((int64_t *) mem_22580.mem)[nest_i_22835] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_22582, bytes_22579, "mem_22582")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:5:4-40
    for (int64_t nest_i_22836 = 0; nest_i_22836 < capacity_17577; nest_i_22836++) {
        ((int64_t *) mem_22582.mem)[nest_i_22836] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_22583, capacity_17577, "mem_22583")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:5:4-40
    for (int64_t nest_i_22837 = 0; nest_i_22837 < capacity_17577; nest_i_22837++) {
        ((bool *) mem_22583.mem)[nest_i_22837] = 0;
    }
    if (memblock_set(ctx, &mem_out_22831, &mem_22580, "mem_22580") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22832, &mem_22582, "mem_22582") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22833, &mem_22583, "mem_22583") != 0)
        return 1;
    prim_out_22834 = (int64_t) 0;
    if (memblock_set(ctx, &*mem_out_p_22941, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22942, &mem_out_22832, "mem_out_22832") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22943, &mem_out_22833, "mem_out_22833") != 0)
        return 1;
    *out_prim_out_22944 = prim_out_22834;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_22583, "mem_22583") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22582, "mem_22582") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22580, "mem_22580") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22833, "mem_out_22833") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22832, "mem_out_22832") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_arithmetic(struct futhark_context *ctx, struct memblock *mem_out_p_22945, struct memblock *mem_out_p_22946, struct memblock *mem_out_p_22947, struct memblock *mem_out_p_22948, int64_t *out_prim_out_22949, int64_t *out_prim_out_22950, int64_t *out_prim_out_22951, int64_t *out_prim_out_22952, bool *out_prim_out_22953, bool *out_prim_out_22954, bool *out_prim_out_22955, bool *out_prim_out_22956, int64_t *out_prim_out_22957, struct memblock a_mem_22579, struct memblock b_mem_22580, int64_t dz2080U_15866, int64_t dz2081U_15867, bool negative_a_15868, bool negative_b_15870, int32_t factor_15872)
{
    (void) ctx;
    
    int err = 0;
    struct memblock ext_mem_22715;
    
    ext_mem_22715.references = NULL;
    
    struct memblock ext_mem_22698;
    
    ext_mem_22698.references = NULL;
    
    struct memblock ext_mem_22697;
    
    ext_mem_22697.references = NULL;
    
    struct memblock mem_22688;
    
    mem_22688.references = NULL;
    
    struct memblock ext_mem_22710;
    
    ext_mem_22710.references = NULL;
    
    struct memblock ext_mem_22709;
    
    ext_mem_22709.references = NULL;
    
    struct memblock mem_22700;
    
    mem_22700.references = NULL;
    
    struct memblock ext_mem_22711;
    
    ext_mem_22711.references = NULL;
    
    struct memblock ext_mem_22713;
    
    ext_mem_22713.references = NULL;
    
    struct memblock ext_mem_22712;
    
    ext_mem_22712.references = NULL;
    
    struct memblock ext_mem_22714;
    
    ext_mem_22714.references = NULL;
    
    struct memblock ext_mem_22686;
    
    ext_mem_22686.references = NULL;
    
    struct memblock ext_mem_22716;
    
    ext_mem_22716.references = NULL;
    
    struct memblock ext_mem_22717;
    
    ext_mem_22717.references = NULL;
    
    struct memblock ext_mem_22718;
    
    ext_mem_22718.references = NULL;
    
    struct memblock ext_mem_22685;
    
    ext_mem_22685.references = NULL;
    
    struct memblock mem_22684;
    
    mem_22684.references = NULL;
    
    struct memblock ext_mem_22683;
    
    ext_mem_22683.references = NULL;
    
    struct memblock mem_22674;
    
    mem_22674.references = NULL;
    
    struct memblock mem_param_tmp_22915;
    
    mem_param_tmp_22915.references = NULL;
    
    struct memblock ext_mem_22669;
    
    ext_mem_22669.references = NULL;
    
    struct memblock ext_mem_22668;
    
    ext_mem_22668.references = NULL;
    
    struct memblock mem_param_22665;
    
    mem_param_22665.references = NULL;
    
    struct memblock ext_mem_22672;
    
    ext_mem_22672.references = NULL;
    
    struct memblock ext_mem_22662;
    
    ext_mem_22662.references = NULL;
    
    struct memblock mem_param_tmp_22901;
    
    mem_param_tmp_22901.references = NULL;
    
    struct memblock ext_mem_22656;
    
    ext_mem_22656.references = NULL;
    
    struct memblock mem_22647;
    
    mem_22647.references = NULL;
    
    struct memblock mem_param_22645;
    
    mem_param_22645.references = NULL;
    
    struct memblock ext_mem_22659;
    
    ext_mem_22659.references = NULL;
    
    struct memblock ext_mem_22642;
    
    ext_mem_22642.references = NULL;
    
    struct memblock ext_mem_22719;
    
    ext_mem_22719.references = NULL;
    
    struct memblock ext_mem_22720;
    
    ext_mem_22720.references = NULL;
    
    struct memblock ext_mem_22803;
    
    ext_mem_22803.references = NULL;
    
    struct memblock ext_mem_22786;
    
    ext_mem_22786.references = NULL;
    
    struct memblock ext_mem_22785;
    
    ext_mem_22785.references = NULL;
    
    struct memblock mem_22776;
    
    mem_22776.references = NULL;
    
    struct memblock ext_mem_22798;
    
    ext_mem_22798.references = NULL;
    
    struct memblock ext_mem_22797;
    
    ext_mem_22797.references = NULL;
    
    struct memblock mem_22788;
    
    mem_22788.references = NULL;
    
    struct memblock ext_mem_22799;
    
    ext_mem_22799.references = NULL;
    
    struct memblock ext_mem_22801;
    
    ext_mem_22801.references = NULL;
    
    struct memblock ext_mem_22800;
    
    ext_mem_22800.references = NULL;
    
    struct memblock ext_mem_22802;
    
    ext_mem_22802.references = NULL;
    
    struct memblock ext_mem_22774;
    
    ext_mem_22774.references = NULL;
    
    struct memblock ext_mem_22804;
    
    ext_mem_22804.references = NULL;
    
    struct memblock ext_mem_22805;
    
    ext_mem_22805.references = NULL;
    
    struct memblock ext_mem_22806;
    
    ext_mem_22806.references = NULL;
    
    struct memblock ext_mem_22773;
    
    ext_mem_22773.references = NULL;
    
    struct memblock mem_param_tmp_22866;
    
    mem_param_tmp_22866.references = NULL;
    
    struct memblock ext_mem_22764;
    
    ext_mem_22764.references = NULL;
    
    struct memblock mem_22755;
    
    mem_22755.references = NULL;
    
    struct memblock mem_param_tmp_22875;
    
    mem_param_tmp_22875.references = NULL;
    
    struct memblock ext_mem_22750;
    
    ext_mem_22750.references = NULL;
    
    struct memblock mem_22741;
    
    mem_22741.references = NULL;
    
    struct memblock mem_param_22739;
    
    mem_param_22739.references = NULL;
    
    struct memblock ext_mem_22753;
    
    ext_mem_22753.references = NULL;
    
    struct memblock ext_mem_22736;
    
    ext_mem_22736.references = NULL;
    
    struct memblock mem_22735;
    
    mem_22735.references = NULL;
    
    struct memblock ext_mem_22733;
    
    ext_mem_22733.references = NULL;
    
    struct memblock mem_param_22730;
    
    mem_param_22730.references = NULL;
    
    struct memblock ext_mem_22769;
    
    ext_mem_22769.references = NULL;
    
    struct memblock mem_22724;
    
    mem_22724.references = NULL;
    
    struct memblock ext_mem_22722;
    
    ext_mem_22722.references = NULL;
    
    struct memblock ext_mem_22721;
    
    ext_mem_22721.references = NULL;
    
    struct memblock ext_mem_22807;
    
    ext_mem_22807.references = NULL;
    
    struct memblock ext_mem_22641;
    
    ext_mem_22641.references = NULL;
    
    struct memblock ext_mem_22640;
    
    ext_mem_22640.references = NULL;
    
    struct memblock ext_mem_22623;
    
    ext_mem_22623.references = NULL;
    
    struct memblock ext_mem_22622;
    
    ext_mem_22622.references = NULL;
    
    struct memblock mem_22613;
    
    mem_22613.references = NULL;
    
    struct memblock ext_mem_22635;
    
    ext_mem_22635.references = NULL;
    
    struct memblock ext_mem_22634;
    
    ext_mem_22634.references = NULL;
    
    struct memblock mem_22625;
    
    mem_22625.references = NULL;
    
    struct memblock ext_mem_22636;
    
    ext_mem_22636.references = NULL;
    
    struct memblock ext_mem_22638;
    
    ext_mem_22638.references = NULL;
    
    struct memblock ext_mem_22637;
    
    ext_mem_22637.references = NULL;
    
    struct memblock ext_mem_22639;
    
    ext_mem_22639.references = NULL;
    
    struct memblock ext_mem_22611;
    
    ext_mem_22611.references = NULL;
    
    struct memblock ext_mem_22594;
    
    ext_mem_22594.references = NULL;
    
    struct memblock ext_mem_22593;
    
    ext_mem_22593.references = NULL;
    
    struct memblock mem_22584;
    
    mem_22584.references = NULL;
    
    struct memblock ext_mem_22606;
    
    ext_mem_22606.references = NULL;
    
    struct memblock ext_mem_22605;
    
    ext_mem_22605.references = NULL;
    
    struct memblock mem_22596;
    
    mem_22596.references = NULL;
    
    struct memblock ext_mem_22607;
    
    ext_mem_22607.references = NULL;
    
    struct memblock ext_mem_22609;
    
    ext_mem_22609.references = NULL;
    
    struct memblock ext_mem_22608;
    
    ext_mem_22608.references = NULL;
    
    struct memblock ext_mem_22610;
    
    ext_mem_22610.references = NULL;
    
    struct memblock ext_mem_22582;
    
    ext_mem_22582.references = NULL;
    
    struct memblock ext_mem_22581;
    
    ext_mem_22581.references = NULL;
    
    struct memblock mem_out_22834;
    
    mem_out_22834.references = NULL;
    
    struct memblock mem_out_22833;
    
    mem_out_22833.references = NULL;
    
    struct memblock mem_out_22832;
    
    mem_out_22832.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22835;
    int64_t prim_out_22836;
    int64_t prim_out_22837;
    int64_t prim_out_22838;
    bool prim_out_22839;
    bool prim_out_22840;
    bool prim_out_22841;
    bool prim_out_22842;
    int64_t prim_out_22843;
    
    // probes/stdlib-fast-tests.fut:6:11-33
    
    int64_t arithmetic_res_20358;
    bool arithmetic_res_20359;
    
    if (futrts_normalise_11622(ctx, &ext_mem_22581, &arithmetic_res_20358, &arithmetic_res_20359, a_mem_22579, dz2080U_15866, negative_a_15868) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:7:11-33
    
    int64_t arithmetic_res_20361;
    bool arithmetic_res_20362;
    
    if (futrts_normalise_11622(ctx, &ext_mem_22582, &arithmetic_res_20361, &arithmetic_res_20362, b_mem_22580, dz2081U_15867, negative_b_15870) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:38:3-40:39
    
    bool cond_20364 = arithmetic_res_20359 == arithmetic_res_20362;
    
    // stdlib/bigint-fast.fut:83:13-43
    
    bool cond_20545 = arithmetic_res_20361 == (int64_t) 1;
    
    // stdlib/bigint-fast.fut:83:13-43
    
    bool tmp_20546;
    
    if (cond_20545) {
        // stdlib/bigint-fast.fut:83:32-38
        
        bool y_22034 = slt64((int64_t) 0, arithmetic_res_20361);
        
        // stdlib/bigint-fast.fut:83:32-38
        
        bool index_certs_22035;
        
        if (!y_22034) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) arithmetic_res_20361, "].", "-> #0  stdlib/bigint-fast.fut:83:32-38\n   #1  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:83:32-38
        
        int32_t zeze_lhs_22036 = ((int32_t *) ext_mem_22582.mem)[(int64_t) 0];
        
        // stdlib/bigint-fast.fut:83:39-43
        
        bool tmp_t_res_22037 = zeze_lhs_22036 == 0;
        
        tmp_20546 = tmp_t_res_22037;
    } else {
        tmp_20546 = 0;
    }
    // stdlib/bigint-fast.fut:87:25-27
    
    int64_t u32_arg0_20551 = sub64(arithmetic_res_20358, (int64_t) 1);
    
    // stdlib/bigint-fast.fut:87:13-28
    
    bool x_20552 = sle64((int64_t) 0, u32_arg0_20551);
    
    // stdlib/bigint-fast.fut:87:13-28
    
    bool y_20553 = slt64(u32_arg0_20551, arithmetic_res_20358);
    
    // stdlib/bigint-fast.fut:87:13-28
    
    bool bounds_check_20554 = x_20552 && y_20553;
    
    // stdlib/bigint-fast.fut:87:13-28
    
    bool index_certs_20555;
    
    if (!bounds_check_20554) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u32_arg0_20551, "] out of bounds for array of shape [", (long long) arithmetic_res_20358, "].", "-> #0  stdlib/bigint-fast.fut:87:13-28\n   #1  stdlib/bigint-fast.fut:88:26-29\n   #2  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:87:13-28
    
    int32_t u32_arg0_20556 = ((int32_t *) ext_mem_22581.mem)[u32_arg0_20551];
    
    // stdlib/bigint-fast.fut:87:5-28
    
    double u32_res_20557 = uitofp_i32_f64(u32_arg0_20556);
    
    // stdlib/bigint-fast.fut:87:30-96
    
    bool cond_20558 = slt64((int64_t) 1, arithmetic_res_20358);
    
    // stdlib/bigint-fast.fut:87:30-96
    
    double zp_rhs_20559;
    
    if (cond_20558) {
        // stdlib/bigint-fast.fut:87:72-74
        
        int64_t u32_arg0_22038 = sub64(arithmetic_res_20358, (int64_t) 2);
        
        // stdlib/bigint-fast.fut:87:60-75
        
        bool x_22039 = sle64((int64_t) 0, u32_arg0_22038);
        
        // stdlib/bigint-fast.fut:87:60-75
        
        bool y_22040 = slt64(u32_arg0_22038, arithmetic_res_20358);
        
        // stdlib/bigint-fast.fut:87:60-75
        
        bool bounds_check_22041 = x_22039 && y_22040;
        
        // stdlib/bigint-fast.fut:87:60-75
        
        bool index_certs_22042;
        
        if (!bounds_check_22041) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u32_arg0_22038, "] out of bounds for array of shape [", (long long) arithmetic_res_20358, "].", "-> #0  stdlib/bigint-fast.fut:87:60-75\n   #1  stdlib/bigint-fast.fut:88:26-29\n   #2  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:87:60-75
        
        int32_t u32_arg0_22043 = ((int32_t *) ext_mem_22581.mem)[u32_arg0_22038];
        
        // stdlib/bigint-fast.fut:87:52-75
        
        double u32_res_22044 = uitofp_i32_f64(u32_arg0_22043);
        
        // stdlib/bigint-fast.fut:87:75-89
        
        double zp_rhs_t_res_22045 = u32_res_22044 / 4.294967296e9;
        
        zp_rhs_20559 = zp_rhs_t_res_22045;
    } else {
        zp_rhs_20559 = 0.0;
    }
    // stdlib/bigint-fast.fut:87:28-96
    
    double lifted_leading_res_20568 = u32_res_20557 + zp_rhs_20559;
    
    // stdlib/bigint-fast.fut:87:25-27
    
    int64_t u32_arg0_20569 = sub64(arithmetic_res_20361, (int64_t) 1);
    
    // stdlib/bigint-fast.fut:87:13-28
    
    bool x_20570 = sle64((int64_t) 0, u32_arg0_20569);
    
    // stdlib/bigint-fast.fut:87:13-28
    
    bool y_20571 = slt64(u32_arg0_20569, arithmetic_res_20361);
    
    // stdlib/bigint-fast.fut:87:13-28
    
    bool bounds_check_20572 = x_20570 && y_20571;
    
    // stdlib/bigint-fast.fut:87:13-28
    
    bool index_certs_20573;
    
    if (!bounds_check_20572) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u32_arg0_20569, "] out of bounds for array of shape [", (long long) arithmetic_res_20361, "].", "-> #0  stdlib/bigint-fast.fut:87:13-28\n   #1  stdlib/bigint-fast.fut:88:38-41\n   #2  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:87:13-28
    
    int32_t u32_arg0_20574 = ((int32_t *) ext_mem_22582.mem)[u32_arg0_20569];
    
    // stdlib/bigint-fast.fut:87:5-28
    
    double u32_res_20575 = uitofp_i32_f64(u32_arg0_20574);
    
    // stdlib/bigint-fast.fut:87:30-96
    
    bool cond_20576 = slt64((int64_t) 1, arithmetic_res_20361);
    
    // stdlib/bigint-fast.fut:87:30-96
    
    double zp_rhs_20577;
    
    if (cond_20576) {
        // stdlib/bigint-fast.fut:87:72-74
        
        int64_t u32_arg0_22046 = sub64(arithmetic_res_20361, (int64_t) 2);
        
        // stdlib/bigint-fast.fut:87:60-75
        
        bool x_22047 = sle64((int64_t) 0, u32_arg0_22046);
        
        // stdlib/bigint-fast.fut:87:60-75
        
        bool y_22048 = slt64(u32_arg0_22046, arithmetic_res_20361);
        
        // stdlib/bigint-fast.fut:87:60-75
        
        bool bounds_check_22049 = x_22047 && y_22048;
        
        // stdlib/bigint-fast.fut:87:60-75
        
        bool index_certs_22050;
        
        if (!bounds_check_22049) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u32_arg0_22046, "] out of bounds for array of shape [", (long long) arithmetic_res_20361, "].", "-> #0  stdlib/bigint-fast.fut:87:60-75\n   #1  stdlib/bigint-fast.fut:88:38-41\n   #2  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:87:60-75
        
        int32_t u32_arg0_22051 = ((int32_t *) ext_mem_22582.mem)[u32_arg0_22046];
        
        // stdlib/bigint-fast.fut:87:52-75
        
        double u32_res_22052 = uitofp_i32_f64(u32_arg0_22051);
        
        // stdlib/bigint-fast.fut:87:75-89
        
        double zp_rhs_t_res_22053 = u32_res_22052 / 4.294967296e9;
        
        zp_rhs_20577 = zp_rhs_t_res_22053;
    } else {
        zp_rhs_20577 = 0.0;
    }
    // stdlib/bigint-fast.fut:87:28-96
    
    double lifted_leading_res_20586 = u32_res_20575 + zp_rhs_20577;
    
    // stdlib/bigint-fast.fut:88:29-41
    
    double zt_lhs_20587 = lifted_leading_res_20568 / lifted_leading_res_20586;
    
    // stdlib/bigint-fast.fut:88:54-56
    
    int64_t zeze_rhs_20588 = add64((int64_t) 1, arithmetic_res_20361);
    
    // stdlib/bigint-fast.fut:88:43-82
    
    bool cond_20589 = arithmetic_res_20358 == zeze_rhs_20588;
    
    // stdlib/bigint-fast.fut:88:43-82
    
    double zt_rhs_20590;
    
    if (cond_20589) {
        zt_rhs_20590 = 4.294967296e9;
    } else {
        zt_rhs_20590 = 1.0;
    }
    // stdlib/bigint-fast.fut:88:41-82
    
    double estimate_20591 = zt_lhs_20587 * zt_rhs_20590;
    
    // stdlib/bigint-fast.fut:83:3-103:50
    
    bool assert_cond_20592 = !tmp_20546;
    
    // stdlib/bigint-fast.fut:83:3-103:50
    
    bool assert_c_20593;
    
    if (!assert_cond_20592) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "Assertion is false: (!(length b.1 == 1 && b.1[0] == 0))", "-> #0  stdlib/bigint-fast.fut:83:3-103:50\n   #1  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:83:3-103:50
    
    bool cond_20594 = slt64(zeze_rhs_20588, arithmetic_res_20358);
    
    // stdlib/bigint-fast.fut:89:31-47
    
    bool cond_f_res_20595 = 4.294967294e9 <= estimate_20591;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool x_20596 = !cond_20594;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool y_20597 = cond_f_res_20595 && x_20596;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool cond_20598 = cond_20594 || y_20597;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool x_20599 = !cond_20598;
    
    // stdlib/bigint-fast.fut:90:14-33
    
    int64_t zl_lhs_20600;
    
    if (x_20599) {
        // stdlib/bigint-fast.fut:12:3-16:17
        
        bool cond_22056 = arithmetic_res_20358 == arithmetic_res_20361;
        
        // stdlib/bigint-fast.fut:12:3-16:17
        
        bool cond_22057 = !cond_22056;
        
        // stdlib/bigint-fast.fut:12:3-16:17
        
        int64_t compare_abs_res_22058;
        
        if (cond_22057) {
            // stdlib/bigint-fast.fut:12:32-69
            
            bool cond_22059 = slt64(arithmetic_res_20358, arithmetic_res_20361);
            
            // stdlib/bigint-fast.fut:12:32-69
            
            int64_t compare_abs_res_t_res_22060;
            
            if (cond_22059) {
                compare_abs_res_t_res_22060 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_22060 = (int64_t) 1;
            }
            compare_abs_res_22058 = compare_abs_res_t_res_22060;
        } else {
            // stdlib/bigint-fast.fut:13:36-15:74
            
            bool compare_abs_res_f_res_22061;
            int64_t compare_abs_res_f_res_22062;
            int64_t compare_abs_res_f_res_22063;
            bool loop_while_22064;
            int64_t i_22065;
            int64_t result_22066;
            
            loop_while_22064 = x_20552;
            i_22065 = u32_arg0_20551;
            result_22066 = (int64_t) 0;
            while (loop_while_22064) {
                // stdlib/bigint-fast.fut:15:14-16
                
                int64_t tmp_22067 = sub64(i_22065, (int64_t) 1);
                
                // stdlib/bigint-fast.fut:15:20-24
                
                int32_t zl_lhs_22068 = ((int32_t *) ext_mem_22581.mem)[i_22065];
                
                // stdlib/bigint-fast.fut:15:27-31
                
                int32_t zl_rhs_22069 = ((int32_t *) ext_mem_22582.mem)[i_22065];
                
                // stdlib/bigint-fast.fut:15:17-73
                
                bool cond_22070 = ult32(zl_lhs_22068, zl_rhs_22069);
                
                // stdlib/bigint-fast.fut:15:17-73
                
                int64_t tmp_22071;
                
                if (cond_22070) {
                    tmp_22071 = (int64_t) -1;
                } else {
                    // stdlib/bigint-fast.fut:15:45-73
                    
                    bool cond_22072 = ult32(zl_rhs_22069, zl_lhs_22068);
                    
                    // probes/stdlib-fast-tests.fut:6:7-8:60
                    
                    int64_t tmp_f_res_22073 = btoi_bool_i64(cond_22072);
                    
                    tmp_22071 = tmp_f_res_22073;
                }
                // stdlib/bigint-fast.fut:14:16-37
                
                bool cond_22074 = sle64((int64_t) 0, tmp_22067);
                
                // stdlib/bigint-fast.fut:14:33-37
                
                bool loop_cond_t_res_22075 = tmp_22071 == (int64_t) 0;
                
                // probes/stdlib-fast-tests.fut:6:7-8:60
                
                bool x_22076 = cond_22074 && loop_cond_t_res_22075;
                bool loop_while_tmp_22844 = x_22076;
                int64_t i_tmp_22845 = tmp_22067;
                int64_t result_tmp_22846 = tmp_22071;
                
                loop_while_22064 = loop_while_tmp_22844;
                i_22065 = i_tmp_22845;
                result_22066 = result_tmp_22846;
            }
            compare_abs_res_f_res_22061 = loop_while_22064;
            compare_abs_res_f_res_22062 = i_22065;
            compare_abs_res_f_res_22063 = result_22066;
            compare_abs_res_22058 = compare_abs_res_f_res_22063;
        }
        zl_lhs_20600 = compare_abs_res_22058;
    } else {
        zl_lhs_20600 = (int64_t) 0;
    }
    // stdlib/bigint-fast.fut:90:11-103:50
    
    bool cond_20622 = slt64(zl_lhs_20600, (int64_t) 0);
    
    // stdlib/bigint-fast.fut:90:71-101
    
    bool cond_20625 = arithmetic_res_20358 == (int64_t) 1;
    
    // stdlib/bigint-fast.fut:90:90-96
    
    bool y_20626 = slt64((int64_t) 0, arithmetic_res_20358);
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool loop_not_taken_20627 = !cond_20625;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool protect_assert_disj_20628 = y_20626 || loop_not_taken_20627;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool protect_assert_disj_20629 = cond_20364 || protect_assert_disj_20628;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool loop_not_taken_20630 = !cond_20622;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool protect_assert_disj_20631 = protect_assert_disj_20629 || loop_not_taken_20630;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool protect_assert_disj_20632 = cond_20598 || protect_assert_disj_20631;
    
    // stdlib/bigint-fast.fut:90:90-96
    
    bool index_certs_20633;
    
    if (!protect_assert_disj_20632) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) arithmetic_res_20358, "].", "-> #0  stdlib/bigint-fast.fut:90:90-96\n   #1  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:52:32-38
    
    bool y_20644 = slt64((int64_t) 0, arithmetic_res_20361);
    bool loop_not_taken_20645 = !cond_20545;
    bool protect_assert_disj_20646 = y_20644 || loop_not_taken_20645;
    bool protect_assert_disj_20647 = x_20599 || protect_assert_disj_20646;
    
    // stdlib/bigint-fast.fut:52:32-38
    
    bool index_certs_20648;
    
    if (!protect_assert_disj_20647) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) arithmetic_res_20361, "].", "-> #0  stdlib/bigint-fast.fut:52:32-38\n   #1  stdlib/bigint-fast.fut:82:15-89:66\n   #2  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    
    bool protect_cond_conj_20649 = cond_20545 && cond_20598;
    
    // stdlib/bigint-fast.fut:52:32-38
    
    int32_t zeze_lhs_20650;
    
    if (protect_cond_conj_20649) {
        int32_t x_22078 = ((int32_t *) ext_mem_22582.mem)[(int64_t) 0];
        
        zeze_lhs_20650 = x_22078;
    } else {
        zeze_lhs_20650 = 0;
    }
    // stdlib/bigint-fast.fut:52:39-43
    
    bool tmp_t_res_20652 = zeze_lhs_20650 == 0;
    bool x_20653 = cond_20545 && tmp_t_res_20652;
    
    // stdlib/bigint-fast.fut:53:40-65
    
    int64_t u32_res_20657 = zext_i32_i64(u32_arg0_20574);
    
    // stdlib/bigint-fast.fut:53:65-67
    
    int64_t zs_rhs_20658 = add64((int64_t) 1, u32_res_20657);
    
    // stdlib/bigint-fast.fut:53:38-67
    
    bool zzero_20659 = zs_rhs_20658 == (int64_t) 0;
    
    // stdlib/bigint-fast.fut:53:38-67
    
    bool nonzzero_20660 = !zzero_20659;
    bool protect_assert_disj_20661 = x_20599 || nonzzero_20660;
    
    // stdlib/bigint-fast.fut:53:38-67
    
    bool nonzzero_cert_20662;
    
    if (!protect_assert_disj_20661) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/bigint-fast.fut:53:38-67\n   #1  stdlib/bigint-fast.fut:82:15-89:66\n   #2  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:52:3-74:42
    
    bool assert_cond_20665 = !x_20653;
    bool protect_assert_disj_20666 = x_20599 || assert_cond_20665;
    
    // stdlib/bigint-fast.fut:52:3-74:42
    
    bool assert_c_20667;
    
    if (!protect_assert_disj_20666) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "Assertion is false: (!(length b.1 == 1 && b.1[0] == 0))", "-> #0  stdlib/bigint-fast.fut:52:3-74:42\n   #1  stdlib/bigint-fast.fut:82:15-89:66\n   #2  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:6:7-8:10
    
    bool x_20365 = !cond_20364;
    
    // stdlib/bigint-fast.fut:39:11-30
    
    int64_t zgze_lhs_20366;
    
    if (x_20365) {
        // stdlib/bigint-fast.fut:12:3-16:17
        
        bool cond_21920 = arithmetic_res_20358 == arithmetic_res_20361;
        
        // stdlib/bigint-fast.fut:12:3-16:17
        
        bool cond_21921 = !cond_21920;
        
        // stdlib/bigint-fast.fut:12:3-16:17
        
        int64_t compare_abs_res_21922;
        
        if (cond_21921) {
            // stdlib/bigint-fast.fut:12:32-69
            
            bool cond_21923 = slt64(arithmetic_res_20358, arithmetic_res_20361);
            
            // stdlib/bigint-fast.fut:12:32-69
            
            int64_t compare_abs_res_t_res_21924;
            
            if (cond_21923) {
                compare_abs_res_t_res_21924 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_21924 = (int64_t) 1;
            }
            compare_abs_res_21922 = compare_abs_res_t_res_21924;
        } else {
            // stdlib/bigint-fast.fut:13:36-15:74
            
            bool compare_abs_res_f_res_21927;
            int64_t compare_abs_res_f_res_21928;
            int64_t compare_abs_res_f_res_21929;
            bool loop_while_21930;
            int64_t i_21931;
            int64_t result_21932;
            
            loop_while_21930 = x_20552;
            i_21931 = u32_arg0_20551;
            result_21932 = (int64_t) 0;
            while (loop_while_21930) {
                // stdlib/bigint-fast.fut:15:14-16
                
                int64_t tmp_21933 = sub64(i_21931, (int64_t) 1);
                
                // stdlib/bigint-fast.fut:15:20-24
                
                int32_t zl_lhs_21934 = ((int32_t *) ext_mem_22581.mem)[i_21931];
                
                // stdlib/bigint-fast.fut:15:27-31
                
                int32_t zl_rhs_21935 = ((int32_t *) ext_mem_22582.mem)[i_21931];
                
                // stdlib/bigint-fast.fut:15:17-73
                
                bool cond_21936 = ult32(zl_lhs_21934, zl_rhs_21935);
                
                // stdlib/bigint-fast.fut:15:17-73
                
                int64_t tmp_21937;
                
                if (cond_21936) {
                    tmp_21937 = (int64_t) -1;
                } else {
                    // stdlib/bigint-fast.fut:15:45-73
                    
                    bool cond_21938 = ult32(zl_rhs_21935, zl_lhs_21934);
                    
                    // probes/stdlib-fast-tests.fut:6:7-8:10
                    
                    int64_t tmp_f_res_21939 = btoi_bool_i64(cond_21938);
                    
                    tmp_21937 = tmp_f_res_21939;
                }
                // stdlib/bigint-fast.fut:14:16-37
                
                bool cond_21940 = sle64((int64_t) 0, tmp_21933);
                
                // stdlib/bigint-fast.fut:14:33-37
                
                bool loop_cond_t_res_21941 = tmp_21937 == (int64_t) 0;
                
                // probes/stdlib-fast-tests.fut:6:7-8:10
                
                bool x_21942 = cond_21940 && loop_cond_t_res_21941;
                bool loop_while_tmp_22847 = x_21942;
                int64_t i_tmp_22848 = tmp_21933;
                int64_t result_tmp_22849 = tmp_21937;
                
                loop_while_21930 = loop_while_tmp_22847;
                i_21931 = i_tmp_22848;
                result_21932 = result_tmp_22849;
            }
            compare_abs_res_f_res_21927 = loop_while_21930;
            compare_abs_res_f_res_21928 = i_21931;
            compare_abs_res_f_res_21929 = result_21932;
            compare_abs_res_21922 = compare_abs_res_f_res_21929;
        }
        zgze_lhs_20366 = compare_abs_res_21922;
    } else {
        zgze_lhs_20366 = (int64_t) 0;
    }
    // stdlib/bigint-fast.fut:39:8-40:39
    
    bool cond_20390 = sle64((int64_t) 0, zgze_lhs_20366);
    int64_t bytes_22583 = (int64_t) 4 * arithmetic_res_20361;
    int64_t bytes_22595 = (int64_t) 4 * arithmetic_res_20358;
    
    // stdlib/bigint-fast.fut:38:3-40:39
    
    int64_t add_res_20391;
    bool add_res_20392;
    
    if (cond_20364) {
        // stdlib/bigint-fast.fut:38:37-52
        
        int64_t normalise_arg1_21943;
        
        if (futrts_add_abs_11626(ctx, &ext_mem_22608, &normalise_arg1_21943, ext_mem_22581, ext_mem_22582, arithmetic_res_20358, arithmetic_res_20361) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:38:22-52
        
        int64_t add_res_t_res_21945;
        bool add_res_t_res_21946;
        
        if (futrts_normalise_11622(ctx, &ext_mem_22609, &add_res_t_res_21945, &add_res_t_res_21946, ext_mem_22608, normalise_arg1_21943, arithmetic_res_20359) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_22608, "ext_mem_22608") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_22610, &ext_mem_22609, "ext_mem_22609") != 0)
            return 1;
        add_res_20391 = add_res_t_res_21945;
        add_res_20392 = add_res_t_res_21946;
    } else {
        // stdlib/bigint-fast.fut:39:8-40:39
        
        int64_t add_res_f_res_20399;
        bool add_res_f_res_20400;
        
        if (cond_20390) {
            if (memblock_alloc(ctx, &mem_22596, bytes_22595, "mem_22596")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:31:54-79
            for (int64_t nest_i_22850 = 0; nest_i_22850 < arithmetic_res_20358; nest_i_22850++) {
                ((int32_t *) mem_22596.mem)[nest_i_22850] = 0;
            }
            // stdlib/bigint-fast.fut:31:33-35:39
            
            int64_t sub_abs_res_21952;
            int64_t borrow_21955 = (int64_t) 0;
            
            for (int64_t i_21953 = 0; i_21953 < arithmetic_res_20358; i_21953++) {
                // stdlib/bigint-fast.fut:32:21-25
                
                int32_t u32_arg0_21956 = ((int32_t *) ext_mem_22581.mem)[i_21953];
                
                // stdlib/bigint-fast.fut:32:13-25
                
                int64_t u32_res_21957 = zext_i32_i64(u32_arg0_21956);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                bool cond_21958 = slt64(i_21953, arithmetic_res_20361);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                int64_t zp_lhs_21959;
                
                if (cond_21958) {
                    // stdlib/bigint-fast.fut:33:43-47
                    
                    int32_t u32_arg0_21960 = ((int32_t *) ext_mem_22582.mem)[i_21953];
                    
                    // stdlib/bigint-fast.fut:33:35-47
                    
                    int64_t u32_res_21961 = zext_i32_i64(u32_arg0_21960);
                    
                    zp_lhs_21959 = u32_res_21961;
                } else {
                    zp_lhs_21959 = (int64_t) 0;
                }
                // stdlib/bigint-fast.fut:33:55-62
                
                int64_t y_21962 = add64(borrow_21955, zp_lhs_21959);
                
                // stdlib/bigint-fast.fut:34:28-30
                
                int64_t u64_arg0_21963 = sub64(u32_res_21957, y_21962);
                
                // stdlib/bigint-fast.fut:34:18-30
                
                int32_t u64_res_21964 = zext_i64_i32(u64_arg0_21963);
                
                // stdlib/bigint-fast.fut:34:9-31
                ((int32_t *) mem_22596.mem)[i_21953] = u64_res_21964;
                // stdlib/bigint-fast.fut:35:13-38
                
                bool cond_21966 = ult64(u32_res_21957, y_21962);
                
                // stdlib/bigint-fast.fut:39:56-71
                
                int64_t tmp_21967 = btoi_bool_i64(cond_21966);
                int64_t borrow_tmp_22852 = tmp_21967;
                
                borrow_21955 = borrow_tmp_22852;
            }
            sub_abs_res_21952 = borrow_21955;
            // stdlib/bigint-fast.fut:36:7-26
            
            int64_t sub_abs_res_21968;
            bool sub_abs_res_21969;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22605, &sub_abs_res_21968, &sub_abs_res_21969, mem_22596, arithmetic_res_20358, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_22596, "mem_22596") != 0)
                return 1;
            // stdlib/bigint-fast.fut:39:41-71
            
            int64_t add_res_f_res_t_res_21971;
            bool add_res_f_res_t_res_21972;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22606, &add_res_f_res_t_res_21971, &add_res_f_res_t_res_21972, ext_mem_22605, sub_abs_res_21968, arithmetic_res_20359) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_22605, "ext_mem_22605") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22607, &ext_mem_22606, "ext_mem_22606") != 0)
                return 1;
            add_res_f_res_20399 = add_res_f_res_t_res_21971;
            add_res_f_res_20400 = add_res_f_res_t_res_21972;
        } else {
            if (memblock_alloc(ctx, &mem_22584, bytes_22583, "mem_22584")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:31:54-79
            for (int64_t nest_i_22853 = 0; nest_i_22853 < arithmetic_res_20361; nest_i_22853++) {
                ((int32_t *) mem_22584.mem)[nest_i_22853] = 0;
            }
            // stdlib/bigint-fast.fut:31:33-35:39
            
            int64_t sub_abs_res_20428;
            int64_t borrow_20431 = (int64_t) 0;
            
            for (int64_t i_20429 = 0; i_20429 < arithmetic_res_20361; i_20429++) {
                // stdlib/bigint-fast.fut:32:21-25
                
                int32_t u32_arg0_20432 = ((int32_t *) ext_mem_22582.mem)[i_20429];
                
                // stdlib/bigint-fast.fut:32:13-25
                
                int64_t u32_res_20433 = zext_i32_i64(u32_arg0_20432);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                bool cond_20434 = slt64(i_20429, arithmetic_res_20358);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                int64_t zp_lhs_20435;
                
                if (cond_20434) {
                    // stdlib/bigint-fast.fut:33:43-47
                    
                    int32_t u32_arg0_21974 = ((int32_t *) ext_mem_22581.mem)[i_20429];
                    
                    // stdlib/bigint-fast.fut:33:35-47
                    
                    int64_t u32_res_21975 = zext_i32_i64(u32_arg0_21974);
                    
                    zp_lhs_20435 = u32_res_21975;
                } else {
                    zp_lhs_20435 = (int64_t) 0;
                }
                // stdlib/bigint-fast.fut:33:55-62
                
                int64_t y_20438 = add64(borrow_20431, zp_lhs_20435);
                
                // stdlib/bigint-fast.fut:34:28-30
                
                int64_t u64_arg0_20439 = sub64(u32_res_20433, y_20438);
                
                // stdlib/bigint-fast.fut:34:18-30
                
                int32_t u64_res_20440 = zext_i64_i32(u64_arg0_20439);
                
                // stdlib/bigint-fast.fut:34:9-31
                ((int32_t *) mem_22584.mem)[i_20429] = u64_res_20440;
                // stdlib/bigint-fast.fut:35:13-38
                
                bool cond_20442 = ult64(u32_res_20433, y_20438);
                
                // stdlib/bigint-fast.fut:40:23-38
                
                int64_t tmp_20443 = btoi_bool_i64(cond_20442);
                int64_t borrow_tmp_22855 = tmp_20443;
                
                borrow_20431 = borrow_tmp_22855;
            }
            sub_abs_res_20428 = borrow_20431;
            // stdlib/bigint-fast.fut:36:7-26
            
            int64_t sub_abs_res_20444;
            bool sub_abs_res_20445;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22593, &sub_abs_res_20444, &sub_abs_res_20445, mem_22584, arithmetic_res_20361, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_22584, "mem_22584") != 0)
                return 1;
            // stdlib/bigint-fast.fut:40:8-38
            
            int64_t add_res_f_res_f_res_20447;
            bool add_res_f_res_f_res_20448;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22594, &add_res_f_res_f_res_20447, &add_res_f_res_f_res_20448, ext_mem_22593, sub_abs_res_20444, arithmetic_res_20362) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_22593, "ext_mem_22593") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22607, &ext_mem_22594, "ext_mem_22594") != 0)
                return 1;
            add_res_f_res_20399 = add_res_f_res_f_res_20447;
            add_res_f_res_20400 = add_res_f_res_f_res_20448;
        }
        if (memblock_set(ctx, &ext_mem_22610, &ext_mem_22607, "ext_mem_22607") != 0)
            return 1;
        add_res_20391 = add_res_f_res_20399;
        add_res_20392 = add_res_f_res_20400;
    }
    // stdlib/bigint-fast.fut:41:34-54
    
    bool normalise_arg_20450 = !arithmetic_res_20362;
    
    // stdlib/bigint-fast.fut:41:34-54
    
    int64_t negate_res_20451;
    bool negate_res_20452;
    
    if (futrts_normalise_11622(ctx, &ext_mem_22611, &negate_res_20451, &negate_res_20452, ext_mem_22582, arithmetic_res_20361, normalise_arg_20450) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:38:3-40:39
    
    bool cond_20454 = arithmetic_res_20359 == negate_res_20452;
    
    // stdlib/bigint-fast.fut:42:15-63
    
    bool x_20455 = !cond_20454;
    
    // stdlib/bigint-fast.fut:39:11-30
    
    int64_t zgze_lhs_20456;
    
    if (x_20455) {
        // stdlib/bigint-fast.fut:12:3-16:17
        
        bool cond_21978 = arithmetic_res_20358 == negate_res_20451;
        
        // stdlib/bigint-fast.fut:12:3-16:17
        
        bool cond_21979 = !cond_21978;
        
        // stdlib/bigint-fast.fut:12:3-16:17
        
        int64_t compare_abs_res_21980;
        
        if (cond_21979) {
            // stdlib/bigint-fast.fut:12:32-69
            
            bool cond_21981 = slt64(arithmetic_res_20358, negate_res_20451);
            
            // stdlib/bigint-fast.fut:12:32-69
            
            int64_t compare_abs_res_t_res_21982;
            
            if (cond_21981) {
                compare_abs_res_t_res_21982 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_21982 = (int64_t) 1;
            }
            compare_abs_res_21980 = compare_abs_res_t_res_21982;
        } else {
            // stdlib/bigint-fast.fut:13:36-15:74
            
            bool compare_abs_res_f_res_21985;
            int64_t compare_abs_res_f_res_21986;
            int64_t compare_abs_res_f_res_21987;
            bool loop_while_21988;
            int64_t i_21989;
            int64_t result_21990;
            
            loop_while_21988 = x_20552;
            i_21989 = u32_arg0_20551;
            result_21990 = (int64_t) 0;
            while (loop_while_21988) {
                // stdlib/bigint-fast.fut:15:14-16
                
                int64_t tmp_21991 = sub64(i_21989, (int64_t) 1);
                
                // stdlib/bigint-fast.fut:15:20-24
                
                int32_t zl_lhs_21992 = ((int32_t *) ext_mem_22581.mem)[i_21989];
                
                // stdlib/bigint-fast.fut:15:27-31
                
                int32_t zl_rhs_21993 = ((int32_t *) ext_mem_22611.mem)[i_21989];
                
                // stdlib/bigint-fast.fut:15:17-73
                
                bool cond_21994 = ult32(zl_lhs_21992, zl_rhs_21993);
                
                // stdlib/bigint-fast.fut:15:17-73
                
                int64_t tmp_21995;
                
                if (cond_21994) {
                    tmp_21995 = (int64_t) -1;
                } else {
                    // stdlib/bigint-fast.fut:15:45-73
                    
                    bool cond_21996 = ult32(zl_rhs_21993, zl_lhs_21992);
                    
                    // stdlib/bigint-fast.fut:42:15-63
                    
                    int64_t tmp_f_res_21997 = btoi_bool_i64(cond_21996);
                    
                    tmp_21995 = tmp_f_res_21997;
                }
                // stdlib/bigint-fast.fut:14:16-37
                
                bool cond_21998 = sle64((int64_t) 0, tmp_21991);
                
                // stdlib/bigint-fast.fut:14:33-37
                
                bool loop_cond_t_res_21999 = tmp_21995 == (int64_t) 0;
                
                // stdlib/bigint-fast.fut:42:15-63
                
                bool x_22000 = cond_21998 && loop_cond_t_res_21999;
                bool loop_while_tmp_22856 = x_22000;
                int64_t i_tmp_22857 = tmp_21991;
                int64_t result_tmp_22858 = tmp_21995;
                
                loop_while_21988 = loop_while_tmp_22856;
                i_21989 = i_tmp_22857;
                result_21990 = result_tmp_22858;
            }
            compare_abs_res_f_res_21985 = loop_while_21988;
            compare_abs_res_f_res_21986 = i_21989;
            compare_abs_res_f_res_21987 = result_21990;
            compare_abs_res_21980 = compare_abs_res_f_res_21987;
        }
        zgze_lhs_20456 = compare_abs_res_21980;
    } else {
        zgze_lhs_20456 = (int64_t) 0;
    }
    // stdlib/bigint-fast.fut:39:8-40:39
    
    bool cond_20480 = sle64((int64_t) 0, zgze_lhs_20456);
    int64_t bytes_22612 = (int64_t) 4 * negate_res_20451;
    
    // stdlib/bigint-fast.fut:38:3-40:39
    
    int64_t add_res_20481;
    bool add_res_20482;
    
    if (cond_20454) {
        // stdlib/bigint-fast.fut:38:37-52
        
        int64_t normalise_arg1_22001;
        
        if (futrts_add_abs_11626(ctx, &ext_mem_22637, &normalise_arg1_22001, ext_mem_22581, ext_mem_22611, arithmetic_res_20358, negate_res_20451) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:38:22-52
        
        int64_t add_res_t_res_22003;
        bool add_res_t_res_22004;
        
        if (futrts_normalise_11622(ctx, &ext_mem_22638, &add_res_t_res_22003, &add_res_t_res_22004, ext_mem_22637, normalise_arg1_22001, arithmetic_res_20359) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_22637, "ext_mem_22637") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_22639, &ext_mem_22638, "ext_mem_22638") != 0)
            return 1;
        add_res_20481 = add_res_t_res_22003;
        add_res_20482 = add_res_t_res_22004;
    } else {
        // stdlib/bigint-fast.fut:39:8-40:39
        
        int64_t add_res_f_res_20489;
        bool add_res_f_res_20490;
        
        if (cond_20480) {
            if (memblock_alloc(ctx, &mem_22625, bytes_22595, "mem_22625")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:31:54-79
            for (int64_t nest_i_22859 = 0; nest_i_22859 < arithmetic_res_20358; nest_i_22859++) {
                ((int32_t *) mem_22625.mem)[nest_i_22859] = 0;
            }
            // stdlib/bigint-fast.fut:31:33-35:39
            
            int64_t sub_abs_res_22010;
            int64_t borrow_22013 = (int64_t) 0;
            
            for (int64_t i_22011 = 0; i_22011 < arithmetic_res_20358; i_22011++) {
                // stdlib/bigint-fast.fut:32:21-25
                
                int32_t u32_arg0_22014 = ((int32_t *) ext_mem_22581.mem)[i_22011];
                
                // stdlib/bigint-fast.fut:32:13-25
                
                int64_t u32_res_22015 = zext_i32_i64(u32_arg0_22014);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                bool cond_22016 = slt64(i_22011, negate_res_20451);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                int64_t zp_lhs_22017;
                
                if (cond_22016) {
                    // stdlib/bigint-fast.fut:33:43-47
                    
                    int32_t u32_arg0_22018 = ((int32_t *) ext_mem_22611.mem)[i_22011];
                    
                    // stdlib/bigint-fast.fut:33:35-47
                    
                    int64_t u32_res_22019 = zext_i32_i64(u32_arg0_22018);
                    
                    zp_lhs_22017 = u32_res_22019;
                } else {
                    zp_lhs_22017 = (int64_t) 0;
                }
                // stdlib/bigint-fast.fut:33:55-62
                
                int64_t y_22020 = add64(borrow_22013, zp_lhs_22017);
                
                // stdlib/bigint-fast.fut:34:28-30
                
                int64_t u64_arg0_22021 = sub64(u32_res_22015, y_22020);
                
                // stdlib/bigint-fast.fut:34:18-30
                
                int32_t u64_res_22022 = zext_i64_i32(u64_arg0_22021);
                
                // stdlib/bigint-fast.fut:34:9-31
                ((int32_t *) mem_22625.mem)[i_22011] = u64_res_22022;
                // stdlib/bigint-fast.fut:35:13-38
                
                bool cond_22024 = ult64(u32_res_22015, y_22020);
                
                // stdlib/bigint-fast.fut:39:56-71
                
                int64_t tmp_22025 = btoi_bool_i64(cond_22024);
                int64_t borrow_tmp_22861 = tmp_22025;
                
                borrow_22013 = borrow_tmp_22861;
            }
            sub_abs_res_22010 = borrow_22013;
            // stdlib/bigint-fast.fut:36:7-26
            
            int64_t sub_abs_res_22026;
            bool sub_abs_res_22027;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22634, &sub_abs_res_22026, &sub_abs_res_22027, mem_22625, arithmetic_res_20358, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_22625, "mem_22625") != 0)
                return 1;
            // stdlib/bigint-fast.fut:39:41-71
            
            int64_t add_res_f_res_t_res_22029;
            bool add_res_f_res_t_res_22030;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22635, &add_res_f_res_t_res_22029, &add_res_f_res_t_res_22030, ext_mem_22634, sub_abs_res_22026, arithmetic_res_20359) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_22634, "ext_mem_22634") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22636, &ext_mem_22635, "ext_mem_22635") != 0)
                return 1;
            add_res_f_res_20489 = add_res_f_res_t_res_22029;
            add_res_f_res_20490 = add_res_f_res_t_res_22030;
        } else {
            if (memblock_alloc(ctx, &mem_22613, bytes_22612, "mem_22613")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:31:54-79
            for (int64_t nest_i_22862 = 0; nest_i_22862 < negate_res_20451; nest_i_22862++) {
                ((int32_t *) mem_22613.mem)[nest_i_22862] = 0;
            }
            // stdlib/bigint-fast.fut:31:33-35:39
            
            int64_t sub_abs_res_20518;
            int64_t borrow_20521 = (int64_t) 0;
            
            for (int64_t i_20519 = 0; i_20519 < negate_res_20451; i_20519++) {
                // stdlib/bigint-fast.fut:32:21-25
                
                int32_t u32_arg0_20522 = ((int32_t *) ext_mem_22611.mem)[i_20519];
                
                // stdlib/bigint-fast.fut:32:13-25
                
                int64_t u32_res_20523 = zext_i32_i64(u32_arg0_20522);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                bool cond_20524 = slt64(i_20519, arithmetic_res_20358);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                int64_t zp_lhs_20525;
                
                if (cond_20524) {
                    // stdlib/bigint-fast.fut:33:43-47
                    
                    int32_t u32_arg0_22032 = ((int32_t *) ext_mem_22581.mem)[i_20519];
                    
                    // stdlib/bigint-fast.fut:33:35-47
                    
                    int64_t u32_res_22033 = zext_i32_i64(u32_arg0_22032);
                    
                    zp_lhs_20525 = u32_res_22033;
                } else {
                    zp_lhs_20525 = (int64_t) 0;
                }
                // stdlib/bigint-fast.fut:33:55-62
                
                int64_t y_20528 = add64(borrow_20521, zp_lhs_20525);
                
                // stdlib/bigint-fast.fut:34:28-30
                
                int64_t u64_arg0_20529 = sub64(u32_res_20523, y_20528);
                
                // stdlib/bigint-fast.fut:34:18-30
                
                int32_t u64_res_20530 = zext_i64_i32(u64_arg0_20529);
                
                // stdlib/bigint-fast.fut:34:9-31
                ((int32_t *) mem_22613.mem)[i_20519] = u64_res_20530;
                // stdlib/bigint-fast.fut:35:13-38
                
                bool cond_20532 = ult64(u32_res_20523, y_20528);
                
                // stdlib/bigint-fast.fut:40:23-38
                
                int64_t tmp_20533 = btoi_bool_i64(cond_20532);
                int64_t borrow_tmp_22864 = tmp_20533;
                
                borrow_20521 = borrow_tmp_22864;
            }
            sub_abs_res_20518 = borrow_20521;
            // stdlib/bigint-fast.fut:36:7-26
            
            int64_t sub_abs_res_20534;
            bool sub_abs_res_20535;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22622, &sub_abs_res_20534, &sub_abs_res_20535, mem_22613, negate_res_20451, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_22613, "mem_22613") != 0)
                return 1;
            // stdlib/bigint-fast.fut:40:8-38
            
            int64_t add_res_f_res_f_res_20537;
            bool add_res_f_res_f_res_20538;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22623, &add_res_f_res_f_res_20537, &add_res_f_res_f_res_20538, ext_mem_22622, sub_abs_res_20534, negate_res_20452) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_22622, "ext_mem_22622") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22636, &ext_mem_22623, "ext_mem_22623") != 0)
                return 1;
            add_res_f_res_20489 = add_res_f_res_f_res_20537;
            add_res_f_res_20490 = add_res_f_res_f_res_20538;
        }
        if (memblock_set(ctx, &ext_mem_22639, &ext_mem_22636, "ext_mem_22636") != 0)
            return 1;
        add_res_20481 = add_res_f_res_20489;
        add_res_20482 = add_res_f_res_20490;
    }
    if (memblock_unref(ctx, &ext_mem_22611, "ext_mem_22611") != 0)
        return 1;
    // stdlib/bigint-fast.fut:50:65-78
    
    int64_t normalise_arg1_20540;
    
    if (futrts_mul_abs_11639(ctx, &ext_mem_22640, &normalise_arg1_20540, ext_mem_22581, arithmetic_res_20358, factor_15872) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:50:50-78
    
    int64_t multiply_small_res_20542;
    bool multiply_small_res_20543;
    
    if (futrts_normalise_11622(ctx, &ext_mem_22641, &multiply_small_res_20542, &multiply_small_res_20543, ext_mem_22640, normalise_arg1_20540, arithmetic_res_20359) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &ext_mem_22640, "ext_mem_22640") != 0)
        return 1;
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    int64_t f64_res_20623 = fptosi_f64_i64(estimate_20591);
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    int32_t unsign_arg0_20624 = zext_i64_i32(f64_res_20623);
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool protect_cond_conj_20634 = x_20365 && cond_20625;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool protect_cond_conj_20635 = cond_20622 && protect_cond_conj_20634;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool protect_cond_conj_20636 = x_20599 && protect_cond_conj_20635;
    
    // stdlib/bigint-fast.fut:90:90-96
    
    int32_t zeze_lhs_20637;
    
    if (protect_cond_conj_20636) {
        // probes/stdlib-fast-tests.fut:6:7-8:60
        
        int32_t x_22077 = ((int32_t *) ext_mem_22581.mem)[(int64_t) 0];
        
        zeze_lhs_20637 = x_22077;
    } else {
        zeze_lhs_20637 = 0;
    }
    // stdlib/bigint-fast.fut:90:97-101
    
    bool tmp_t_res_20639 = zeze_lhs_20637 == 0;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool x_20640 = cond_20625 && tmp_t_res_20639;
    
    // stdlib/bigint-fast.fut:90:69-102
    
    bool cond_t_res_20641 = !x_20640;
    
    // probes/stdlib-fast-tests.fut:6:7-8:60
    
    bool x_20642 = x_20365 && cond_t_res_20641;
    
    // stdlib/bigint-fast.fut:90:52-117
    
    int64_t integer_arg0_20643;
    
    if (x_20642) {
        integer_arg0_20643 = (int64_t) -1;
    } else {
        integer_arg0_20643 = (int64_t) 0;
    }
    // stdlib/bigint-fast.fut:53:38-67
    
    int64_t u64_arg0_20663 = udiv_safe64((int64_t) 4294967296, zs_rhs_20658);
    
    // stdlib/bigint-fast.fut:53:16-67
    
    int32_t u64_res_20664 = zext_i64_i32(u64_arg0_20663);
    
    // stdlib/bigint-fast.fut:83:3-103:50
    
    int64_t quotient_res_20668;
    bool quotient_res_20669;
    
    if (cond_20598) {
        // stdlib/bigint-fast.fut:54:19-37
        
        int64_t numerator_22172;
        
        if (futrts_mul_abs_11639(ctx, &ext_mem_22721, &numerator_22172, ext_mem_22581, arithmetic_res_20358, u64_res_20664) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:55:17-35
        
        int64_t divisor_22174;
        
        if (futrts_mul_abs_11639(ctx, &ext_mem_22722, &divisor_22174, ext_mem_22582, arithmetic_res_20361, u64_res_20664) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:56:42-57
        
        int64_t zp_lhs_22176 = sub64(numerator_22172, divisor_22174);
        
        // stdlib/bigint-fast.fut:56:57-59
        
        int64_t max_arg1_22177 = add64((int64_t) 1, zp_lhs_22176);
        
        // stdlib/bigint-fast.fut:56:15-59
        
        int64_t max_res_22178 = smax64((int64_t) 1, max_arg1_22177);
        int64_t bytes_22723 = (int64_t) 4 * max_res_22178;
        
        if (memblock_alloc(ctx, &mem_22724, bytes_22723, "mem_22724")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:57:47-67
        for (int64_t nest_i_22865 = 0; nest_i_22865 < max_res_22178; nest_i_22865++) {
            ((int32_t *) mem_22724.mem)[nest_i_22865] = 0;
        }
        // stdlib/bigint-fast.fut:57:33-69:15
        
        int64_t w_minus_1_22180 = sub64(max_res_22178, (int64_t) 1);
        
        // stdlib/bigint-fast.fut:82:15-89:66
        
        bool loop_nonempty_22181 = slt64((int64_t) 0, max_res_22178);
        
        // stdlib/bigint-fast.fut:62:103-105
        
        int64_t u32_arg0_22182 = sub64(divisor_22174, (int64_t) 1);
        
        // stdlib/bigint-fast.fut:62:81-106
        
        int32_t u32_arg0_22183;
        
        if (loop_nonempty_22181) {
            // stdlib/bigint-fast.fut:82:15-89:66
            
            int32_t x_22184 = ((int32_t *) ext_mem_22722.mem)[u32_arg0_22182];
            
            u32_arg0_22183 = x_22184;
        } else {
            u32_arg0_22183 = 0;
        }
        // stdlib/bigint-fast.fut:82:15-89:66
        
        int64_t u32_res_22185 = zext_i32_i64(u32_arg0_22183);
        
        // stdlib/bigint-fast.fut:57:33-69:15
        
        int64_t quotient_long_res_22186;
        int64_t loop_dz2081Uz2080U_22190;
        
        if (memblock_set(ctx, &mem_param_22730, &ext_mem_22721, "ext_mem_22721") != 0)
            return 1;
        loop_dz2081Uz2080U_22190 = numerator_22172;
        for (int64_t i_22189 = 0; i_22189 < max_res_22178; i_22189++) {
            // stdlib/bigint-fast.fut:82:15-89:66
            
            int64_t binop_x_22193 = w_minus_1_22180 - i_22189;
            
            // stdlib/bigint-fast.fut:59:38-44
            
            int64_t high_index_22194 = add64(divisor_22174, binop_x_22193);
            
            // stdlib/bigint-fast.fut:60:18-76
            
            bool cond_22195 = slt64(high_index_22194, loop_dz2081Uz2080U_22190);
            
            // stdlib/bigint-fast.fut:60:56-69
            
            int32_t u32_arg0_22196;
            
            if (cond_22195) {
                // stdlib/bigint-fast.fut:82:15-89:66
                
                int32_t x_22197 = ((int32_t *) mem_param_22730.mem)[high_index_22194];
                
                u32_arg0_22196 = x_22197;
            } else {
                u32_arg0_22196 = 0;
            }
            // stdlib/bigint-fast.fut:60:48-69
            
            int64_t u32_res_22198 = zext_i32_i64(u32_arg0_22196);
            
            // stdlib/bigint-fast.fut:60:18-76
            
            int64_t high_22199;
            
            if (cond_22195) {
                high_22199 = u32_res_22198;
            } else {
                high_22199 = (int64_t) 0;
            }
            // stdlib/bigint-fast.fut:61:30-32
            
            int64_t zl_lhs_22200 = sub64(high_index_22194, (int64_t) 1);
            
            // stdlib/bigint-fast.fut:61:17-79
            
            bool cond_22201 = slt64(zl_lhs_22200, loop_dz2081Uz2080U_22190);
            
            // stdlib/bigint-fast.fut:61:57-72
            
            int32_t u32_arg0_22202;
            
            if (cond_22201) {
                // stdlib/bigint-fast.fut:82:15-89:66
                
                int32_t x_22203 = ((int32_t *) mem_param_22730.mem)[zl_lhs_22200];
                
                u32_arg0_22202 = x_22203;
            } else {
                u32_arg0_22202 = 0;
            }
            // stdlib/bigint-fast.fut:61:49-72
            
            int64_t u32_res_22204 = zext_i32_i64(u32_arg0_22202);
            
            // stdlib/bigint-fast.fut:61:17-79
            
            int64_t low_22205;
            
            if (cond_22201) {
                low_22205 = u32_res_22204;
            } else {
                low_22205 = (int64_t) 0;
            }
            // stdlib/bigint-fast.fut:62:61-66
            
            int64_t zb_lhs_22206 = shl64(high_22199, (int64_t) 32);
            
            // stdlib/bigint-fast.fut:62:67-71
            
            int64_t zs_lhs_22207 = low_22205 | zb_lhs_22206;
            
            // stdlib/bigint-fast.fut:62:72-106
            
            int64_t min_arg1_22208 = udiv64(zs_lhs_22207, u32_res_22185);
            
            // stdlib/bigint-fast.fut:62:31-106
            
            int64_t unsign_arg0_22209 = umin64((int64_t) 4294967295, min_arg1_22208);
            
            // stdlib/bigint-fast.fut:62:22-107
            
            int32_t u64_res_22210 = zext_i64_i32(unsign_arg0_22209);
            
            // stdlib/bigint-fast.fut:64:63-87
            
            int64_t zpzp_rhs_22212;
            
            if (futrts_mul_abs_11639(ctx, &ext_mem_22733, &zpzp_rhs_22212, ext_mem_22722, divisor_22174, u64_res_22210) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:64:39-87
            
            int64_t conc_tmp_22214 = binop_x_22193 + zpzp_rhs_22212;
            int64_t bytes_22734 = (int64_t) 4 * conc_tmp_22214;
            int64_t bytes_22754 = (int64_t) 4 * loop_dz2081Uz2080U_22190;
            
            if (memblock_alloc(ctx, &mem_22735, bytes_22734, "mem_22735")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:63:21-41
            for (int64_t nest_i_22870 = 0; nest_i_22870 < binop_x_22193; nest_i_22870++) {
                ((int32_t *) mem_22735.mem)[nest_i_22870] = 0;
            }
            // stdlib/bigint-fast.fut:64:39-87
            
            int64_t tmp_offs_22871 = (int64_t) 0;
            
            if (!(tmp_offs_22871 == (int64_t) 0)) {
                // stdlib/bigint-fast.fut:64:39-87
                lmad_copy_4b(ctx, 1, (uint32_t *) mem_22735.mem, tmp_offs_22871, (int64_t []) {(int64_t) 1}, (uint32_t *) mem_22735.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {binop_x_22193});
            }
            tmp_offs_22871 += binop_x_22193;
            // stdlib/bigint-fast.fut:64:39-87
            lmad_copy_4b(ctx, 1, (uint32_t *) mem_22735.mem, tmp_offs_22871, (int64_t []) {(int64_t) 1}, (uint32_t *) ext_mem_22733.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {zpzp_rhs_22212});
            tmp_offs_22871 += zpzp_rhs_22212;
            if (memblock_unref(ctx, &ext_mem_22733, "ext_mem_22733") != 0)
                return 1;
            // stdlib/bigint-fast.fut:64:22-87
            
            int64_t loopres_22216;
            bool loopres_22217;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22736, &loopres_22216, &loopres_22217, mem_22735, conc_tmp_22214, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_22735, "mem_22735") != 0)
                return 1;
            // stdlib/bigint-fast.fut:12:3-16:17
            
            bool cond_22219 = loopres_22216 == loop_dz2081Uz2080U_22190;
            
            // stdlib/bigint-fast.fut:12:3-16:17
            
            bool cond_22220 = !cond_22219;
            
            // stdlib/bigint-fast.fut:12:3-16:17
            
            int64_t compare_abs_res_22221;
            
            if (cond_22220) {
                // stdlib/bigint-fast.fut:12:32-69
                
                bool cond_22222 = slt64(loopres_22216, loop_dz2081Uz2080U_22190);
                
                // stdlib/bigint-fast.fut:12:32-69
                
                int64_t compare_abs_res_t_res_22223;
                
                if (cond_22222) {
                    compare_abs_res_t_res_22223 = (int64_t) -1;
                } else {
                    compare_abs_res_t_res_22223 = (int64_t) 1;
                }
                compare_abs_res_22221 = compare_abs_res_t_res_22223;
            } else {
                // stdlib/bigint-fast.fut:13:63-65
                
                int64_t tmp_22224 = sub64(loopres_22216, (int64_t) 1);
                
                // stdlib/bigint-fast.fut:13:36-15:74
                
                bool cond_22225 = sle64((int64_t) 0, tmp_22224);
                
                // stdlib/bigint-fast.fut:13:36-15:74
                
                bool compare_abs_res_f_res_22226;
                int64_t compare_abs_res_f_res_22227;
                int64_t compare_abs_res_f_res_22228;
                bool loop_while_22229;
                int64_t i_22230;
                int64_t result_22231;
                
                loop_while_22229 = cond_22225;
                i_22230 = tmp_22224;
                result_22231 = (int64_t) 0;
                while (loop_while_22229) {
                    // stdlib/bigint-fast.fut:15:14-16
                    
                    int64_t tmp_22232 = sub64(i_22230, (int64_t) 1);
                    
                    // stdlib/bigint-fast.fut:15:20-24
                    
                    int32_t zl_lhs_22233 = ((int32_t *) ext_mem_22736.mem)[i_22230];
                    
                    // stdlib/bigint-fast.fut:15:27-31
                    
                    int32_t zl_rhs_22234 = ((int32_t *) mem_param_22730.mem)[i_22230];
                    
                    // stdlib/bigint-fast.fut:15:17-73
                    
                    bool cond_22235 = ult32(zl_lhs_22233, zl_rhs_22234);
                    
                    // stdlib/bigint-fast.fut:15:17-73
                    
                    int64_t tmp_22236;
                    
                    if (cond_22235) {
                        tmp_22236 = (int64_t) -1;
                    } else {
                        // stdlib/bigint-fast.fut:15:45-73
                        
                        bool cond_22237 = ult32(zl_rhs_22234, zl_lhs_22233);
                        
                        // stdlib/bigint-fast.fut:65:39-66:77
                        
                        int64_t tmp_f_res_22238 = btoi_bool_i64(cond_22237);
                        
                        tmp_22236 = tmp_f_res_22238;
                    }
                    // stdlib/bigint-fast.fut:14:16-37
                    
                    bool cond_22239 = sle64((int64_t) 0, tmp_22232);
                    
                    // stdlib/bigint-fast.fut:14:33-37
                    
                    bool loop_cond_t_res_22240 = tmp_22236 == (int64_t) 0;
                    
                    // stdlib/bigint-fast.fut:65:39-66:77
                    
                    bool x_22241 = cond_22239 && loop_cond_t_res_22240;
                    bool loop_while_tmp_22872 = x_22241;
                    int64_t i_tmp_22873 = tmp_22232;
                    int64_t result_tmp_22874 = tmp_22236;
                    
                    loop_while_22229 = loop_while_tmp_22872;
                    i_22230 = i_tmp_22873;
                    result_22231 = result_tmp_22874;
                }
                compare_abs_res_f_res_22226 = loop_while_22229;
                compare_abs_res_f_res_22227 = i_22230;
                compare_abs_res_f_res_22228 = result_22231;
                compare_abs_res_22221 = compare_abs_res_f_res_22228;
            }
            // stdlib/bigint-fast.fut:65:39-66:77
            
            bool loop_cond_22242 = slt64((int64_t) 0, compare_abs_res_22221);
            
            // stdlib/bigint-fast.fut:65:39-66:77
            
            int64_t loopres_22243;
            bool loopres_22244;
            int32_t loopres_22245;
            int64_t loop_dz2083Uz2081U_22247;
            bool loop_while_22248;
            int32_t digit_22249;
            
            if (memblock_set(ctx, &mem_param_22739, &ext_mem_22736, "ext_mem_22736") != 0)
                return 1;
            loop_dz2083Uz2081U_22247 = loopres_22216;
            loop_while_22248 = loop_cond_22242;
            digit_22249 = u64_res_22210;
            while (loop_while_22248) {
                int64_t bytes_22740 = (int64_t) 4 * loop_dz2083Uz2081U_22247;
                
                // stdlib/bigint-fast.fut:66:50-52
                
                int32_t tmp_22251 = sub32(digit_22249, 1);
                
                if (memblock_alloc(ctx, &mem_22741, bytes_22740, "mem_22741")) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/bigint-fast.fut:31:54-79
                for (int64_t nest_i_22880 = 0; nest_i_22880 < loop_dz2083Uz2081U_22247; nest_i_22880++) {
                    ((int32_t *) mem_22741.mem)[nest_i_22880] = 0;
                }
                // stdlib/bigint-fast.fut:31:33-35:39
                
                int64_t sub_abs_res_22254;
                int64_t borrow_22257 = (int64_t) 0;
                
                for (int64_t i_22255 = 0; i_22255 < loop_dz2083Uz2081U_22247; i_22255++) {
                    // stdlib/bigint-fast.fut:32:21-25
                    
                    int32_t u32_arg0_22258 = ((int32_t *) mem_param_22739.mem)[i_22255];
                    
                    // stdlib/bigint-fast.fut:32:13-25
                    
                    int64_t u32_res_22259 = zext_i32_i64(u32_arg0_22258);
                    
                    // stdlib/bigint-fast.fut:33:14-54
                    
                    bool cond_22260 = slt64(i_22255, high_index_22194);
                    
                    // stdlib/bigint-fast.fut:33:14-54
                    
                    int64_t zp_lhs_22261;
                    
                    if (cond_22260) {
                        bool index_concat_cmp_22262 = sle64(binop_x_22193, i_22255);
                        int32_t index_concat_branch_22263;
                        
                        if (index_concat_cmp_22262) {
                            int64_t index_concat_i_22264 = sub64(i_22255, binop_x_22193);
                            int32_t index_concat_22265 = ((int32_t *) ext_mem_22722.mem)[index_concat_i_22264];
                            
                            index_concat_branch_22263 = index_concat_22265;
                        } else {
                            index_concat_branch_22263 = 0;
                        }
                        // stdlib/bigint-fast.fut:33:35-47
                        
                        int64_t u32_res_22266 = zext_i32_i64(index_concat_branch_22263);
                        
                        zp_lhs_22261 = u32_res_22266;
                    } else {
                        zp_lhs_22261 = (int64_t) 0;
                    }
                    // stdlib/bigint-fast.fut:33:55-62
                    
                    int64_t y_22267 = add64(borrow_22257, zp_lhs_22261);
                    
                    // stdlib/bigint-fast.fut:34:28-30
                    
                    int64_t u64_arg0_22268 = sub64(u32_res_22259, y_22267);
                    
                    // stdlib/bigint-fast.fut:34:18-30
                    
                    int32_t u64_res_22269 = zext_i64_i32(u64_arg0_22268);
                    
                    // stdlib/bigint-fast.fut:34:9-31
                    ((int32_t *) mem_22741.mem)[i_22255] = u64_res_22269;
                    // stdlib/bigint-fast.fut:35:13-38
                    
                    bool cond_22271 = ult64(u32_res_22259, y_22267);
                    
                    // stdlib/bigint-fast.fut:66:53-76
                    
                    int64_t tmp_22272 = btoi_bool_i64(cond_22271);
                    int64_t borrow_tmp_22882 = tmp_22272;
                    
                    borrow_22257 = borrow_tmp_22882;
                }
                sub_abs_res_22254 = borrow_22257;
                // stdlib/bigint-fast.fut:36:7-26
                
                int64_t sub_abs_res_22273;
                bool sub_abs_res_22274;
                
                if (futrts_normalise_11622(ctx, &ext_mem_22750, &sub_abs_res_22273, &sub_abs_res_22274, mem_22741, loop_dz2083Uz2081U_22247, 0) != 0) {
                    err = 1;
                    goto cleanup;
                }
                if (memblock_unref(ctx, &mem_22741, "mem_22741") != 0)
                    return 1;
                // stdlib/bigint-fast.fut:12:3-16:17
                
                bool cond_22276 = sub_abs_res_22273 == loop_dz2081Uz2080U_22190;
                
                // stdlib/bigint-fast.fut:12:3-16:17
                
                bool cond_22277 = !cond_22276;
                
                // stdlib/bigint-fast.fut:12:3-16:17
                
                int64_t compare_abs_res_22278;
                
                if (cond_22277) {
                    // stdlib/bigint-fast.fut:12:32-69
                    
                    bool cond_22279 = slt64(sub_abs_res_22273, loop_dz2081Uz2080U_22190);
                    
                    // stdlib/bigint-fast.fut:12:32-69
                    
                    int64_t compare_abs_res_t_res_22280;
                    
                    if (cond_22279) {
                        compare_abs_res_t_res_22280 = (int64_t) -1;
                    } else {
                        compare_abs_res_t_res_22280 = (int64_t) 1;
                    }
                    compare_abs_res_22278 = compare_abs_res_t_res_22280;
                } else {
                    // stdlib/bigint-fast.fut:13:63-65
                    
                    int64_t tmp_22281 = sub64(sub_abs_res_22273, (int64_t) 1);
                    
                    // stdlib/bigint-fast.fut:13:36-15:74
                    
                    bool cond_22282 = sle64((int64_t) 0, tmp_22281);
                    
                    // stdlib/bigint-fast.fut:13:36-15:74
                    
                    bool compare_abs_res_f_res_22283;
                    int64_t compare_abs_res_f_res_22284;
                    int64_t compare_abs_res_f_res_22285;
                    bool loop_while_22286;
                    int64_t i_22287;
                    int64_t result_22288;
                    
                    loop_while_22286 = cond_22282;
                    i_22287 = tmp_22281;
                    result_22288 = (int64_t) 0;
                    while (loop_while_22286) {
                        // stdlib/bigint-fast.fut:15:14-16
                        
                        int64_t tmp_22289 = sub64(i_22287, (int64_t) 1);
                        
                        // stdlib/bigint-fast.fut:15:20-24
                        
                        int32_t zl_lhs_22290 = ((int32_t *) ext_mem_22750.mem)[i_22287];
                        
                        // stdlib/bigint-fast.fut:15:27-31
                        
                        int32_t zl_rhs_22291 = ((int32_t *) mem_param_22730.mem)[i_22287];
                        
                        // stdlib/bigint-fast.fut:15:17-73
                        
                        bool cond_22292 = ult32(zl_lhs_22290, zl_rhs_22291);
                        
                        // stdlib/bigint-fast.fut:15:17-73
                        
                        int64_t tmp_22293;
                        
                        if (cond_22292) {
                            tmp_22293 = (int64_t) -1;
                        } else {
                            // stdlib/bigint-fast.fut:15:45-73
                            
                            bool cond_22294 = ult32(zl_rhs_22291, zl_lhs_22290);
                            
                            // stdlib/bigint-fast.fut:66:15-36
                            
                            int64_t tmp_f_res_22295 = btoi_bool_i64(cond_22294);
                            
                            tmp_22293 = tmp_f_res_22295;
                        }
                        // stdlib/bigint-fast.fut:14:16-37
                        
                        bool cond_22296 = sle64((int64_t) 0, tmp_22289);
                        
                        // stdlib/bigint-fast.fut:14:33-37
                        
                        bool loop_cond_t_res_22297 = tmp_22293 == (int64_t) 0;
                        
                        // stdlib/bigint-fast.fut:66:15-36
                        
                        bool x_22298 = cond_22296 && loop_cond_t_res_22297;
                        bool loop_while_tmp_22883 = x_22298;
                        int64_t i_tmp_22884 = tmp_22289;
                        int64_t result_tmp_22885 = tmp_22293;
                        
                        loop_while_22286 = loop_while_tmp_22883;
                        i_22287 = i_tmp_22884;
                        result_22288 = result_tmp_22885;
                    }
                    compare_abs_res_f_res_22283 = loop_while_22286;
                    compare_abs_res_f_res_22284 = i_22287;
                    compare_abs_res_f_res_22285 = result_22288;
                    compare_abs_res_22278 = compare_abs_res_f_res_22285;
                }
                // stdlib/bigint-fast.fut:66:37-40
                
                bool loop_cond_22299 = slt64((int64_t) 0, compare_abs_res_22278);
                
                if (memblock_set(ctx, &mem_param_tmp_22875, &ext_mem_22750, "ext_mem_22750") != 0)
                    return 1;
                
                int64_t loop_dz2083Uz2081U_tmp_22876 = sub_abs_res_22273;
                bool loop_while_tmp_22877 = loop_cond_22299;
                int32_t digit_tmp_22878 = tmp_22251;
                
                if (memblock_set(ctx, &mem_param_22739, &mem_param_tmp_22875, "mem_param_tmp_22875") != 0)
                    return 1;
                loop_dz2083Uz2081U_22247 = loop_dz2083Uz2081U_tmp_22876;
                loop_while_22248 = loop_while_tmp_22877;
                digit_22249 = digit_tmp_22878;
            }
            if (memblock_set(ctx, &ext_mem_22753, &mem_param_22739, "mem_param_22739") != 0)
                return 1;
            loopres_22243 = loop_dz2083Uz2081U_22247;
            loopres_22244 = loop_while_22248;
            loopres_22245 = digit_22249;
            if (memblock_unref(ctx, &ext_mem_22736, "ext_mem_22736") != 0)
                return 1;
            if (memblock_alloc(ctx, &mem_22755, bytes_22754, "mem_22755")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:31:54-79
            for (int64_t nest_i_22886 = 0; nest_i_22886 < loop_dz2081Uz2080U_22190; nest_i_22886++) {
                ((int32_t *) mem_22755.mem)[nest_i_22886] = 0;
            }
            // stdlib/bigint-fast.fut:31:33-35:39
            
            int64_t sub_abs_res_22302;
            int64_t borrow_22305 = (int64_t) 0;
            
            for (int64_t i_22303 = 0; i_22303 < loop_dz2081Uz2080U_22190; i_22303++) {
                // stdlib/bigint-fast.fut:32:21-25
                
                int32_t u32_arg0_22306 = ((int32_t *) mem_param_22730.mem)[i_22303];
                
                // stdlib/bigint-fast.fut:32:13-25
                
                int64_t u32_res_22307 = zext_i32_i64(u32_arg0_22306);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                bool cond_22308 = slt64(i_22303, loopres_22243);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                int64_t zp_lhs_22309;
                
                if (cond_22308) {
                    // stdlib/bigint-fast.fut:33:43-47
                    
                    int32_t u32_arg0_22310 = ((int32_t *) ext_mem_22753.mem)[i_22303];
                    
                    // stdlib/bigint-fast.fut:33:35-47
                    
                    int64_t u32_res_22311 = zext_i32_i64(u32_arg0_22310);
                    
                    zp_lhs_22309 = u32_res_22311;
                } else {
                    zp_lhs_22309 = (int64_t) 0;
                }
                // stdlib/bigint-fast.fut:33:55-62
                
                int64_t y_22312 = add64(borrow_22305, zp_lhs_22309);
                
                // stdlib/bigint-fast.fut:34:28-30
                
                int64_t u64_arg0_22313 = sub64(u32_res_22307, y_22312);
                
                // stdlib/bigint-fast.fut:34:18-30
                
                int32_t u64_res_22314 = zext_i64_i32(u64_arg0_22313);
                
                // stdlib/bigint-fast.fut:34:9-31
                ((int32_t *) mem_22755.mem)[i_22303] = u64_res_22314;
                // stdlib/bigint-fast.fut:35:13-38
                
                bool cond_22316 = ult64(u32_res_22307, y_22312);
                
                // stdlib/bigint-fast.fut:67:15-32
                
                int64_t tmp_22317 = btoi_bool_i64(cond_22316);
                int64_t borrow_tmp_22888 = tmp_22317;
                
                borrow_22305 = borrow_tmp_22888;
            }
            sub_abs_res_22302 = borrow_22305;
            if (memblock_unref(ctx, &ext_mem_22753, "ext_mem_22753") != 0)
                return 1;
            // stdlib/bigint-fast.fut:36:7-26
            
            int64_t sub_abs_res_22318;
            bool sub_abs_res_22319;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22764, &sub_abs_res_22318, &sub_abs_res_22319, mem_22755, loop_dz2081Uz2080U_22190, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_22755, "mem_22755") != 0)
                return 1;
            // stdlib/bigint-fast.fut:68:11-27
            ((int32_t *) mem_22724.mem)[binop_x_22193] = loopres_22245;
            if (memblock_set(ctx, &mem_param_tmp_22866, &ext_mem_22764, "ext_mem_22764") != 0)
                return 1;
            
            int64_t loop_dz2081Uz2080U_tmp_22867 = sub_abs_res_22318;
            
            if (memblock_set(ctx, &mem_param_22730, &mem_param_tmp_22866, "mem_param_tmp_22866") != 0)
                return 1;
            loop_dz2081Uz2080U_22190 = loop_dz2081Uz2080U_tmp_22867;
        }
        if (memblock_set(ctx, &ext_mem_22769, &mem_param_22730, "mem_param_22730") != 0)
            return 1;
        quotient_long_res_22186 = loop_dz2081Uz2080U_22190;
        if (memblock_unref(ctx, &ext_mem_22721, "ext_mem_22721") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22722, "ext_mem_22722") != 0)
            return 1;
        // stdlib/bigint-fast.fut:73:11-74:42
        
        bool cond_22325 = quotient_long_res_22186 == (int64_t) 1;
        
        // stdlib/bigint-fast.fut:73:39-51
        
        bool y_22326 = slt64((int64_t) 0, quotient_long_res_22186);
        
        // stdlib/bigint-fast.fut:82:15-89:66
        
        bool loop_not_taken_22327 = !cond_22325;
        
        // stdlib/bigint-fast.fut:82:15-89:66
        
        bool protect_assert_disj_22328 = y_22326 || loop_not_taken_22327;
        
        // stdlib/bigint-fast.fut:82:15-89:66
        
        bool protect_assert_disj_22329 = cond_20364 || protect_assert_disj_22328;
        
        // stdlib/bigint-fast.fut:73:39-51
        
        bool index_certs_22330;
        
        if (!protect_assert_disj_22329) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) quotient_long_res_22186, "].", "-> #0  stdlib/bigint-fast.fut:73:39-51\n   #1  stdlib/bigint-fast.fut:82:15-89:66\n   #2  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint-fast.fut:70:16-33
        
        int64_t quotient_long_res_22322;
        bool quotient_long_res_22323;
        
        if (futrts_normalise_11622(ctx, &ext_mem_22773, &quotient_long_res_22322, &quotient_long_res_22323, mem_22724, max_res_22178, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_22724, "mem_22724") != 0)
            return 1;
        // stdlib/bigint-fast.fut:82:15-89:66
        
        bool protect_cond_conj_22331 = x_20365 && cond_22325;
        
        // stdlib/bigint-fast.fut:73:39-51
        
        int32_t zeze_lhs_22332;
        
        if (protect_cond_conj_22331) {
            // stdlib/bigint-fast.fut:82:15-89:66
            
            int32_t x_22333 = ((int32_t *) ext_mem_22769.mem)[(int64_t) 0];
            
            zeze_lhs_22332 = x_22333;
        } else {
            zeze_lhs_22332 = 0;
        }
        if (memblock_unref(ctx, &ext_mem_22769, "ext_mem_22769") != 0)
            return 1;
        // stdlib/bigint-fast.fut:73:52-56
        
        bool cond_t_res_22334 = zeze_lhs_22332 == 0;
        
        // stdlib/bigint-fast.fut:82:15-89:66
        
        bool x_22335 = cond_22325 && cond_t_res_22334;
        
        // stdlib/bigint-fast.fut:41:34-54
        
        bool normalise_arg_22336 = !quotient_long_res_22323;
        int64_t bytes_22787 = (int64_t) 4 * quotient_long_res_22322;
        
        // stdlib/bigint-fast.fut:52:3-74:42
        
        int64_t quotient_long_res_22337;
        bool quotient_long_res_22338;
        
        if (cond_20364) {
            if (memblock_set(ctx, &ext_mem_22806, &ext_mem_22773, "ext_mem_22773") != 0)
                return 1;
            quotient_long_res_22337 = quotient_long_res_22322;
            quotient_long_res_22338 = quotient_long_res_22323;
        } else {
            // stdlib/bigint-fast.fut:73:11-74:42
            
            int64_t quotient_long_res_f_res_22340;
            bool quotient_long_res_f_res_22341;
            
            if (x_22335) {
                // stdlib/bigint-fast.fut:41:34-54
                
                int64_t negate_res_22343;
                bool negate_res_22344;
                
                if (futrts_normalise_11622(ctx, &ext_mem_22804, &negate_res_22343, &negate_res_22344, ext_mem_22773, quotient_long_res_22322, normalise_arg_22336) != 0) {
                    err = 1;
                    goto cleanup;
                }
                if (memblock_set(ctx, &ext_mem_22805, &ext_mem_22804, "ext_mem_22804") != 0)
                    return 1;
                quotient_long_res_f_res_22340 = negate_res_22343;
                quotient_long_res_f_res_22341 = negate_res_22344;
            } else {
                // stdlib/bigint-fast.fut:74:31-40
                
                int64_t quotient_long_res_f_res_f_res_22346;
                bool quotient_long_res_f_res_f_res_22347;
                
                if (futrts_integer_6855(ctx, &ext_mem_22774, &quotient_long_res_f_res_f_res_22346, &quotient_long_res_f_res_f_res_22347, (int64_t) 1) != 0) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/bigint-fast.fut:38:3-40:39
                
                bool cond_22349 = quotient_long_res_22323 == quotient_long_res_f_res_f_res_22347;
                
                // stdlib/bigint-fast.fut:70:7-74:40
                
                bool x_22350 = !cond_22349;
                
                // stdlib/bigint-fast.fut:39:11-30
                
                int64_t zgze_lhs_22351;
                
                if (x_22350) {
                    // stdlib/bigint-fast.fut:12:3-16:17
                    
                    bool cond_22352 = quotient_long_res_22322 == quotient_long_res_f_res_f_res_22346;
                    
                    // stdlib/bigint-fast.fut:12:3-16:17
                    
                    bool cond_22353 = !cond_22352;
                    
                    // stdlib/bigint-fast.fut:13:63-65
                    
                    int64_t tmp_22354 = sub64(quotient_long_res_22322, (int64_t) 1);
                    
                    // stdlib/bigint-fast.fut:13:36-15:74
                    
                    bool cond_22355 = sle64((int64_t) 0, tmp_22354);
                    int64_t compare_abs_res_f_res_22356;
                    
                    if (cond_22352) {
                        bool x_22357;
                        int64_t x_22358;
                        int64_t x_22359;
                        bool loop_while_22360;
                        int64_t i_22361;
                        int64_t result_22362;
                        
                        loop_while_22360 = cond_22355;
                        i_22361 = tmp_22354;
                        result_22362 = (int64_t) 0;
                        while (loop_while_22360) {
                            // stdlib/bigint-fast.fut:15:14-16
                            
                            int64_t tmp_22363 = sub64(i_22361, (int64_t) 1);
                            
                            // stdlib/bigint-fast.fut:15:20-24
                            
                            int32_t zl_lhs_22364 = ((int32_t *) ext_mem_22773.mem)[i_22361];
                            
                            // stdlib/bigint-fast.fut:15:27-31
                            
                            int32_t zl_rhs_22365 = ((int32_t *) ext_mem_22774.mem)[i_22361];
                            
                            // stdlib/bigint-fast.fut:15:17-73
                            
                            bool cond_22366 = ult32(zl_lhs_22364, zl_rhs_22365);
                            
                            // stdlib/bigint-fast.fut:15:17-73
                            
                            int64_t tmp_22367;
                            
                            if (cond_22366) {
                                tmp_22367 = (int64_t) -1;
                            } else {
                                // stdlib/bigint-fast.fut:15:45-73
                                
                                bool cond_22368 = ult32(zl_rhs_22365, zl_lhs_22364);
                                
                                // stdlib/bigint-fast.fut:70:7-74:40
                                
                                int64_t tmp_f_res_22369 = btoi_bool_i64(cond_22368);
                                
                                tmp_22367 = tmp_f_res_22369;
                            }
                            // stdlib/bigint-fast.fut:14:16-37
                            
                            bool cond_22370 = sle64((int64_t) 0, tmp_22363);
                            
                            // stdlib/bigint-fast.fut:14:33-37
                            
                            bool loop_cond_t_res_22371 = tmp_22367 == (int64_t) 0;
                            
                            // stdlib/bigint-fast.fut:70:7-74:40
                            
                            bool x_22372 = cond_22370 && loop_cond_t_res_22371;
                            bool loop_while_tmp_22889 = x_22372;
                            int64_t i_tmp_22890 = tmp_22363;
                            int64_t result_tmp_22891 = tmp_22367;
                            
                            loop_while_22360 = loop_while_tmp_22889;
                            i_22361 = i_tmp_22890;
                            result_22362 = result_tmp_22891;
                        }
                        x_22357 = loop_while_22360;
                        x_22358 = i_22361;
                        x_22359 = result_22362;
                        compare_abs_res_f_res_22356 = x_22359;
                    } else {
                        compare_abs_res_f_res_22356 = (int64_t) 0;
                    }
                    // stdlib/bigint-fast.fut:12:32-69
                    
                    bool cond_22373 = slt64(quotient_long_res_22322, quotient_long_res_f_res_f_res_22346);
                    
                    // stdlib/bigint-fast.fut:12:32-69
                    
                    int64_t compare_abs_res_t_res_22374;
                    
                    if (cond_22373) {
                        compare_abs_res_t_res_22374 = (int64_t) -1;
                    } else {
                        compare_abs_res_t_res_22374 = (int64_t) 1;
                    }
                    // stdlib/bigint-fast.fut:12:3-16:17
                    
                    int64_t compare_abs_res_22375;
                    
                    if (cond_22353) {
                        compare_abs_res_22375 = compare_abs_res_t_res_22374;
                    } else {
                        compare_abs_res_22375 = compare_abs_res_f_res_22356;
                    }
                    zgze_lhs_22351 = compare_abs_res_22375;
                } else {
                    zgze_lhs_22351 = (int64_t) 0;
                }
                // stdlib/bigint-fast.fut:39:8-40:39
                
                bool cond_22376 = sle64((int64_t) 0, zgze_lhs_22351);
                int64_t bytes_22775 = (int64_t) 4 * quotient_long_res_f_res_f_res_22346;
                
                // stdlib/bigint-fast.fut:38:3-40:39
                
                int64_t add_res_22377;
                bool add_res_22378;
                
                if (cond_22349) {
                    // stdlib/bigint-fast.fut:38:37-52
                    
                    int64_t normalise_arg1_22380;
                    
                    if (futrts_add_abs_11626(ctx, &ext_mem_22800, &normalise_arg1_22380, ext_mem_22773, ext_mem_22774, quotient_long_res_22322, quotient_long_res_f_res_f_res_22346) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/bigint-fast.fut:38:22-52
                    
                    int64_t add_res_t_res_22382;
                    bool add_res_t_res_22383;
                    
                    if (futrts_normalise_11622(ctx, &ext_mem_22801, &add_res_t_res_22382, &add_res_t_res_22383, ext_mem_22800, normalise_arg1_22380, quotient_long_res_22323) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &ext_mem_22800, "ext_mem_22800") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_22802, &ext_mem_22801, "ext_mem_22801") != 0)
                        return 1;
                    add_res_22377 = add_res_t_res_22382;
                    add_res_22378 = add_res_t_res_22383;
                } else {
                    // stdlib/bigint-fast.fut:39:8-40:39
                    
                    int64_t add_res_f_res_22385;
                    bool add_res_f_res_22386;
                    
                    if (cond_22376) {
                        if (memblock_alloc(ctx, &mem_22788, bytes_22787, "mem_22788")) {
                            err = 1;
                            goto cleanup;
                        }
                        // stdlib/bigint-fast.fut:31:54-79
                        for (int64_t nest_i_22892 = 0; nest_i_22892 < quotient_long_res_22322; nest_i_22892++) {
                            ((int32_t *) mem_22788.mem)[nest_i_22892] = 0;
                        }
                        // stdlib/bigint-fast.fut:31:33-35:39
                        
                        int64_t sub_abs_res_22390;
                        int64_t borrow_22393 = (int64_t) 0;
                        
                        for (int64_t i_22391 = 0; i_22391 < quotient_long_res_22322; i_22391++) {
                            // stdlib/bigint-fast.fut:32:21-25
                            
                            int32_t u32_arg0_22394 = ((int32_t *) ext_mem_22773.mem)[i_22391];
                            
                            // stdlib/bigint-fast.fut:32:13-25
                            
                            int64_t u32_res_22395 = zext_i32_i64(u32_arg0_22394);
                            
                            // stdlib/bigint-fast.fut:33:14-54
                            
                            bool cond_22396 = slt64(i_22391, quotient_long_res_f_res_f_res_22346);
                            
                            // stdlib/bigint-fast.fut:33:14-54
                            
                            int64_t zp_lhs_22397;
                            
                            if (cond_22396) {
                                // stdlib/bigint-fast.fut:33:43-47
                                
                                int32_t u32_arg0_22398 = ((int32_t *) ext_mem_22774.mem)[i_22391];
                                
                                // stdlib/bigint-fast.fut:33:35-47
                                
                                int64_t u32_res_22399 = zext_i32_i64(u32_arg0_22398);
                                
                                zp_lhs_22397 = u32_res_22399;
                            } else {
                                zp_lhs_22397 = (int64_t) 0;
                            }
                            // stdlib/bigint-fast.fut:33:55-62
                            
                            int64_t y_22400 = add64(borrow_22393, zp_lhs_22397);
                            
                            // stdlib/bigint-fast.fut:34:28-30
                            
                            int64_t u64_arg0_22401 = sub64(u32_res_22395, y_22400);
                            
                            // stdlib/bigint-fast.fut:34:18-30
                            
                            int32_t u64_res_22402 = zext_i64_i32(u64_arg0_22401);
                            
                            // stdlib/bigint-fast.fut:34:9-31
                            ((int32_t *) mem_22788.mem)[i_22391] = u64_res_22402;
                            // stdlib/bigint-fast.fut:35:13-38
                            
                            bool cond_22404 = ult64(u32_res_22395, y_22400);
                            
                            // stdlib/bigint-fast.fut:39:56-71
                            
                            int64_t tmp_22405 = btoi_bool_i64(cond_22404);
                            int64_t borrow_tmp_22894 = tmp_22405;
                            
                            borrow_22393 = borrow_tmp_22894;
                        }
                        sub_abs_res_22390 = borrow_22393;
                        // stdlib/bigint-fast.fut:36:7-26
                        
                        int64_t sub_abs_res_22406;
                        bool sub_abs_res_22407;
                        
                        if (futrts_normalise_11622(ctx, &ext_mem_22797, &sub_abs_res_22406, &sub_abs_res_22407, mem_22788, quotient_long_res_22322, 0) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        if (memblock_unref(ctx, &mem_22788, "mem_22788") != 0)
                            return 1;
                        // stdlib/bigint-fast.fut:39:41-71
                        
                        int64_t add_res_f_res_t_res_22409;
                        bool add_res_f_res_t_res_22410;
                        
                        if (futrts_normalise_11622(ctx, &ext_mem_22798, &add_res_f_res_t_res_22409, &add_res_f_res_t_res_22410, ext_mem_22797, sub_abs_res_22406, quotient_long_res_22323) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        if (memblock_unref(ctx, &ext_mem_22797, "ext_mem_22797") != 0)
                            return 1;
                        if (memblock_set(ctx, &ext_mem_22799, &ext_mem_22798, "ext_mem_22798") != 0)
                            return 1;
                        add_res_f_res_22385 = add_res_f_res_t_res_22409;
                        add_res_f_res_22386 = add_res_f_res_t_res_22410;
                    } else {
                        if (memblock_alloc(ctx, &mem_22776, bytes_22775, "mem_22776")) {
                            err = 1;
                            goto cleanup;
                        }
                        // stdlib/bigint-fast.fut:31:54-79
                        for (int64_t nest_i_22895 = 0; nest_i_22895 < quotient_long_res_f_res_f_res_22346; nest_i_22895++) {
                            ((int32_t *) mem_22776.mem)[nest_i_22895] = 0;
                        }
                        // stdlib/bigint-fast.fut:31:33-35:39
                        
                        int64_t sub_abs_res_22414;
                        int64_t borrow_22417 = (int64_t) 0;
                        
                        for (int64_t i_22415 = 0; i_22415 < quotient_long_res_f_res_f_res_22346; i_22415++) {
                            // stdlib/bigint-fast.fut:32:21-25
                            
                            int32_t u32_arg0_22418 = ((int32_t *) ext_mem_22774.mem)[i_22415];
                            
                            // stdlib/bigint-fast.fut:32:13-25
                            
                            int64_t u32_res_22419 = zext_i32_i64(u32_arg0_22418);
                            
                            // stdlib/bigint-fast.fut:33:14-54
                            
                            bool cond_22420 = slt64(i_22415, quotient_long_res_22322);
                            
                            // stdlib/bigint-fast.fut:33:14-54
                            
                            int64_t zp_lhs_22421;
                            
                            if (cond_22420) {
                                // stdlib/bigint-fast.fut:33:43-47
                                
                                int32_t u32_arg0_22422 = ((int32_t *) ext_mem_22773.mem)[i_22415];
                                
                                // stdlib/bigint-fast.fut:33:35-47
                                
                                int64_t u32_res_22423 = zext_i32_i64(u32_arg0_22422);
                                
                                zp_lhs_22421 = u32_res_22423;
                            } else {
                                zp_lhs_22421 = (int64_t) 0;
                            }
                            // stdlib/bigint-fast.fut:33:55-62
                            
                            int64_t y_22424 = add64(borrow_22417, zp_lhs_22421);
                            
                            // stdlib/bigint-fast.fut:34:28-30
                            
                            int64_t u64_arg0_22425 = sub64(u32_res_22419, y_22424);
                            
                            // stdlib/bigint-fast.fut:34:18-30
                            
                            int32_t u64_res_22426 = zext_i64_i32(u64_arg0_22425);
                            
                            // stdlib/bigint-fast.fut:34:9-31
                            ((int32_t *) mem_22776.mem)[i_22415] = u64_res_22426;
                            // stdlib/bigint-fast.fut:35:13-38
                            
                            bool cond_22428 = ult64(u32_res_22419, y_22424);
                            
                            // stdlib/bigint-fast.fut:40:23-38
                            
                            int64_t tmp_22429 = btoi_bool_i64(cond_22428);
                            int64_t borrow_tmp_22897 = tmp_22429;
                            
                            borrow_22417 = borrow_tmp_22897;
                        }
                        sub_abs_res_22414 = borrow_22417;
                        // stdlib/bigint-fast.fut:36:7-26
                        
                        int64_t sub_abs_res_22430;
                        bool sub_abs_res_22431;
                        
                        if (futrts_normalise_11622(ctx, &ext_mem_22785, &sub_abs_res_22430, &sub_abs_res_22431, mem_22776, quotient_long_res_f_res_f_res_22346, 0) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        if (memblock_unref(ctx, &mem_22776, "mem_22776") != 0)
                            return 1;
                        // stdlib/bigint-fast.fut:40:8-38
                        
                        int64_t add_res_f_res_f_res_22433;
                        bool add_res_f_res_f_res_22434;
                        
                        if (futrts_normalise_11622(ctx, &ext_mem_22786, &add_res_f_res_f_res_22433, &add_res_f_res_f_res_22434, ext_mem_22785, sub_abs_res_22430, quotient_long_res_f_res_f_res_22347) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        if (memblock_unref(ctx, &ext_mem_22785, "ext_mem_22785") != 0)
                            return 1;
                        if (memblock_set(ctx, &ext_mem_22799, &ext_mem_22786, "ext_mem_22786") != 0)
                            return 1;
                        add_res_f_res_22385 = add_res_f_res_f_res_22433;
                        add_res_f_res_22386 = add_res_f_res_f_res_22434;
                    }
                    if (memblock_set(ctx, &ext_mem_22802, &ext_mem_22799, "ext_mem_22799") != 0)
                        return 1;
                    add_res_22377 = add_res_f_res_22385;
                    add_res_22378 = add_res_f_res_22386;
                }
                if (memblock_unref(ctx, &ext_mem_22774, "ext_mem_22774") != 0)
                    return 1;
                // stdlib/bigint-fast.fut:41:34-54
                
                bool normalise_arg_22436 = !add_res_22378;
                
                // stdlib/bigint-fast.fut:41:34-54
                
                int64_t negate_res_22437;
                bool negate_res_22438;
                
                if (futrts_normalise_11622(ctx, &ext_mem_22803, &negate_res_22437, &negate_res_22438, ext_mem_22802, add_res_22377, normalise_arg_22436) != 0) {
                    err = 1;
                    goto cleanup;
                }
                if (memblock_unref(ctx, &ext_mem_22802, "ext_mem_22802") != 0)
                    return 1;
                if (memblock_set(ctx, &ext_mem_22805, &ext_mem_22803, "ext_mem_22803") != 0)
                    return 1;
                quotient_long_res_f_res_22340 = negate_res_22437;
                quotient_long_res_f_res_22341 = negate_res_22438;
            }
            if (memblock_set(ctx, &ext_mem_22806, &ext_mem_22805, "ext_mem_22805") != 0)
                return 1;
            quotient_long_res_22337 = quotient_long_res_f_res_22340;
            quotient_long_res_22338 = quotient_long_res_f_res_22341;
        }
        if (memblock_unref(ctx, &ext_mem_22773, "ext_mem_22773") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_22807, &ext_mem_22806, "ext_mem_22806") != 0)
            return 1;
        quotient_res_20668 = quotient_long_res_22337;
        quotient_res_20669 = quotient_long_res_22338;
    } else {
        // stdlib/bigint-fast.fut:90:11-103:50
        
        int64_t quotient_res_f_res_20939;
        bool quotient_res_f_res_20940;
        
        if (cond_20622) {
            // stdlib/bigint-fast.fut:90:43-117
            
            int64_t quotient_res_f_res_t_res_22440;
            bool quotient_res_f_res_t_res_22441;
            
            if (futrts_integer_6855(ctx, &ext_mem_22719, &quotient_res_f_res_t_res_22440, &quotient_res_f_res_t_res_22441, integer_arg0_20643) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_22720, &ext_mem_22719, "ext_mem_22719") != 0)
                return 1;
            quotient_res_f_res_20939 = quotient_res_f_res_t_res_22440;
            quotient_res_f_res_20940 = quotient_res_f_res_t_res_22441;
        } else {
            // stdlib/bigint-fast.fut:94:25-42
            
            int64_t product_20945;
            
            if (futrts_mul_abs_11639(ctx, &ext_mem_22642, &product_20945, ext_mem_22582, arithmetic_res_20361, unsign_arg0_20624) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:12:3-16:17
            
            bool cond_20947 = product_20945 == arithmetic_res_20358;
            
            // stdlib/bigint-fast.fut:12:3-16:17
            
            bool cond_20948 = !cond_20947;
            
            // stdlib/bigint-fast.fut:13:63-65
            
            int64_t tmp_20949 = sub64(product_20945, (int64_t) 1);
            
            // stdlib/bigint-fast.fut:13:36-15:74
            
            bool cond_20950 = sle64((int64_t) 0, tmp_20949);
            int64_t compare_abs_res_f_res_20951;
            
            if (cond_20947) {
                bool x_22443;
                int64_t x_22444;
                int64_t x_22445;
                bool loop_while_22446;
                int64_t i_22447;
                int64_t result_22448;
                
                loop_while_22446 = cond_20950;
                i_22447 = tmp_20949;
                result_22448 = (int64_t) 0;
                while (loop_while_22446) {
                    // stdlib/bigint-fast.fut:15:14-16
                    
                    int64_t tmp_22449 = sub64(i_22447, (int64_t) 1);
                    
                    // stdlib/bigint-fast.fut:15:20-24
                    
                    int32_t zl_lhs_22450 = ((int32_t *) ext_mem_22642.mem)[i_22447];
                    
                    // stdlib/bigint-fast.fut:15:27-31
                    
                    int32_t zl_rhs_22451 = ((int32_t *) ext_mem_22581.mem)[i_22447];
                    
                    // stdlib/bigint-fast.fut:15:17-73
                    
                    bool cond_22452 = ult32(zl_lhs_22450, zl_rhs_22451);
                    
                    // stdlib/bigint-fast.fut:15:17-73
                    
                    int64_t tmp_22453;
                    
                    if (cond_22452) {
                        tmp_22453 = (int64_t) -1;
                    } else {
                        // stdlib/bigint-fast.fut:15:45-73
                        
                        bool cond_22454 = ult32(zl_rhs_22451, zl_lhs_22450);
                        
                        // stdlib/bigint-fast.fut:95:33-96:79
                        
                        int64_t tmp_f_res_22455 = btoi_bool_i64(cond_22454);
                        
                        tmp_22453 = tmp_f_res_22455;
                    }
                    // stdlib/bigint-fast.fut:14:16-37
                    
                    bool cond_22456 = sle64((int64_t) 0, tmp_22449);
                    
                    // stdlib/bigint-fast.fut:14:33-37
                    
                    bool loop_cond_t_res_22457 = tmp_22453 == (int64_t) 0;
                    
                    // stdlib/bigint-fast.fut:95:33-96:79
                    
                    bool x_22458 = cond_22456 && loop_cond_t_res_22457;
                    bool loop_while_tmp_22898 = x_22458;
                    int64_t i_tmp_22899 = tmp_22449;
                    int64_t result_tmp_22900 = tmp_22453;
                    
                    loop_while_22446 = loop_while_tmp_22898;
                    i_22447 = i_tmp_22899;
                    result_22448 = result_tmp_22900;
                }
                x_22443 = loop_while_22446;
                x_22444 = i_22447;
                x_22445 = result_22448;
                compare_abs_res_f_res_20951 = x_22445;
            } else {
                compare_abs_res_f_res_20951 = (int64_t) 0;
            }
            // stdlib/bigint-fast.fut:12:32-69
            
            bool cond_20968 = slt64(product_20945, arithmetic_res_20358);
            
            // stdlib/bigint-fast.fut:12:32-69
            
            int64_t compare_abs_res_t_res_20969;
            
            if (cond_20968) {
                compare_abs_res_t_res_20969 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_20969 = (int64_t) 1;
            }
            // stdlib/bigint-fast.fut:12:3-16:17
            
            int64_t compare_abs_res_20970;
            
            if (cond_20948) {
                compare_abs_res_20970 = compare_abs_res_t_res_20969;
            } else {
                compare_abs_res_20970 = compare_abs_res_f_res_20951;
            }
            // stdlib/bigint-fast.fut:95:33-96:79
            
            bool loop_cond_20971 = slt64((int64_t) 0, compare_abs_res_20970);
            
            // stdlib/bigint-fast.fut:95:33-96:79
            
            int64_t quotient_res_f_res_f_res_20972;
            bool quotient_res_f_res_f_res_20973;
            int32_t quotient_res_f_res_f_res_20974;
            int64_t loop_dz2081Uz2088U_20976;
            bool loop_while_20977;
            int32_t digit_20978;
            
            if (memblock_set(ctx, &mem_param_22645, &ext_mem_22642, "ext_mem_22642") != 0)
                return 1;
            loop_dz2081Uz2088U_20976 = product_20945;
            loop_while_20977 = loop_cond_20971;
            digit_20978 = unsign_arg0_20624;
            while (loop_while_20977) {
                int64_t bytes_22646 = (int64_t) 4 * loop_dz2081Uz2088U_20976;
                
                // stdlib/bigint-fast.fut:96:56-58
                
                int32_t tmp_20980 = sub32(digit_20978, 1);
                
                if (memblock_alloc(ctx, &mem_22647, bytes_22646, "mem_22647")) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/bigint-fast.fut:31:54-79
                for (int64_t nest_i_22906 = 0; nest_i_22906 < loop_dz2081Uz2088U_20976; nest_i_22906++) {
                    ((int32_t *) mem_22647.mem)[nest_i_22906] = 0;
                }
                // stdlib/bigint-fast.fut:31:33-35:39
                
                int64_t sub_abs_res_20983;
                int64_t borrow_20986 = (int64_t) 0;
                
                for (int64_t i_20984 = 0; i_20984 < loop_dz2081Uz2088U_20976; i_20984++) {
                    // stdlib/bigint-fast.fut:32:21-25
                    
                    int32_t u32_arg0_20987 = ((int32_t *) mem_param_22645.mem)[i_20984];
                    
                    // stdlib/bigint-fast.fut:32:13-25
                    
                    int64_t u32_res_20988 = zext_i32_i64(u32_arg0_20987);
                    
                    // stdlib/bigint-fast.fut:33:14-54
                    
                    bool cond_20989 = slt64(i_20984, arithmetic_res_20361);
                    
                    // stdlib/bigint-fast.fut:33:14-54
                    
                    int64_t zp_lhs_20990;
                    
                    if (cond_20989) {
                        // stdlib/bigint-fast.fut:33:43-47
                        
                        int32_t u32_arg0_22459 = ((int32_t *) ext_mem_22582.mem)[i_20984];
                        
                        // stdlib/bigint-fast.fut:33:35-47
                        
                        int64_t u32_res_22460 = zext_i32_i64(u32_arg0_22459);
                        
                        zp_lhs_20990 = u32_res_22460;
                    } else {
                        zp_lhs_20990 = (int64_t) 0;
                    }
                    // stdlib/bigint-fast.fut:33:55-62
                    
                    int64_t y_20993 = add64(borrow_20986, zp_lhs_20990);
                    
                    // stdlib/bigint-fast.fut:34:28-30
                    
                    int64_t u64_arg0_20994 = sub64(u32_res_20988, y_20993);
                    
                    // stdlib/bigint-fast.fut:34:18-30
                    
                    int32_t u64_res_20995 = zext_i64_i32(u64_arg0_20994);
                    
                    // stdlib/bigint-fast.fut:34:9-31
                    ((int32_t *) mem_22647.mem)[i_20984] = u64_res_20995;
                    // stdlib/bigint-fast.fut:35:13-38
                    
                    bool cond_20997 = ult64(u32_res_20988, y_20993);
                    
                    // stdlib/bigint-fast.fut:96:59-78
                    
                    int64_t tmp_20998 = btoi_bool_i64(cond_20997);
                    int64_t borrow_tmp_22908 = tmp_20998;
                    
                    borrow_20986 = borrow_tmp_22908;
                }
                sub_abs_res_20983 = borrow_20986;
                // stdlib/bigint-fast.fut:36:7-26
                
                int64_t sub_abs_res_20999;
                bool sub_abs_res_21000;
                
                if (futrts_normalise_11622(ctx, &ext_mem_22656, &sub_abs_res_20999, &sub_abs_res_21000, mem_22647, loop_dz2081Uz2088U_20976, 0) != 0) {
                    err = 1;
                    goto cleanup;
                }
                if (memblock_unref(ctx, &mem_22647, "mem_22647") != 0)
                    return 1;
                // stdlib/bigint-fast.fut:12:3-16:17
                
                bool cond_21002 = sub_abs_res_20999 == arithmetic_res_20358;
                
                // stdlib/bigint-fast.fut:12:3-16:17
                
                bool cond_21003 = !cond_21002;
                
                // stdlib/bigint-fast.fut:12:3-16:17
                
                int64_t compare_abs_res_21004;
                
                if (cond_21003) {
                    // stdlib/bigint-fast.fut:12:32-69
                    
                    bool cond_22461 = slt64(sub_abs_res_20999, arithmetic_res_20358);
                    
                    // stdlib/bigint-fast.fut:12:32-69
                    
                    int64_t compare_abs_res_t_res_22462;
                    
                    if (cond_22461) {
                        compare_abs_res_t_res_22462 = (int64_t) -1;
                    } else {
                        compare_abs_res_t_res_22462 = (int64_t) 1;
                    }
                    compare_abs_res_21004 = compare_abs_res_t_res_22462;
                } else {
                    // stdlib/bigint-fast.fut:13:63-65
                    
                    int64_t tmp_21007 = sub64(sub_abs_res_20999, (int64_t) 1);
                    
                    // stdlib/bigint-fast.fut:13:36-15:74
                    
                    bool cond_21008 = sle64((int64_t) 0, tmp_21007);
                    
                    // stdlib/bigint-fast.fut:13:36-15:74
                    
                    bool compare_abs_res_f_res_21009;
                    int64_t compare_abs_res_f_res_21010;
                    int64_t compare_abs_res_f_res_21011;
                    bool loop_while_21012;
                    int64_t i_21013;
                    int64_t result_21014;
                    
                    loop_while_21012 = cond_21008;
                    i_21013 = tmp_21007;
                    result_21014 = (int64_t) 0;
                    while (loop_while_21012) {
                        // stdlib/bigint-fast.fut:15:14-16
                        
                        int64_t tmp_21015 = sub64(i_21013, (int64_t) 1);
                        
                        // stdlib/bigint-fast.fut:15:20-24
                        
                        int32_t zl_lhs_21016 = ((int32_t *) ext_mem_22656.mem)[i_21013];
                        
                        // stdlib/bigint-fast.fut:15:27-31
                        
                        int32_t zl_rhs_21017 = ((int32_t *) ext_mem_22581.mem)[i_21013];
                        
                        // stdlib/bigint-fast.fut:15:17-73
                        
                        bool cond_21018 = ult32(zl_lhs_21016, zl_rhs_21017);
                        
                        // stdlib/bigint-fast.fut:15:17-73
                        
                        int64_t tmp_21019;
                        
                        if (cond_21018) {
                            tmp_21019 = (int64_t) -1;
                        } else {
                            // stdlib/bigint-fast.fut:15:45-73
                            
                            bool cond_21020 = ult32(zl_rhs_21017, zl_lhs_21016);
                            
                            // stdlib/bigint-fast.fut:96:19-42
                            
                            int64_t tmp_f_res_21021 = btoi_bool_i64(cond_21020);
                            
                            tmp_21019 = tmp_f_res_21021;
                        }
                        // stdlib/bigint-fast.fut:14:16-37
                        
                        bool cond_21022 = sle64((int64_t) 0, tmp_21015);
                        
                        // stdlib/bigint-fast.fut:14:33-37
                        
                        bool loop_cond_t_res_21023 = tmp_21019 == (int64_t) 0;
                        
                        // stdlib/bigint-fast.fut:96:19-42
                        
                        bool x_21024 = cond_21022 && loop_cond_t_res_21023;
                        bool loop_while_tmp_22909 = x_21024;
                        int64_t i_tmp_22910 = tmp_21015;
                        int64_t result_tmp_22911 = tmp_21019;
                        
                        loop_while_21012 = loop_while_tmp_22909;
                        i_21013 = i_tmp_22910;
                        result_21014 = result_tmp_22911;
                    }
                    compare_abs_res_f_res_21009 = loop_while_21012;
                    compare_abs_res_f_res_21010 = i_21013;
                    compare_abs_res_f_res_21011 = result_21014;
                    compare_abs_res_21004 = compare_abs_res_f_res_21011;
                }
                // stdlib/bigint-fast.fut:96:43-46
                
                bool loop_cond_21025 = slt64((int64_t) 0, compare_abs_res_21004);
                
                if (memblock_set(ctx, &mem_param_tmp_22901, &ext_mem_22656, "ext_mem_22656") != 0)
                    return 1;
                
                int64_t loop_dz2081Uz2088U_tmp_22902 = sub_abs_res_20999;
                bool loop_while_tmp_22903 = loop_cond_21025;
                int32_t digit_tmp_22904 = tmp_20980;
                
                if (memblock_set(ctx, &mem_param_22645, &mem_param_tmp_22901, "mem_param_tmp_22901") != 0)
                    return 1;
                loop_dz2081Uz2088U_20976 = loop_dz2081Uz2088U_tmp_22902;
                loop_while_20977 = loop_while_tmp_22903;
                digit_20978 = digit_tmp_22904;
            }
            if (memblock_set(ctx, &ext_mem_22659, &mem_param_22645, "mem_param_22645") != 0)
                return 1;
            quotient_res_f_res_f_res_20972 = loop_dz2081Uz2088U_20976;
            quotient_res_f_res_f_res_20973 = loop_while_20977;
            quotient_res_f_res_f_res_20974 = digit_20978;
            if (memblock_unref(ctx, &ext_mem_22642, "ext_mem_22642") != 0)
                return 1;
            // stdlib/bigint-fast.fut:97:33-98:94
            
            int64_t compare_abs_arg0_21026;
            
            if (futrts_add_abs_11626(ctx, &ext_mem_22662, &compare_abs_arg0_21026, ext_mem_22659, ext_mem_22582, quotient_res_f_res_f_res_20972, arithmetic_res_20361) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:12:3-16:17
            
            bool cond_21028 = compare_abs_arg0_21026 == arithmetic_res_20358;
            
            // stdlib/bigint-fast.fut:12:3-16:17
            
            bool cond_21029 = !cond_21028;
            
            // stdlib/bigint-fast.fut:13:63-65
            
            int64_t tmp_21030 = sub64(compare_abs_arg0_21026, (int64_t) 1);
            
            // stdlib/bigint-fast.fut:13:36-15:74
            
            bool cond_21031 = sle64((int64_t) 0, tmp_21030);
            int64_t compare_abs_res_f_res_21032;
            
            if (cond_21028) {
                bool x_22463;
                int64_t x_22464;
                int64_t x_22465;
                bool loop_while_22466;
                int64_t i_22467;
                int64_t result_22468;
                
                loop_while_22466 = cond_21031;
                i_22467 = tmp_21030;
                result_22468 = (int64_t) 0;
                while (loop_while_22466) {
                    // stdlib/bigint-fast.fut:15:14-16
                    
                    int64_t tmp_22469 = sub64(i_22467, (int64_t) 1);
                    
                    // stdlib/bigint-fast.fut:15:20-24
                    
                    int32_t zl_lhs_22470 = ((int32_t *) ext_mem_22662.mem)[i_22467];
                    
                    // stdlib/bigint-fast.fut:15:27-31
                    
                    int32_t zl_rhs_22471 = ((int32_t *) ext_mem_22581.mem)[i_22467];
                    
                    // stdlib/bigint-fast.fut:15:17-73
                    
                    bool cond_22472 = ult32(zl_lhs_22470, zl_rhs_22471);
                    
                    // stdlib/bigint-fast.fut:15:17-73
                    
                    int64_t tmp_22473;
                    
                    if (cond_22472) {
                        tmp_22473 = (int64_t) -1;
                    } else {
                        // stdlib/bigint-fast.fut:15:45-73
                        
                        bool cond_22474 = ult32(zl_rhs_22471, zl_lhs_22470);
                        
                        // stdlib/bigint-fast.fut:97:33-98:94
                        
                        int64_t tmp_f_res_22475 = btoi_bool_i64(cond_22474);
                        
                        tmp_22473 = tmp_f_res_22475;
                    }
                    // stdlib/bigint-fast.fut:14:16-37
                    
                    bool cond_22476 = sle64((int64_t) 0, tmp_22469);
                    
                    // stdlib/bigint-fast.fut:14:33-37
                    
                    bool loop_cond_t_res_22477 = tmp_22473 == (int64_t) 0;
                    
                    // stdlib/bigint-fast.fut:97:33-98:94
                    
                    bool x_22478 = cond_22476 && loop_cond_t_res_22477;
                    bool loop_while_tmp_22912 = x_22478;
                    int64_t i_tmp_22913 = tmp_22469;
                    int64_t result_tmp_22914 = tmp_22473;
                    
                    loop_while_22466 = loop_while_tmp_22912;
                    i_22467 = i_tmp_22913;
                    result_22468 = result_tmp_22914;
                }
                x_22463 = loop_while_22466;
                x_22464 = i_22467;
                x_22465 = result_22468;
                compare_abs_res_f_res_21032 = x_22465;
            } else {
                compare_abs_res_f_res_21032 = (int64_t) 0;
            }
            if (memblock_unref(ctx, &ext_mem_22662, "ext_mem_22662") != 0)
                return 1;
            // stdlib/bigint-fast.fut:12:32-69
            
            bool cond_21049 = slt64(compare_abs_arg0_21026, arithmetic_res_20358);
            
            // stdlib/bigint-fast.fut:12:32-69
            
            int64_t compare_abs_res_t_res_21050;
            
            if (cond_21049) {
                compare_abs_res_t_res_21050 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_21050 = (int64_t) 1;
            }
            // stdlib/bigint-fast.fut:12:3-16:17
            
            int64_t compare_abs_res_21051;
            
            if (cond_21029) {
                compare_abs_res_21051 = compare_abs_res_t_res_21050;
            } else {
                compare_abs_res_21051 = compare_abs_res_f_res_21032;
            }
            // stdlib/bigint-fast.fut:97:33-98:94
            
            bool loop_cond_21052 = sle64(compare_abs_res_21051, (int64_t) 0);
            
            // stdlib/bigint-fast.fut:97:33-98:94
            
            int64_t quotient_res_f_res_f_res_21053;
            bool quotient_res_f_res_f_res_21054;
            int32_t quotient_res_f_res_f_res_21055;
            int64_t loop_dz2082Uz2088U_21057;
            bool loop_while_21058;
            int32_t digit_21059;
            
            if (memblock_set(ctx, &mem_param_22665, &ext_mem_22659, "ext_mem_22659") != 0)
                return 1;
            loop_dz2082Uz2088U_21057 = quotient_res_f_res_f_res_20972;
            loop_while_21058 = loop_cond_21052;
            digit_21059 = quotient_res_f_res_f_res_20974;
            while (loop_while_21058) {
                // stdlib/bigint-fast.fut:98:71-73
                
                int32_t tmp_21061 = add32(1, digit_21059);
                
                // stdlib/bigint-fast.fut:98:74-93
                
                int64_t tmp_21062;
                
                if (futrts_add_abs_11626(ctx, &ext_mem_22668, &tmp_21062, mem_param_22665, ext_mem_22582, loop_dz2082Uz2088U_21057, arithmetic_res_20361) != 0) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/bigint-fast.fut:98:32-51
                
                int64_t compare_abs_arg0_21064;
                
                if (futrts_add_abs_11626(ctx, &ext_mem_22669, &compare_abs_arg0_21064, ext_mem_22668, ext_mem_22582, tmp_21062, arithmetic_res_20361) != 0) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/bigint-fast.fut:12:3-16:17
                
                bool cond_21066 = compare_abs_arg0_21064 == arithmetic_res_20358;
                
                // stdlib/bigint-fast.fut:12:3-16:17
                
                bool cond_21067 = !cond_21066;
                
                // stdlib/bigint-fast.fut:12:3-16:17
                
                int64_t compare_abs_res_21068;
                
                if (cond_21067) {
                    // stdlib/bigint-fast.fut:12:32-69
                    
                    bool cond_22479 = slt64(compare_abs_arg0_21064, arithmetic_res_20358);
                    
                    // stdlib/bigint-fast.fut:12:32-69
                    
                    int64_t compare_abs_res_t_res_22480;
                    
                    if (cond_22479) {
                        compare_abs_res_t_res_22480 = (int64_t) -1;
                    } else {
                        compare_abs_res_t_res_22480 = (int64_t) 1;
                    }
                    compare_abs_res_21068 = compare_abs_res_t_res_22480;
                } else {
                    // stdlib/bigint-fast.fut:13:63-65
                    
                    int64_t tmp_21071 = sub64(compare_abs_arg0_21064, (int64_t) 1);
                    
                    // stdlib/bigint-fast.fut:13:36-15:74
                    
                    bool cond_21072 = sle64((int64_t) 0, tmp_21071);
                    
                    // stdlib/bigint-fast.fut:13:36-15:74
                    
                    bool compare_abs_res_f_res_21073;
                    int64_t compare_abs_res_f_res_21074;
                    int64_t compare_abs_res_f_res_21075;
                    bool loop_while_21076;
                    int64_t i_21077;
                    int64_t result_21078;
                    
                    loop_while_21076 = cond_21072;
                    i_21077 = tmp_21071;
                    result_21078 = (int64_t) 0;
                    while (loop_while_21076) {
                        // stdlib/bigint-fast.fut:15:14-16
                        
                        int64_t tmp_21079 = sub64(i_21077, (int64_t) 1);
                        
                        // stdlib/bigint-fast.fut:15:20-24
                        
                        int32_t zl_lhs_21080 = ((int32_t *) ext_mem_22669.mem)[i_21077];
                        
                        // stdlib/bigint-fast.fut:15:27-31
                        
                        int32_t zl_rhs_21081 = ((int32_t *) ext_mem_22581.mem)[i_21077];
                        
                        // stdlib/bigint-fast.fut:15:17-73
                        
                        bool cond_21082 = ult32(zl_lhs_21080, zl_rhs_21081);
                        
                        // stdlib/bigint-fast.fut:15:17-73
                        
                        int64_t tmp_21083;
                        
                        if (cond_21082) {
                            tmp_21083 = (int64_t) -1;
                        } else {
                            // stdlib/bigint-fast.fut:15:45-73
                            
                            bool cond_21084 = ult32(zl_rhs_21081, zl_lhs_21080);
                            
                            // stdlib/bigint-fast.fut:98:19-56
                            
                            int64_t tmp_f_res_21085 = btoi_bool_i64(cond_21084);
                            
                            tmp_21083 = tmp_f_res_21085;
                        }
                        // stdlib/bigint-fast.fut:14:16-37
                        
                        bool cond_21086 = sle64((int64_t) 0, tmp_21079);
                        
                        // stdlib/bigint-fast.fut:14:33-37
                        
                        bool loop_cond_t_res_21087 = tmp_21083 == (int64_t) 0;
                        
                        // stdlib/bigint-fast.fut:98:19-56
                        
                        bool x_21088 = cond_21086 && loop_cond_t_res_21087;
                        bool loop_while_tmp_22920 = x_21088;
                        int64_t i_tmp_22921 = tmp_21079;
                        int64_t result_tmp_22922 = tmp_21083;
                        
                        loop_while_21076 = loop_while_tmp_22920;
                        i_21077 = i_tmp_22921;
                        result_21078 = result_tmp_22922;
                    }
                    compare_abs_res_f_res_21073 = loop_while_21076;
                    compare_abs_res_f_res_21074 = i_21077;
                    compare_abs_res_f_res_21075 = result_21078;
                    compare_abs_res_21068 = compare_abs_res_f_res_21075;
                }
                if (memblock_unref(ctx, &ext_mem_22669, "ext_mem_22669") != 0)
                    return 1;
                // stdlib/bigint-fast.fut:98:57-61
                
                bool loop_cond_21089 = sle64(compare_abs_res_21068, (int64_t) 0);
                
                if (memblock_set(ctx, &mem_param_tmp_22915, &ext_mem_22668, "ext_mem_22668") != 0)
                    return 1;
                
                int64_t loop_dz2082Uz2088U_tmp_22916 = tmp_21062;
                bool loop_while_tmp_22917 = loop_cond_21089;
                int32_t digit_tmp_22918 = tmp_21061;
                
                if (memblock_set(ctx, &mem_param_22665, &mem_param_tmp_22915, "mem_param_tmp_22915") != 0)
                    return 1;
                loop_dz2082Uz2088U_21057 = loop_dz2082Uz2088U_tmp_22916;
                loop_while_21058 = loop_while_tmp_22917;
                digit_21059 = digit_tmp_22918;
            }
            if (memblock_set(ctx, &ext_mem_22672, &mem_param_22665, "mem_param_22665") != 0)
                return 1;
            quotient_res_f_res_f_res_21053 = loop_dz2082Uz2088U_21057;
            quotient_res_f_res_f_res_21054 = loop_while_21058;
            quotient_res_f_res_f_res_21055 = digit_21059;
            if (memblock_unref(ctx, &ext_mem_22659, "ext_mem_22659") != 0)
                return 1;
            if (memblock_alloc(ctx, &mem_22674, bytes_22595, "mem_22674")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint-fast.fut:31:54-79
            for (int64_t nest_i_22923 = 0; nest_i_22923 < arithmetic_res_20358; nest_i_22923++) {
                ((int32_t *) mem_22674.mem)[nest_i_22923] = 0;
            }
            // stdlib/bigint-fast.fut:31:33-35:39
            
            int64_t sub_abs_res_21092;
            int64_t borrow_21095 = (int64_t) 0;
            
            for (int64_t i_21093 = 0; i_21093 < arithmetic_res_20358; i_21093++) {
                // stdlib/bigint-fast.fut:32:21-25
                
                int32_t u32_arg0_21096 = ((int32_t *) ext_mem_22581.mem)[i_21093];
                
                // stdlib/bigint-fast.fut:32:13-25
                
                int64_t u32_res_21097 = zext_i32_i64(u32_arg0_21096);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                bool cond_21098 = slt64(i_21093, quotient_res_f_res_f_res_21053);
                
                // stdlib/bigint-fast.fut:33:14-54
                
                int64_t zp_lhs_21099;
                
                if (cond_21098) {
                    // stdlib/bigint-fast.fut:33:43-47
                    
                    int32_t u32_arg0_22481 = ((int32_t *) ext_mem_22672.mem)[i_21093];
                    
                    // stdlib/bigint-fast.fut:33:35-47
                    
                    int64_t u32_res_22482 = zext_i32_i64(u32_arg0_22481);
                    
                    zp_lhs_21099 = u32_res_22482;
                } else {
                    zp_lhs_21099 = (int64_t) 0;
                }
                // stdlib/bigint-fast.fut:33:55-62
                
                int64_t y_21102 = add64(borrow_21095, zp_lhs_21099);
                
                // stdlib/bigint-fast.fut:34:28-30
                
                int64_t u64_arg0_21103 = sub64(u32_res_21097, y_21102);
                
                // stdlib/bigint-fast.fut:34:18-30
                
                int32_t u64_res_21104 = zext_i64_i32(u64_arg0_21103);
                
                // stdlib/bigint-fast.fut:34:9-31
                ((int32_t *) mem_22674.mem)[i_21093] = u64_res_21104;
                // stdlib/bigint-fast.fut:35:13-38
                
                bool cond_21106 = ult64(u32_res_21097, y_21102);
                
                // stdlib/bigint-fast.fut:99:27-46
                
                int64_t tmp_21107 = btoi_bool_i64(cond_21106);
                int64_t borrow_tmp_22925 = tmp_21107;
                
                borrow_21095 = borrow_tmp_22925;
            }
            sub_abs_res_21092 = borrow_21095;
            if (memblock_unref(ctx, &ext_mem_22672, "ext_mem_22672") != 0)
                return 1;
            // stdlib/bigint-fast.fut:36:7-26
            
            int64_t sub_abs_res_21108;
            bool sub_abs_res_21109;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22683, &sub_abs_res_21108, &sub_abs_res_21109, mem_22674, arithmetic_res_20358, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_22674, "mem_22674") != 0)
                return 1;
            // stdlib/bigint-fast.fut:102:19-103:50
            
            bool cond_21115 = sub_abs_res_21108 == (int64_t) 1;
            
            // stdlib/bigint-fast.fut:102:47-59
            
            bool y_21116 = slt64((int64_t) 0, sub_abs_res_21108);
            
            // probes/stdlib-fast-tests.fut:6:7-8:60
            
            bool loop_not_taken_21117 = !cond_21115;
            
            // probes/stdlib-fast-tests.fut:6:7-8:60
            
            bool protect_assert_disj_21118 = y_21116 || loop_not_taken_21117;
            
            // probes/stdlib-fast-tests.fut:6:7-8:60
            
            bool protect_assert_disj_21119 = cond_20364 || protect_assert_disj_21118;
            
            // stdlib/bigint-fast.fut:102:47-59
            
            bool index_certs_21120;
            
            if (!protect_assert_disj_21119) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) sub_abs_res_21108, "].", "-> #0  stdlib/bigint-fast.fut:102:47-59\n   #1  probes/stdlib-fast-tests.fut:6:7-8:60\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            if (memblock_alloc(ctx, &mem_22684, (int64_t) 4, "mem_22684")) {
                err = 1;
                goto cleanup;
            }
            // probes/stdlib-fast-tests.fut:6:7-8:60
            for (int64_t nest_i_22926 = 0; nest_i_22926 < (int64_t) 1; nest_i_22926++) {
                ((int32_t *) mem_22684.mem)[nest_i_22926] = quotient_res_f_res_f_res_21055;
            }
            // stdlib/bigint-fast.fut:100:24-47
            
            int64_t quotient_res_f_res_f_res_21112;
            bool quotient_res_f_res_f_res_21113;
            
            if (futrts_normalise_11622(ctx, &ext_mem_22685, &quotient_res_f_res_f_res_21112, &quotient_res_f_res_f_res_21113, mem_22684, (int64_t) 1, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_22684, "mem_22684") != 0)
                return 1;
            // probes/stdlib-fast-tests.fut:6:7-8:60
            
            bool protect_cond_conj_21121 = x_20365 && cond_21115;
            
            // stdlib/bigint-fast.fut:102:47-59
            
            int32_t zeze_lhs_21122;
            
            if (protect_cond_conj_21121) {
                // probes/stdlib-fast-tests.fut:6:7-8:60
                
                int32_t x_22483 = ((int32_t *) ext_mem_22683.mem)[(int64_t) 0];
                
                zeze_lhs_21122 = x_22483;
            } else {
                zeze_lhs_21122 = 0;
            }
            if (memblock_unref(ctx, &ext_mem_22683, "ext_mem_22683") != 0)
                return 1;
            // stdlib/bigint-fast.fut:102:60-64
            
            bool cond_t_res_21124 = zeze_lhs_21122 == 0;
            
            // probes/stdlib-fast-tests.fut:6:7-8:60
            
            bool x_21125 = cond_21115 && cond_t_res_21124;
            
            // stdlib/bigint-fast.fut:41:34-54
            
            bool normalise_arg_21126 = !quotient_res_f_res_f_res_21113;
            int64_t bytes_22699 = (int64_t) 4 * quotient_res_f_res_f_res_21112;
            
            // stdlib/bigint-fast.fut:101:14-103:50
            
            int64_t quotient_res_f_res_f_res_21127;
            bool quotient_res_f_res_f_res_21128;
            
            if (cond_20364) {
                if (memblock_set(ctx, &ext_mem_22718, &ext_mem_22685, "ext_mem_22685") != 0)
                    return 1;
                quotient_res_f_res_f_res_21127 = quotient_res_f_res_f_res_21112;
                quotient_res_f_res_f_res_21128 = quotient_res_f_res_f_res_21113;
            } else {
                // stdlib/bigint-fast.fut:102:19-103:50
                
                int64_t quotient_res_f_res_f_res_f_res_21130;
                bool quotient_res_f_res_f_res_f_res_21131;
                
                if (x_21125) {
                    // stdlib/bigint-fast.fut:41:34-54
                    
                    int64_t negate_res_22484;
                    bool negate_res_22485;
                    
                    if (futrts_normalise_11622(ctx, &ext_mem_22716, &negate_res_22484, &negate_res_22485, ext_mem_22685, quotient_res_f_res_f_res_21112, normalise_arg_21126) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_set(ctx, &ext_mem_22717, &ext_mem_22716, "ext_mem_22716") != 0)
                        return 1;
                    quotient_res_f_res_f_res_f_res_21130 = negate_res_22484;
                    quotient_res_f_res_f_res_f_res_21131 = negate_res_22485;
                } else {
                    // stdlib/bigint-fast.fut:103:39-48
                    
                    int64_t quotient_res_f_res_f_res_f_res_f_res_21136;
                    bool quotient_res_f_res_f_res_f_res_f_res_21137;
                    
                    if (futrts_integer_6855(ctx, &ext_mem_22686, &quotient_res_f_res_f_res_f_res_f_res_21136, &quotient_res_f_res_f_res_f_res_f_res_21137, (int64_t) 1) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/bigint-fast.fut:38:3-40:39
                    
                    bool cond_21139 = quotient_res_f_res_f_res_21113 == quotient_res_f_res_f_res_f_res_f_res_21137;
                    
                    // stdlib/bigint-fast.fut:100:15-103:48
                    
                    bool x_21140 = !cond_21139;
                    
                    // stdlib/bigint-fast.fut:39:11-30
                    
                    int64_t zgze_lhs_21141;
                    
                    if (x_21140) {
                        // stdlib/bigint-fast.fut:12:3-16:17
                        
                        bool cond_22503 = quotient_res_f_res_f_res_21112 == quotient_res_f_res_f_res_f_res_f_res_21136;
                        
                        // stdlib/bigint-fast.fut:12:3-16:17
                        
                        bool cond_22504 = !cond_22503;
                        
                        // stdlib/bigint-fast.fut:13:63-65
                        
                        int64_t tmp_22505 = sub64(quotient_res_f_res_f_res_21112, (int64_t) 1);
                        
                        // stdlib/bigint-fast.fut:13:36-15:74
                        
                        bool cond_22506 = sle64((int64_t) 0, tmp_22505);
                        int64_t compare_abs_res_f_res_22507;
                        
                        if (cond_22503) {
                            bool x_22508;
                            int64_t x_22509;
                            int64_t x_22510;
                            bool loop_while_22511;
                            int64_t i_22512;
                            int64_t result_22513;
                            
                            loop_while_22511 = cond_22506;
                            i_22512 = tmp_22505;
                            result_22513 = (int64_t) 0;
                            while (loop_while_22511) {
                                // stdlib/bigint-fast.fut:15:14-16
                                
                                int64_t tmp_22514 = sub64(i_22512, (int64_t) 1);
                                
                                // stdlib/bigint-fast.fut:15:20-24
                                
                                int32_t zl_lhs_22515 = ((int32_t *) ext_mem_22685.mem)[i_22512];
                                
                                // stdlib/bigint-fast.fut:15:27-31
                                
                                int32_t zl_rhs_22516 = ((int32_t *) ext_mem_22686.mem)[i_22512];
                                
                                // stdlib/bigint-fast.fut:15:17-73
                                
                                bool cond_22517 = ult32(zl_lhs_22515, zl_rhs_22516);
                                
                                // stdlib/bigint-fast.fut:15:17-73
                                
                                int64_t tmp_22518;
                                
                                if (cond_22517) {
                                    tmp_22518 = (int64_t) -1;
                                } else {
                                    // stdlib/bigint-fast.fut:15:45-73
                                    
                                    bool cond_22519 = ult32(zl_rhs_22516, zl_lhs_22515);
                                    
                                    // stdlib/bigint-fast.fut:100:15-103:48
                                    
                                    int64_t tmp_f_res_22520 = btoi_bool_i64(cond_22519);
                                    
                                    tmp_22518 = tmp_f_res_22520;
                                }
                                // stdlib/bigint-fast.fut:14:16-37
                                
                                bool cond_22521 = sle64((int64_t) 0, tmp_22514);
                                
                                // stdlib/bigint-fast.fut:14:33-37
                                
                                bool loop_cond_t_res_22522 = tmp_22518 == (int64_t) 0;
                                
                                // stdlib/bigint-fast.fut:100:15-103:48
                                
                                bool x_22523 = cond_22521 && loop_cond_t_res_22522;
                                bool loop_while_tmp_22927 = x_22523;
                                int64_t i_tmp_22928 = tmp_22514;
                                int64_t result_tmp_22929 = tmp_22518;
                                
                                loop_while_22511 = loop_while_tmp_22927;
                                i_22512 = i_tmp_22928;
                                result_22513 = result_tmp_22929;
                            }
                            x_22508 = loop_while_22511;
                            x_22509 = i_22512;
                            x_22510 = result_22513;
                            compare_abs_res_f_res_22507 = x_22510;
                        } else {
                            compare_abs_res_f_res_22507 = (int64_t) 0;
                        }
                        // stdlib/bigint-fast.fut:12:32-69
                        
                        bool cond_22524 = slt64(quotient_res_f_res_f_res_21112, quotient_res_f_res_f_res_f_res_f_res_21136);
                        
                        // stdlib/bigint-fast.fut:12:32-69
                        
                        int64_t compare_abs_res_t_res_22525;
                        
                        if (cond_22524) {
                            compare_abs_res_t_res_22525 = (int64_t) -1;
                        } else {
                            compare_abs_res_t_res_22525 = (int64_t) 1;
                        }
                        // stdlib/bigint-fast.fut:12:3-16:17
                        
                        int64_t compare_abs_res_22526;
                        
                        if (cond_22504) {
                            compare_abs_res_22526 = compare_abs_res_t_res_22525;
                        } else {
                            compare_abs_res_22526 = compare_abs_res_f_res_22507;
                        }
                        zgze_lhs_21141 = compare_abs_res_22526;
                    } else {
                        zgze_lhs_21141 = (int64_t) 0;
                    }
                    // stdlib/bigint-fast.fut:39:8-40:39
                    
                    bool cond_21166 = sle64((int64_t) 0, zgze_lhs_21141);
                    int64_t bytes_22687 = (int64_t) 4 * quotient_res_f_res_f_res_f_res_f_res_21136;
                    
                    // stdlib/bigint-fast.fut:38:3-40:39
                    
                    int64_t add_res_21167;
                    bool add_res_21168;
                    
                    if (cond_21139) {
                        // stdlib/bigint-fast.fut:38:37-52
                        
                        int64_t normalise_arg1_22527;
                        
                        if (futrts_add_abs_11626(ctx, &ext_mem_22712, &normalise_arg1_22527, ext_mem_22685, ext_mem_22686, quotient_res_f_res_f_res_21112, quotient_res_f_res_f_res_f_res_f_res_21136) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        // stdlib/bigint-fast.fut:38:22-52
                        
                        int64_t add_res_t_res_22529;
                        bool add_res_t_res_22530;
                        
                        if (futrts_normalise_11622(ctx, &ext_mem_22713, &add_res_t_res_22529, &add_res_t_res_22530, ext_mem_22712, normalise_arg1_22527, quotient_res_f_res_f_res_21113) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        if (memblock_unref(ctx, &ext_mem_22712, "ext_mem_22712") != 0)
                            return 1;
                        if (memblock_set(ctx, &ext_mem_22714, &ext_mem_22713, "ext_mem_22713") != 0)
                            return 1;
                        add_res_21167 = add_res_t_res_22529;
                        add_res_21168 = add_res_t_res_22530;
                    } else {
                        // stdlib/bigint-fast.fut:39:8-40:39
                        
                        int64_t add_res_f_res_21175;
                        bool add_res_f_res_21176;
                        
                        if (cond_21166) {
                            if (memblock_alloc(ctx, &mem_22700, bytes_22699, "mem_22700")) {
                                err = 1;
                                goto cleanup;
                            }
                            // stdlib/bigint-fast.fut:31:54-79
                            for (int64_t nest_i_22930 = 0; nest_i_22930 < quotient_res_f_res_f_res_21112; nest_i_22930++) {
                                ((int32_t *) mem_22700.mem)[nest_i_22930] = 0;
                            }
                            // stdlib/bigint-fast.fut:31:33-35:39
                            
                            int64_t sub_abs_res_22536;
                            int64_t borrow_22539 = (int64_t) 0;
                            
                            for (int64_t i_22537 = 0; i_22537 < quotient_res_f_res_f_res_21112; i_22537++) {
                                // stdlib/bigint-fast.fut:32:21-25
                                
                                int32_t u32_arg0_22540 = ((int32_t *) ext_mem_22685.mem)[i_22537];
                                
                                // stdlib/bigint-fast.fut:32:13-25
                                
                                int64_t u32_res_22541 = zext_i32_i64(u32_arg0_22540);
                                
                                // stdlib/bigint-fast.fut:33:14-54
                                
                                bool cond_22542 = slt64(i_22537, quotient_res_f_res_f_res_f_res_f_res_21136);
                                
                                // stdlib/bigint-fast.fut:33:14-54
                                
                                int64_t zp_lhs_22543;
                                
                                if (cond_22542) {
                                    // stdlib/bigint-fast.fut:33:43-47
                                    
                                    int32_t u32_arg0_22544 = ((int32_t *) ext_mem_22686.mem)[i_22537];
                                    
                                    // stdlib/bigint-fast.fut:33:35-47
                                    
                                    int64_t u32_res_22545 = zext_i32_i64(u32_arg0_22544);
                                    
                                    zp_lhs_22543 = u32_res_22545;
                                } else {
                                    zp_lhs_22543 = (int64_t) 0;
                                }
                                // stdlib/bigint-fast.fut:33:55-62
                                
                                int64_t y_22546 = add64(borrow_22539, zp_lhs_22543);
                                
                                // stdlib/bigint-fast.fut:34:28-30
                                
                                int64_t u64_arg0_22547 = sub64(u32_res_22541, y_22546);
                                
                                // stdlib/bigint-fast.fut:34:18-30
                                
                                int32_t u64_res_22548 = zext_i64_i32(u64_arg0_22547);
                                
                                // stdlib/bigint-fast.fut:34:9-31
                                ((int32_t *) mem_22700.mem)[i_22537] = u64_res_22548;
                                // stdlib/bigint-fast.fut:35:13-38
                                
                                bool cond_22550 = ult64(u32_res_22541, y_22546);
                                
                                // stdlib/bigint-fast.fut:39:56-71
                                
                                int64_t tmp_22551 = btoi_bool_i64(cond_22550);
                                int64_t borrow_tmp_22932 = tmp_22551;
                                
                                borrow_22539 = borrow_tmp_22932;
                            }
                            sub_abs_res_22536 = borrow_22539;
                            // stdlib/bigint-fast.fut:36:7-26
                            
                            int64_t sub_abs_res_22552;
                            bool sub_abs_res_22553;
                            
                            if (futrts_normalise_11622(ctx, &ext_mem_22709, &sub_abs_res_22552, &sub_abs_res_22553, mem_22700, quotient_res_f_res_f_res_21112, 0) != 0) {
                                err = 1;
                                goto cleanup;
                            }
                            if (memblock_unref(ctx, &mem_22700, "mem_22700") != 0)
                                return 1;
                            // stdlib/bigint-fast.fut:39:41-71
                            
                            int64_t add_res_f_res_t_res_22555;
                            bool add_res_f_res_t_res_22556;
                            
                            if (futrts_normalise_11622(ctx, &ext_mem_22710, &add_res_f_res_t_res_22555, &add_res_f_res_t_res_22556, ext_mem_22709, sub_abs_res_22552, quotient_res_f_res_f_res_21113) != 0) {
                                err = 1;
                                goto cleanup;
                            }
                            if (memblock_unref(ctx, &ext_mem_22709, "ext_mem_22709") != 0)
                                return 1;
                            if (memblock_set(ctx, &ext_mem_22711, &ext_mem_22710, "ext_mem_22710") != 0)
                                return 1;
                            add_res_f_res_21175 = add_res_f_res_t_res_22555;
                            add_res_f_res_21176 = add_res_f_res_t_res_22556;
                        } else {
                            if (memblock_alloc(ctx, &mem_22688, bytes_22687, "mem_22688")) {
                                err = 1;
                                goto cleanup;
                            }
                            // stdlib/bigint-fast.fut:31:54-79
                            for (int64_t nest_i_22933 = 0; nest_i_22933 < quotient_res_f_res_f_res_f_res_f_res_21136; nest_i_22933++) {
                                ((int32_t *) mem_22688.mem)[nest_i_22933] = 0;
                            }
                            // stdlib/bigint-fast.fut:31:33-35:39
                            
                            int64_t sub_abs_res_21204;
                            int64_t borrow_21207 = (int64_t) 0;
                            
                            for (int64_t i_21205 = 0; i_21205 < quotient_res_f_res_f_res_f_res_f_res_21136; i_21205++) {
                                // stdlib/bigint-fast.fut:32:21-25
                                
                                int32_t u32_arg0_21208 = ((int32_t *) ext_mem_22686.mem)[i_21205];
                                
                                // stdlib/bigint-fast.fut:32:13-25
                                
                                int64_t u32_res_21209 = zext_i32_i64(u32_arg0_21208);
                                
                                // stdlib/bigint-fast.fut:33:14-54
                                
                                bool cond_21210 = slt64(i_21205, quotient_res_f_res_f_res_21112);
                                
                                // stdlib/bigint-fast.fut:33:14-54
                                
                                int64_t zp_lhs_21211;
                                
                                if (cond_21210) {
                                    // stdlib/bigint-fast.fut:33:43-47
                                    
                                    int32_t u32_arg0_22558 = ((int32_t *) ext_mem_22685.mem)[i_21205];
                                    
                                    // stdlib/bigint-fast.fut:33:35-47
                                    
                                    int64_t u32_res_22559 = zext_i32_i64(u32_arg0_22558);
                                    
                                    zp_lhs_21211 = u32_res_22559;
                                } else {
                                    zp_lhs_21211 = (int64_t) 0;
                                }
                                // stdlib/bigint-fast.fut:33:55-62
                                
                                int64_t y_21214 = add64(borrow_21207, zp_lhs_21211);
                                
                                // stdlib/bigint-fast.fut:34:28-30
                                
                                int64_t u64_arg0_21215 = sub64(u32_res_21209, y_21214);
                                
                                // stdlib/bigint-fast.fut:34:18-30
                                
                                int32_t u64_res_21216 = zext_i64_i32(u64_arg0_21215);
                                
                                // stdlib/bigint-fast.fut:34:9-31
                                ((int32_t *) mem_22688.mem)[i_21205] = u64_res_21216;
                                // stdlib/bigint-fast.fut:35:13-38
                                
                                bool cond_21218 = ult64(u32_res_21209, y_21214);
                                
                                // stdlib/bigint-fast.fut:40:23-38
                                
                                int64_t tmp_21219 = btoi_bool_i64(cond_21218);
                                int64_t borrow_tmp_22935 = tmp_21219;
                                
                                borrow_21207 = borrow_tmp_22935;
                            }
                            sub_abs_res_21204 = borrow_21207;
                            // stdlib/bigint-fast.fut:36:7-26
                            
                            int64_t sub_abs_res_21220;
                            bool sub_abs_res_21221;
                            
                            if (futrts_normalise_11622(ctx, &ext_mem_22697, &sub_abs_res_21220, &sub_abs_res_21221, mem_22688, quotient_res_f_res_f_res_f_res_f_res_21136, 0) != 0) {
                                err = 1;
                                goto cleanup;
                            }
                            if (memblock_unref(ctx, &mem_22688, "mem_22688") != 0)
                                return 1;
                            // stdlib/bigint-fast.fut:40:8-38
                            
                            int64_t add_res_f_res_f_res_21223;
                            bool add_res_f_res_f_res_21224;
                            
                            if (futrts_normalise_11622(ctx, &ext_mem_22698, &add_res_f_res_f_res_21223, &add_res_f_res_f_res_21224, ext_mem_22697, sub_abs_res_21220, quotient_res_f_res_f_res_f_res_f_res_21137) != 0) {
                                err = 1;
                                goto cleanup;
                            }
                            if (memblock_unref(ctx, &ext_mem_22697, "ext_mem_22697") != 0)
                                return 1;
                            if (memblock_set(ctx, &ext_mem_22711, &ext_mem_22698, "ext_mem_22698") != 0)
                                return 1;
                            add_res_f_res_21175 = add_res_f_res_f_res_21223;
                            add_res_f_res_21176 = add_res_f_res_f_res_21224;
                        }
                        if (memblock_set(ctx, &ext_mem_22714, &ext_mem_22711, "ext_mem_22711") != 0)
                            return 1;
                        add_res_21167 = add_res_f_res_21175;
                        add_res_21168 = add_res_f_res_21176;
                    }
                    if (memblock_unref(ctx, &ext_mem_22686, "ext_mem_22686") != 0)
                        return 1;
                    // stdlib/bigint-fast.fut:41:34-54
                    
                    bool normalise_arg_21226 = !add_res_21168;
                    
                    // stdlib/bigint-fast.fut:41:34-54
                    
                    int64_t negate_res_21227;
                    bool negate_res_21228;
                    
                    if (futrts_normalise_11622(ctx, &ext_mem_22715, &negate_res_21227, &negate_res_21228, ext_mem_22714, add_res_21167, normalise_arg_21226) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &ext_mem_22714, "ext_mem_22714") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_22717, &ext_mem_22715, "ext_mem_22715") != 0)
                        return 1;
                    quotient_res_f_res_f_res_f_res_21130 = negate_res_21227;
                    quotient_res_f_res_f_res_f_res_21131 = negate_res_21228;
                }
                if (memblock_set(ctx, &ext_mem_22718, &ext_mem_22717, "ext_mem_22717") != 0)
                    return 1;
                quotient_res_f_res_f_res_21127 = quotient_res_f_res_f_res_f_res_21130;
                quotient_res_f_res_f_res_21128 = quotient_res_f_res_f_res_f_res_21131;
            }
            if (memblock_unref(ctx, &ext_mem_22685, "ext_mem_22685") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22720, &ext_mem_22718, "ext_mem_22718") != 0)
                return 1;
            quotient_res_f_res_20939 = quotient_res_f_res_f_res_21127;
            quotient_res_f_res_20940 = quotient_res_f_res_f_res_21128;
        }
        if (memblock_set(ctx, &ext_mem_22807, &ext_mem_22720, "ext_mem_22720") != 0)
            return 1;
        quotient_res_20668 = quotient_res_f_res_20939;
        quotient_res_20669 = quotient_res_f_res_20940;
    }
    // stdlib/bigint-fast.fut:18:3-19:51
    
    int64_t compare_res_21230;
    
    if (x_20365) {
        // stdlib/bigint-fast.fut:18:22-43
        
        int64_t compare_res_t_res_22560;
        
        if (arithmetic_res_20359) {
            compare_res_t_res_22560 = (int64_t) -1;
        } else {
            compare_res_t_res_22560 = (int64_t) 1;
        }
        compare_res_21230 = compare_res_t_res_22560;
    } else {
        // stdlib/bigint-fast.fut:19:9-30
        
        int64_t zt_lhs_21232;
        
        if (arithmetic_res_20359) {
            zt_lhs_21232 = (int64_t) -1;
        } else {
            zt_lhs_21232 = (int64_t) 1;
        }
        // stdlib/bigint-fast.fut:12:3-16:17
        
        bool cond_21233 = arithmetic_res_20358 == arithmetic_res_20361;
        
        // stdlib/bigint-fast.fut:12:3-16:17
        
        bool cond_21234 = !cond_21233;
        
        // stdlib/bigint-fast.fut:12:3-16:17
        
        int64_t compare_abs_res_21235;
        
        if (cond_21234) {
            // stdlib/bigint-fast.fut:12:32-69
            
            bool cond_22561 = slt64(arithmetic_res_20358, arithmetic_res_20361);
            
            // stdlib/bigint-fast.fut:12:32-69
            
            int64_t compare_abs_res_t_res_22562;
            
            if (cond_22561) {
                compare_abs_res_t_res_22562 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_22562 = (int64_t) 1;
            }
            compare_abs_res_21235 = compare_abs_res_t_res_22562;
        } else {
            // stdlib/bigint-fast.fut:13:36-15:74
            
            bool compare_abs_res_f_res_21238;
            int64_t compare_abs_res_f_res_21239;
            int64_t compare_abs_res_f_res_21240;
            bool loop_while_21241;
            int64_t i_21242;
            int64_t result_21243;
            
            loop_while_21241 = x_20552;
            i_21242 = u32_arg0_20551;
            result_21243 = (int64_t) 0;
            while (loop_while_21241) {
                // stdlib/bigint-fast.fut:15:14-16
                
                int64_t tmp_21244 = sub64(i_21242, (int64_t) 1);
                
                // stdlib/bigint-fast.fut:15:20-24
                
                int32_t zl_lhs_21245 = ((int32_t *) ext_mem_22581.mem)[i_21242];
                
                // stdlib/bigint-fast.fut:15:27-31
                
                int32_t zl_rhs_21246 = ((int32_t *) ext_mem_22582.mem)[i_21242];
                
                // stdlib/bigint-fast.fut:15:17-73
                
                bool cond_21247 = ult32(zl_lhs_21245, zl_rhs_21246);
                
                // stdlib/bigint-fast.fut:15:17-73
                
                int64_t tmp_21248;
                
                if (cond_21247) {
                    tmp_21248 = (int64_t) -1;
                } else {
                    // stdlib/bigint-fast.fut:15:45-73
                    
                    bool cond_21249 = ult32(zl_rhs_21246, zl_lhs_21245);
                    
                    // stdlib/bigint-fast.fut:19:32-51
                    
                    int64_t tmp_f_res_21250 = btoi_bool_i64(cond_21249);
                    
                    tmp_21248 = tmp_f_res_21250;
                }
                // stdlib/bigint-fast.fut:14:16-37
                
                bool cond_21251 = sle64((int64_t) 0, tmp_21244);
                
                // stdlib/bigint-fast.fut:14:33-37
                
                bool loop_cond_t_res_21252 = tmp_21248 == (int64_t) 0;
                
                // stdlib/bigint-fast.fut:19:32-51
                
                bool x_21253 = cond_21251 && loop_cond_t_res_21252;
                bool loop_while_tmp_22936 = x_21253;
                int64_t i_tmp_22937 = tmp_21244;
                int64_t result_tmp_22938 = tmp_21248;
                
                loop_while_21241 = loop_while_tmp_22936;
                i_21242 = i_tmp_22937;
                result_21243 = result_tmp_22938;
            }
            compare_abs_res_f_res_21238 = loop_while_21241;
            compare_abs_res_f_res_21239 = i_21242;
            compare_abs_res_f_res_21240 = result_21243;
            compare_abs_res_21235 = compare_abs_res_f_res_21240;
        }
        // stdlib/bigint-fast.fut:19:31-51
        
        int64_t compare_res_f_res_21254 = mul64(zt_lhs_21232, compare_abs_res_21235);
        
        compare_res_21230 = compare_res_f_res_21254;
    }
    if (memblock_unref(ctx, &ext_mem_22581, "ext_mem_22581") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &ext_mem_22610, "ext_mem_22610") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22832, &ext_mem_22639, "ext_mem_22639") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22833, &ext_mem_22641, "ext_mem_22641") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22834, &ext_mem_22807, "ext_mem_22807") != 0)
        return 1;
    prim_out_22835 = add_res_20391;
    prim_out_22836 = add_res_20481;
    prim_out_22837 = multiply_small_res_20542;
    prim_out_22838 = quotient_res_20668;
    prim_out_22839 = add_res_20392;
    prim_out_22840 = add_res_20482;
    prim_out_22841 = multiply_small_res_20543;
    prim_out_22842 = quotient_res_20669;
    prim_out_22843 = compare_res_21230;
    if (memblock_set(ctx, &*mem_out_p_22945, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22946, &mem_out_22832, "mem_out_22832") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22947, &mem_out_22833, "mem_out_22833") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22948, &mem_out_22834, "mem_out_22834") != 0)
        return 1;
    *out_prim_out_22949 = prim_out_22835;
    *out_prim_out_22950 = prim_out_22836;
    *out_prim_out_22951 = prim_out_22837;
    *out_prim_out_22952 = prim_out_22838;
    *out_prim_out_22953 = prim_out_22839;
    *out_prim_out_22954 = prim_out_22840;
    *out_prim_out_22955 = prim_out_22841;
    *out_prim_out_22956 = prim_out_22842;
    *out_prim_out_22957 = prim_out_22843;
    
  cleanup:
    {
        if (memblock_unref(ctx, &ext_mem_22715, "ext_mem_22715") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22698, "ext_mem_22698") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22697, "ext_mem_22697") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22688, "mem_22688") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22710, "ext_mem_22710") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22709, "ext_mem_22709") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22700, "mem_22700") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22711, "ext_mem_22711") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22713, "ext_mem_22713") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22712, "ext_mem_22712") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22714, "ext_mem_22714") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22686, "ext_mem_22686") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22716, "ext_mem_22716") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22717, "ext_mem_22717") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22718, "ext_mem_22718") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22685, "ext_mem_22685") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22684, "mem_22684") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22683, "ext_mem_22683") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22674, "mem_22674") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22915, "mem_param_tmp_22915") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22669, "ext_mem_22669") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22668, "ext_mem_22668") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22665, "mem_param_22665") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22672, "ext_mem_22672") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22662, "ext_mem_22662") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22901, "mem_param_tmp_22901") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22656, "ext_mem_22656") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22647, "mem_22647") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22645, "mem_param_22645") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22659, "ext_mem_22659") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22642, "ext_mem_22642") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22719, "ext_mem_22719") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22720, "ext_mem_22720") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22803, "ext_mem_22803") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22786, "ext_mem_22786") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22785, "ext_mem_22785") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22776, "mem_22776") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22798, "ext_mem_22798") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22797, "ext_mem_22797") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22788, "mem_22788") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22799, "ext_mem_22799") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22801, "ext_mem_22801") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22800, "ext_mem_22800") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22802, "ext_mem_22802") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22774, "ext_mem_22774") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22804, "ext_mem_22804") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22805, "ext_mem_22805") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22806, "ext_mem_22806") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22773, "ext_mem_22773") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22866, "mem_param_tmp_22866") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22764, "ext_mem_22764") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22755, "mem_22755") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22875, "mem_param_tmp_22875") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22750, "ext_mem_22750") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22741, "mem_22741") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22739, "mem_param_22739") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22753, "ext_mem_22753") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22736, "ext_mem_22736") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22735, "mem_22735") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22733, "ext_mem_22733") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22730, "mem_param_22730") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22769, "ext_mem_22769") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22724, "mem_22724") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22722, "ext_mem_22722") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22721, "ext_mem_22721") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22807, "ext_mem_22807") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22641, "ext_mem_22641") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22640, "ext_mem_22640") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22623, "ext_mem_22623") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22622, "ext_mem_22622") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22613, "mem_22613") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22635, "ext_mem_22635") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22634, "ext_mem_22634") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22625, "mem_22625") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22636, "ext_mem_22636") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22638, "ext_mem_22638") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22637, "ext_mem_22637") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22639, "ext_mem_22639") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22611, "ext_mem_22611") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22594, "ext_mem_22594") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22593, "ext_mem_22593") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22584, "mem_22584") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22606, "ext_mem_22606") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22605, "ext_mem_22605") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22596, "mem_22596") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22607, "ext_mem_22607") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22609, "ext_mem_22609") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22608, "ext_mem_22608") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22610, "ext_mem_22610") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22581, "ext_mem_22581") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22834, "mem_out_22834") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22833, "mem_out_22833") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22832, "mem_out_22832") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_hashmap_increment(struct futhark_context *ctx, struct memblock *mem_out_p_22958, struct memblock keys_mem_22579, struct memblock deltas_mem_22580, int64_t dz2080U_18149, int64_t dz2081U_18150)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_22621;
    
    mem_22621.references = NULL;
    
    struct memblock mem_param_tmp_22834;
    
    mem_param_tmp_22834.references = NULL;
    
    struct memblock mem_param_tmp_22833;
    
    mem_param_tmp_22833.references = NULL;
    
    struct memblock mem_param_tmp_22832;
    
    mem_param_tmp_22832.references = NULL;
    
    struct memblock ext_mem_22599;
    
    ext_mem_22599.references = NULL;
    
    struct memblock ext_mem_22600;
    
    ext_mem_22600.references = NULL;
    
    struct memblock ext_mem_22601;
    
    ext_mem_22601.references = NULL;
    
    struct memblock ext_mem_22604;
    
    ext_mem_22604.references = NULL;
    
    struct memblock ext_mem_22607;
    
    ext_mem_22607.references = NULL;
    
    struct memblock ext_mem_22610;
    
    ext_mem_22610.references = NULL;
    
    struct memblock mem_param_22592;
    
    mem_param_22592.references = NULL;
    
    struct memblock mem_param_22589;
    
    mem_param_22589.references = NULL;
    
    struct memblock mem_param_22586;
    
    mem_param_22586.references = NULL;
    
    struct memblock ext_mem_22617;
    
    ext_mem_22617.references = NULL;
    
    struct memblock ext_mem_22618;
    
    ext_mem_22618.references = NULL;
    
    struct memblock ext_mem_22619;
    
    ext_mem_22619.references = NULL;
    
    struct memblock ext_mem_22581;
    
    ext_mem_22581.references = NULL;
    
    struct memblock ext_mem_22582;
    
    ext_mem_22582.references = NULL;
    
    struct memblock ext_mem_22583;
    
    ext_mem_22583.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t bytes_22620 = (int64_t) 8 * dz2080U_18149;
    
    // probes/stdlib-fast-tests.fut:18:28-35
    
    int64_t hashmap_increment_res_18925;
    
    if (futrts_empty_8082(ctx, &ext_mem_22583, &ext_mem_22582, &ext_mem_22581, &hashmap_increment_res_18925, (int64_t) 2) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:18:15-92
    
    int64_t hashmap_increment_res_18926;
    int64_t hashmap_increment_res_18930;
    int64_t loop_dz2082U_18932;
    int64_t table_18936;
    
    if (memblock_set(ctx, &mem_param_22586, &ext_mem_22583, "ext_mem_22583") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_22589, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_22592, &ext_mem_22581, "ext_mem_22581") != 0)
        return 1;
    loop_dz2082U_18932 = (int64_t) 2;
    table_18936 = hashmap_increment_res_18925;
    for (int64_t i_18931 = 0; i_18931 < dz2080U_18149; i_18931++) {
        // probes/stdlib-fast-tests.fut:18:83-92
        
        bool y_18937 = slt64(i_18931, dz2081U_18150);
        
        // probes/stdlib-fast-tests.fut:18:83-92
        
        bool index_certs_18938;
        
        if (!y_18937) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18931, "] out of bounds for array of shape [", (long long) dz2081U_18150, "].", "-> #0  probes/stdlib-fast-tests.fut:18:83-92\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // probes/stdlib-fast-tests.fut:18:75-82
        
        int64_t increment_arg1_18942 = ((int64_t *) keys_mem_22579.mem)[i_18931];
        
        // stdlib/hashmap.fut:34:18-20
        
        int64_t zt_lhs_18943 = add64((int64_t) 1, table_18936);
        
        // stdlib/hashmap.fut:34:21-24
        
        int64_t zg_lhs_18944 = mul64((int64_t) 10, zt_lhs_18943);
        
        // stdlib/hashmap.fut:34:37-39
        
        int64_t zg_rhs_18945 = mul64((int64_t) 7, loop_dz2082U_18932);
        
        // stdlib/hashmap.fut:34:11-70
        
        bool cond_18946 = slt64(zg_rhs_18945, zg_lhs_18944);
        int64_t increment_res_18947;
        int64_t increment_res_18951;
        
        if (cond_18946) {
            // stdlib/hashmap.fut:33:16-34:61
            
            int64_t increment_res_t_res_21918;
            int64_t increment_res_t_res_21922;
            
            if (futrts_grow_11710(ctx, &ext_mem_22601, &ext_mem_22600, &ext_mem_22599, &increment_res_t_res_21918, &increment_res_t_res_21922, mem_param_22586, mem_param_22589, mem_param_22592, loop_dz2082U_18932, table_18936) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_22610, &ext_mem_22601, "ext_mem_22601") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22607, &ext_mem_22600, "ext_mem_22600") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22604, &ext_mem_22599, "ext_mem_22599") != 0)
                return 1;
            increment_res_18947 = increment_res_t_res_21918;
            increment_res_18951 = increment_res_t_res_21922;
        } else {
            if (memblock_set(ctx, &ext_mem_22610, &mem_param_22586, "mem_param_22586") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22607, &mem_param_22589, "mem_param_22589") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22604, &mem_param_22592, "mem_param_22592") != 0)
                return 1;
            increment_res_18947 = loop_dz2082U_18932;
            increment_res_18951 = table_18936;
        }
        // stdlib/hashmap.fut:11:26-28
        
        int64_t mask_21861 = sub64(increment_res_18947, (int64_t) 1);
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_21862 = lshr64(increment_arg1_18942, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_21863 = increment_arg1_18942 ^ zc_rhs_21862;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_21864 = mul64((int64_t) -4658895280553007687, zt_lhs_21863);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_21865 = lshr64(x_21864, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_21866 = x_21864 ^ zc_rhs_21865;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_21867 = mul64((int64_t) -7723592293110705685, zt_lhs_21866);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_21868 = lshr64(x_21867, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_21869 = x_21867 ^ zc_rhs_21868;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_21870 = mask_21861 & hash_res_21869;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21871 = ((bool *) ext_mem_22604.mem)[u64_arg0_21870];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21872;
        
        if (loop_cond_21871) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_21923 = ((int64_t *) ext_mem_22610.mem)[u64_arg0_21870];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21924 = loop_cond_t_res_21923 == increment_arg1_18942;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21925 = !loop_cond_t_res_21924;
            
            loop_cond_21872 = loop_cond_t_res_21925;
        } else {
            loop_cond_21872 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_21876;
        int64_t slot_res_21877;
        bool loop_while_21878;
        int64_t i_21879;
        
        loop_while_21878 = loop_cond_21872;
        i_21879 = u64_arg0_21870;
        while (loop_while_21878) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_21880 = add64((int64_t) 1, i_21879);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_21881 = mask_21861 & za_lhs_21880;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_21882 = ((bool *) ext_mem_22604.mem)[loopres_21881];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_21883;
            
            if (loop_cond_21882) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_21926 = ((int64_t *) ext_mem_22610.mem)[loopres_21881];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21927 = loop_cond_t_res_21926 == increment_arg1_18942;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21928 = !loop_cond_t_res_21927;
                
                loop_cond_21883 = loop_cond_t_res_21928;
            } else {
                loop_cond_21883 = 0;
            }
            
            bool loop_while_tmp_22840 = loop_cond_21883;
            int64_t i_tmp_22841 = loopres_21881;
            
            loop_while_21878 = loop_while_tmp_22840;
            i_21879 = i_tmp_22841;
        }
        slot_res_21876 = loop_while_21878;
        slot_res_21877 = i_21879;
        // stdlib/hashmap.fut:38:7-62
        
        bool x_18962 = sle64((int64_t) 0, slot_res_21877);
        
        // stdlib/hashmap.fut:38:7-62
        
        bool y_18963 = slt64(slot_res_21877, increment_res_18947);
        
        // stdlib/hashmap.fut:38:7-62
        
        bool bounds_check_18964 = x_18962 && y_18963;
        
        // stdlib/hashmap.fut:38:7-62
        
        bool index_certs_18965;
        
        if (!bounds_check_18964) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_21877, "] out of bounds for array of shape [", (long long) increment_res_18947, "].", "-> #0  stdlib/hashmap.fut:38:7-62\n   #1  probes/stdlib-fast-tests.fut:18:59-92\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // probes/stdlib-fast-tests.fut:18:83-92
        
        int64_t increment_arg2_18939 = ((int64_t *) deltas_mem_22580.mem)[i_18931];
        
        // stdlib/hashmap.fut:37:40-48
        
        int64_t increment_res_18958 = ((int64_t *) ext_mem_22607.mem)[slot_res_21877];
        
        // stdlib/hashmap.fut:37:40-48
        
        bool increment_res_18959 = ((bool *) ext_mem_22604.mem)[slot_res_21877];
        
        // stdlib/hashmap.fut:38:24-49
        
        int64_t zp_lhs_18960;
        
        if (increment_res_18959) {
            zp_lhs_18960 = increment_res_18958;
        } else {
            zp_lhs_18960 = (int64_t) 0;
        }
        // stdlib/hashmap.fut:38:50-56
        
        int64_t tmp_18961 = add64(increment_arg2_18939, zp_lhs_18960);
        
        // stdlib/hashmap.fut:38:7-62
        ((int64_t *) ext_mem_22610.mem)[slot_res_21877] = increment_arg1_18942;
        // stdlib/hashmap.fut:38:7-62
        ((int64_t *) ext_mem_22607.mem)[slot_res_21877] = tmp_18961;
        // stdlib/hashmap.fut:38:7-62
        ((bool *) ext_mem_22604.mem)[slot_res_21877] = 1;
        // probes/stdlib-fast-tests.fut:18:59-92
        
        bool cond_neg_18969 = !increment_res_18959;
        
        // probes/stdlib-fast-tests.fut:18:59-92
        
        int64_t zp_rhs_18970 = btoi_bool_i64(cond_neg_18969);
        
        // stdlib/hashmap.fut:39:17-40
        
        int64_t tmp_18971 = add64(increment_res_18951, zp_rhs_18970);
        
        if (memblock_set(ctx, &mem_param_tmp_22832, &ext_mem_22610, "ext_mem_22610") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22833, &ext_mem_22607, "ext_mem_22607") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22834, &ext_mem_22604, "ext_mem_22604") != 0)
            return 1;
        
        int64_t loop_dz2082U_tmp_22835 = increment_res_18947;
        int64_t table_tmp_22839 = tmp_18971;
        
        if (memblock_set(ctx, &mem_param_22586, &mem_param_tmp_22832, "mem_param_tmp_22832") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22589, &mem_param_tmp_22833, "mem_param_tmp_22833") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22592, &mem_param_tmp_22834, "mem_param_tmp_22834") != 0)
            return 1;
        loop_dz2082U_18932 = loop_dz2082U_tmp_22835;
        table_18936 = table_tmp_22839;
    }
    if (memblock_set(ctx, &ext_mem_22619, &mem_param_22586, "mem_param_22586") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_22618, &mem_param_22589, "mem_param_22589") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_22617, &mem_param_22592, "mem_param_22592") != 0)
        return 1;
    hashmap_increment_res_18926 = loop_dz2082U_18932;
    hashmap_increment_res_18930 = table_18936;
    if (memblock_unref(ctx, &ext_mem_22581, "ext_mem_22581") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22583, "ext_mem_22583") != 0)
        return 1;
    // stdlib/hashmap.fut:11:26-28
    
    int64_t mask_21892 = sub64(hashmap_increment_res_18926, (int64_t) 1);
    
    if (memblock_alloc(ctx, &mem_22621, bytes_22620, "mem_22621")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:19:6-26
    for (int64_t i_22565 = 0; i_22565 < dz2080U_18149; i_22565++) {
        int64_t eta_p_18973 = ((int64_t *) keys_mem_22579.mem)[i_22565];
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_21893 = lshr64(eta_p_18973, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_21894 = eta_p_18973 ^ zc_rhs_21893;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_21895 = mul64((int64_t) -4658895280553007687, zt_lhs_21894);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_21896 = lshr64(x_21895, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_21897 = x_21895 ^ zc_rhs_21896;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_21898 = mul64((int64_t) -7723592293110705685, zt_lhs_21897);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_21899 = lshr64(x_21898, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_21900 = x_21898 ^ zc_rhs_21899;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_21901 = mask_21892 & hash_res_21900;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21902 = ((bool *) ext_mem_22617.mem)[u64_arg0_21901];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21903;
        
        if (loop_cond_21902) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_21929 = ((int64_t *) ext_mem_22619.mem)[u64_arg0_21901];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21930 = loop_cond_t_res_21929 == eta_p_18973;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21931 = !loop_cond_t_res_21930;
            
            loop_cond_21903 = loop_cond_t_res_21931;
        } else {
            loop_cond_21903 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_21907;
        int64_t slot_res_21908;
        bool loop_while_21909;
        int64_t i_21910;
        
        loop_while_21909 = loop_cond_21903;
        i_21910 = u64_arg0_21901;
        while (loop_while_21909) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_21911 = add64((int64_t) 1, i_21910);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_21912 = mask_21892 & za_lhs_21911;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_21913 = ((bool *) ext_mem_22617.mem)[loopres_21912];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_21914;
            
            if (loop_cond_21913) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_21932 = ((int64_t *) ext_mem_22619.mem)[loopres_21912];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21933 = loop_cond_t_res_21932 == eta_p_18973;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21934 = !loop_cond_t_res_21933;
                
                loop_cond_21914 = loop_cond_t_res_21934;
            } else {
                loop_cond_21914 = 0;
            }
            
            bool loop_while_tmp_22843 = loop_cond_21914;
            int64_t i_tmp_22844 = loopres_21912;
            
            loop_while_21909 = loop_while_tmp_22843;
            i_21910 = i_tmp_22844;
        }
        slot_res_21907 = loop_while_21909;
        slot_res_21908 = i_21910;
        // stdlib/hashmap.fut:17:9-17
        
        bool x_18975 = sle64((int64_t) 0, slot_res_21908);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool y_18976 = slt64(slot_res_21908, hashmap_increment_res_18926);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool bounds_check_18977 = x_18975 && y_18976;
        
        // stdlib/hashmap.fut:17:9-17
        
        bool index_certs_18978;
        
        if (!bounds_check_18977) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_21908, "] out of bounds for array of shape [", (long long) hashmap_increment_res_18926, "].", "-> #0  stdlib/hashmap.fut:17:9-17\n   #1  probes/stdlib-fast-tests.fut:19:11-14\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:17:9-17
        
        bool get_res_18979 = ((bool *) ext_mem_22617.mem)[slot_res_21908];
        
        // stdlib/hashmap.fut:17:6-42
        
        int64_t get_res_18980;
        
        if (get_res_18979) {
            // stdlib/hashmap.fut:17:25-33
            
            int64_t get_res_t_res_21935 = ((int64_t *) ext_mem_22618.mem)[slot_res_21908];
            
            get_res_18980 = get_res_t_res_21935;
        } else {
            get_res_18980 = (int64_t) 0;
        }
        ((int64_t *) mem_22621.mem)[i_22565] = get_res_18980;
    }
    if (memblock_unref(ctx, &ext_mem_22617, "ext_mem_22617") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22618, "ext_mem_22618") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22619, "ext_mem_22619") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &mem_22621, "mem_22621") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22958, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_22621, "mem_22621") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22834, "mem_param_tmp_22834") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22833, "mem_param_tmp_22833") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22832, "mem_param_tmp_22832") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22599, "ext_mem_22599") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22600, "ext_mem_22600") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22601, "ext_mem_22601") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22604, "ext_mem_22604") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22607, "ext_mem_22607") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22610, "ext_mem_22610") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22592, "mem_param_22592") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22589, "mem_param_22589") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22586, "mem_param_22586") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22617, "ext_mem_22617") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22618, "ext_mem_22618") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22619, "ext_mem_22619") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22581, "ext_mem_22581") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22583, "ext_mem_22583") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_hashmap_reserved(struct futhark_context *ctx, struct memblock *mem_out_p_22959, struct memblock keys_mem_22579, struct memblock deltas_mem_22580, int64_t dz2080U_18411, int64_t dz2081U_18412, int64_t batch_18415)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_22648;
    
    mem_22648.references = NULL;
    
    struct memblock mem_param_tmp_22834;
    
    mem_param_tmp_22834.references = NULL;
    
    struct memblock mem_param_tmp_22833;
    
    mem_param_tmp_22833.references = NULL;
    
    struct memblock mem_param_tmp_22832;
    
    mem_param_tmp_22832.references = NULL;
    
    struct memblock mem_param_tmp_22844;
    
    mem_param_tmp_22844.references = NULL;
    
    struct memblock mem_param_tmp_22843;
    
    mem_param_tmp_22843.references = NULL;
    
    struct memblock mem_param_tmp_22842;
    
    mem_param_tmp_22842.references = NULL;
    
    struct memblock ext_mem_22608;
    
    ext_mem_22608.references = NULL;
    
    struct memblock ext_mem_22609;
    
    ext_mem_22609.references = NULL;
    
    struct memblock ext_mem_22610;
    
    ext_mem_22610.references = NULL;
    
    struct memblock mem_param_22601;
    
    mem_param_22601.references = NULL;
    
    struct memblock mem_param_22598;
    
    mem_param_22598.references = NULL;
    
    struct memblock mem_param_22595;
    
    mem_param_22595.references = NULL;
    
    struct memblock ext_mem_22617;
    
    ext_mem_22617.references = NULL;
    
    struct memblock ext_mem_22618;
    
    ext_mem_22618.references = NULL;
    
    struct memblock ext_mem_22619;
    
    ext_mem_22619.references = NULL;
    
    struct memblock mem_param_22592;
    
    mem_param_22592.references = NULL;
    
    struct memblock mem_param_22589;
    
    mem_param_22589.references = NULL;
    
    struct memblock mem_param_22586;
    
    mem_param_22586.references = NULL;
    
    struct memblock ext_mem_22644;
    
    ext_mem_22644.references = NULL;
    
    struct memblock ext_mem_22645;
    
    ext_mem_22645.references = NULL;
    
    struct memblock ext_mem_22646;
    
    ext_mem_22646.references = NULL;
    
    struct memblock ext_mem_22581;
    
    ext_mem_22581.references = NULL;
    
    struct memblock ext_mem_22582;
    
    ext_mem_22582.references = NULL;
    
    struct memblock ext_mem_22583;
    
    ext_mem_22583.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t bytes_22647 = (int64_t) 8 * dz2080U_18411;
    
    // probes/stdlib-fast-tests.fut:21:36-43
    
    int64_t hashmap_reserved_res_18926;
    
    if (futrts_empty_8082(ctx, &ext_mem_22583, &ext_mem_22582, &ext_mem_22581, &hashmap_reserved_res_18926, (int64_t) 2) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:21:15-27:27
    
    bool loop_cond_18927 = slt64((int64_t) 0, dz2080U_18411);
    
    // probes/stdlib-fast-tests.fut:21:15-27:27
    
    int64_t hashmap_reserved_res_18930;
    bool hashmap_reserved_res_18931;
    int64_t hashmap_reserved_res_18935;
    int64_t hashmap_reserved_res_18936;
    int64_t loop_dz2082U_18937;
    bool loop_while_18938;
    int64_t table_18942;
    int64_t next_18943;
    
    if (memblock_set(ctx, &mem_param_22586, &ext_mem_22583, "ext_mem_22583") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_22589, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_22592, &ext_mem_22581, "ext_mem_22581") != 0)
        return 1;
    loop_dz2082U_18937 = (int64_t) 2;
    loop_while_18938 = loop_cond_18927;
    table_18942 = hashmap_reserved_res_18926;
    next_18943 = (int64_t) 0;
    while (loop_while_18938) {
        // probes/stdlib-fast-tests.fut:22:43-49
        
        int64_t min_arg1_18944 = add64(batch_18415, next_18943);
        
        // probes/stdlib-fast-tests.fut:22:16-49
        
        int64_t min_res_18945 = smin64(dz2080U_18411, min_arg1_18944);
        
        // probes/stdlib-fast-tests.fut:24:24-26:28
        
        bool bounds_invalid_upwards_18972 = slt64(min_res_18945, next_18943);
        
        // probes/stdlib-fast-tests.fut:24:24-26:28
        
        bool valid_18974 = !bounds_invalid_upwards_18972;
        
        // probes/stdlib-fast-tests.fut:24:24-26:28
        
        bool range_valid_c_18975;
        
        if (!valid_18974) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) next_18943, "..<", (long long) min_res_18945, " is invalid.", "-> #0  probes/stdlib-fast-tests.fut:24:24-26:28\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // probes/stdlib-fast-tests.fut:24:66-77
        
        int64_t dzlz7bUZLzmZRz20Ustopz20Unextz7dUzg_18946 = sub64(min_res_18945, next_18943);
        
        // stdlib/hashmap.fut:43:3-76
        
        int64_t zt_lhs_18947 = add64(table_18942, dzlz7bUZLzmZRz20Ustopz20Unextz7dUzg_18946);
        
        // stdlib/hashmap.fut:43:3-76
        
        int64_t zg_lhs_18948 = mul64((int64_t) 10, zt_lhs_18947);
        
        // stdlib/hashmap.fut:43:3-76
        
        int64_t zg_rhs_18949 = mul64((int64_t) 7, loop_dz2082U_18937);
        
        // stdlib/hashmap.fut:43:3-76
        
        bool loop_cond_18950 = slt64(zg_rhs_18949, zg_lhs_18948);
        
        // stdlib/hashmap.fut:43:3-76
        
        int64_t reserve_res_18951;
        bool reserve_res_18952;
        int64_t reserve_res_18956;
        int64_t loop_dz2080U_18957;
        bool loop_while_18958;
        int64_t t_18962;
        
        if (memblock_set(ctx, &mem_param_22595, &mem_param_22586, "mem_param_22586") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22598, &mem_param_22589, "mem_param_22589") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22601, &mem_param_22592, "mem_param_22592") != 0)
            return 1;
        loop_dz2080U_18957 = loop_dz2082U_18937;
        loop_while_18958 = loop_cond_18950;
        t_18962 = table_18942;
        while (loop_while_18958) {
            // stdlib/hashmap.fut:43:70-76
            
            int64_t loopres_18963;
            int64_t loopres_18967;
            
            if (futrts_grow_11710(ctx, &ext_mem_22610, &ext_mem_22609, &ext_mem_22608, &loopres_18963, &loopres_18967, mem_param_22595, mem_param_22598, mem_param_22601, loop_dz2080U_18957, t_18962) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/hashmap.fut:43:24-35
            
            int64_t zt_lhs_18968 = add64(dzlz7bUZLzmZRz20Ustopz20Unextz7dUzg_18946, loopres_18967);
            
            // stdlib/hashmap.fut:43:36-39
            
            int64_t zg_lhs_18969 = mul64((int64_t) 10, zt_lhs_18968);
            
            // stdlib/hashmap.fut:43:52-54
            
            int64_t zg_rhs_18970 = mul64((int64_t) 7, loopres_18963);
            
            // stdlib/hashmap.fut:43:40-54
            
            bool loop_cond_18971 = slt64(zg_rhs_18970, zg_lhs_18969);
            
            if (memblock_set(ctx, &mem_param_tmp_22842, &ext_mem_22610, "ext_mem_22610") != 0)
                return 1;
            if (memblock_set(ctx, &mem_param_tmp_22843, &ext_mem_22609, "ext_mem_22609") != 0)
                return 1;
            if (memblock_set(ctx, &mem_param_tmp_22844, &ext_mem_22608, "ext_mem_22608") != 0)
                return 1;
            
            int64_t loop_dz2080U_tmp_22845 = loopres_18963;
            bool loop_while_tmp_22846 = loop_cond_18971;
            int64_t t_tmp_22850 = loopres_18967;
            
            if (memblock_set(ctx, &mem_param_22595, &mem_param_tmp_22842, "mem_param_tmp_22842") != 0)
                return 1;
            if (memblock_set(ctx, &mem_param_22598, &mem_param_tmp_22843, "mem_param_tmp_22843") != 0)
                return 1;
            if (memblock_set(ctx, &mem_param_22601, &mem_param_tmp_22844, "mem_param_tmp_22844") != 0)
                return 1;
            loop_dz2080U_18957 = loop_dz2080U_tmp_22845;
            loop_while_18958 = loop_while_tmp_22846;
            t_18962 = t_tmp_22850;
        }
        if (memblock_set(ctx, &ext_mem_22619, &mem_param_22595, "mem_param_22595") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_22618, &mem_param_22598, "mem_param_22598") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_22617, &mem_param_22601, "mem_param_22601") != 0)
            return 1;
        reserve_res_18951 = loop_dz2080U_18957;
        reserve_res_18952 = loop_while_18958;
        reserve_res_18956 = t_18962;
        // stdlib/hashmap.fut:11:26-28
        
        int64_t mask_21861 = sub64(reserve_res_18951, (int64_t) 1);
        
        // probes/stdlib-fast-tests.fut:24:24-26:28
        
        int64_t loopres_18979;
        int64_t sizze_18984 = reserve_res_18956;
        
        for (int64_t i_18980 = 0; i_18980 < dzlz7bUZLzmZRz20Ustopz20Unextz7dUzg_18946; i_18980++) {
            int64_t index_primexp_18985 = add64(next_18943, i_18980);
            
            // probes/stdlib-fast-tests.fut:25:57-66
            
            bool x_18986 = sle64((int64_t) 0, index_primexp_18985);
            
            // probes/stdlib-fast-tests.fut:25:57-66
            
            bool y_18987 = slt64(index_primexp_18985, dz2081U_18412);
            
            // probes/stdlib-fast-tests.fut:25:57-66
            
            bool bounds_check_18988 = x_18986 && y_18987;
            
            // probes/stdlib-fast-tests.fut:25:57-66
            
            bool index_certs_18989;
            
            if (!bounds_check_18988) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) index_primexp_18985, "] out of bounds for array of shape [", (long long) dz2081U_18412, "].", "-> #0  probes/stdlib-fast-tests.fut:25:57-66\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // probes/stdlib-fast-tests.fut:25:49-56
            
            bool y_18992 = slt64(index_primexp_18985, dz2080U_18411);
            
            // probes/stdlib-fast-tests.fut:25:49-56
            
            bool bounds_check_18993 = x_18986 && y_18992;
            
            // probes/stdlib-fast-tests.fut:25:49-56
            
            bool index_certs_18994;
            
            if (!bounds_check_18993) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) index_primexp_18985, "] out of bounds for array of shape [", (long long) dz2080U_18411, "].", "-> #0  probes/stdlib-fast-tests.fut:25:49-56\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // probes/stdlib-fast-tests.fut:25:57-66
            
            int64_t increment_slots_arg2_18990 = ((int64_t *) deltas_mem_22580.mem)[index_primexp_18985];
            
            // probes/stdlib-fast-tests.fut:25:49-56
            
            int64_t increment_slots_arg1_18995 = ((int64_t *) keys_mem_22579.mem)[index_primexp_18985];
            
            // stdlib/hashmap.fut:7:19-24
            
            int64_t zc_rhs_21862 = lshr64(increment_slots_arg1_18995, (int64_t) 30);
            
            // stdlib/hashmap.fut:7:14-24
            
            int64_t zt_lhs_21863 = increment_slots_arg1_18995 ^ zc_rhs_21862;
            
            // stdlib/hashmap.fut:7:26-48
            
            int64_t x_21864 = mul64((int64_t) -4658895280553007687, zt_lhs_21863);
            
            // stdlib/hashmap.fut:8:19-24
            
            int64_t zc_rhs_21865 = lshr64(x_21864, (int64_t) 27);
            
            // stdlib/hashmap.fut:8:14-24
            
            int64_t zt_lhs_21866 = x_21864 ^ zc_rhs_21865;
            
            // stdlib/hashmap.fut:8:26-48
            
            int64_t x_21867 = mul64((int64_t) -7723592293110705685, zt_lhs_21866);
            
            // stdlib/hashmap.fut:9:13-18
            
            int64_t zc_rhs_21868 = lshr64(x_21867, (int64_t) 31);
            
            // stdlib/hashmap.fut:9:8-18
            
            int64_t hash_res_21869 = x_21867 ^ zc_rhs_21868;
            
            // stdlib/hashmap.fut:12:43-57
            
            int64_t u64_arg0_21870 = mask_21861 & hash_res_21869;
            
            // stdlib/hashmap.fut:12:16-13:57
            
            bool loop_cond_21871 = ((bool *) ext_mem_22617.mem)[u64_arg0_21870];
            
            // stdlib/hashmap.fut:12:16-13:57
            
            bool loop_cond_21872;
            
            if (loop_cond_21871) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_21918 = ((int64_t *) ext_mem_22619.mem)[u64_arg0_21870];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21919 = loop_cond_t_res_21918 == increment_slots_arg1_18995;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21920 = !loop_cond_t_res_21919;
                
                loop_cond_21872 = loop_cond_t_res_21920;
            } else {
                loop_cond_21872 = 0;
            }
            // stdlib/hashmap.fut:12:16-13:57
            
            bool slot_res_21876;
            int64_t slot_res_21877;
            bool loop_while_21878;
            int64_t i_21879;
            
            loop_while_21878 = loop_cond_21872;
            i_21879 = u64_arg0_21870;
            while (loop_while_21878) {
                // stdlib/hashmap.fut:13:49-51
                
                int64_t za_lhs_21880 = add64((int64_t) 1, i_21879);
                
                // stdlib/hashmap.fut:13:52-57
                
                int64_t loopres_21881 = mask_21861 & za_lhs_21880;
                
                // stdlib/hashmap.fut:13:12-20
                
                bool loop_cond_21882 = ((bool *) ext_mem_22617.mem)[loopres_21881];
                
                // stdlib/hashmap.fut:13:12-43
                
                bool loop_cond_21883;
                
                if (loop_cond_21882) {
                    // stdlib/hashmap.fut:13:26-34
                    
                    int64_t loop_cond_t_res_21921 = ((int64_t *) ext_mem_22619.mem)[loopres_21881];
                    
                    // stdlib/hashmap.fut:13:37-43
                    
                    bool loop_cond_t_res_21922 = loop_cond_t_res_21921 == increment_slots_arg1_18995;
                    
                    // stdlib/hashmap.fut:13:37-43
                    
                    bool loop_cond_t_res_21923 = !loop_cond_t_res_21922;
                    
                    loop_cond_21883 = loop_cond_t_res_21923;
                } else {
                    loop_cond_21883 = 0;
                }
                
                bool loop_while_tmp_22855 = loop_cond_21883;
                int64_t i_tmp_22856 = loopres_21881;
                
                loop_while_21878 = loop_while_tmp_22855;
                i_21879 = i_tmp_22856;
            }
            slot_res_21876 = loop_while_21878;
            slot_res_21877 = i_21879;
            // stdlib/hashmap.fut:48:37-45
            
            int64_t increment_slots_res_18997 = ((int64_t *) ext_mem_22618.mem)[slot_res_21877];
            
            // stdlib/hashmap.fut:48:37-45
            
            bool increment_slots_res_18998 = ((bool *) ext_mem_22617.mem)[slot_res_21877];
            
            // stdlib/hashmap.fut:49:49-74
            
            int64_t zp_lhs_18999;
            
            if (increment_slots_res_18998) {
                zp_lhs_18999 = increment_slots_res_18997;
            } else {
                zp_lhs_18999 = (int64_t) 0;
            }
            // stdlib/hashmap.fut:49:75-81
            
            int64_t slots_19000 = add64(increment_slots_arg2_18990, zp_lhs_18999);
            
            // stdlib/hashmap.fut:49:26-87
            ((int64_t *) ext_mem_22619.mem)[slot_res_21877] = increment_slots_arg1_18995;
            // stdlib/hashmap.fut:49:26-87
            ((int64_t *) ext_mem_22618.mem)[slot_res_21877] = slots_19000;
            // stdlib/hashmap.fut:49:26-87
            ((bool *) ext_mem_22617.mem)[slot_res_21877] = 1;
            // probes/stdlib-fast-tests.fut:25:27-66
            
            bool cond_neg_19004 = !increment_slots_res_18998;
            
            // probes/stdlib-fast-tests.fut:25:27-66
            
            int64_t tmp_19005 = btoi_bool_i64(cond_neg_19004);
            
            // probes/stdlib-fast-tests.fut:26:21-27
            
            int64_t tmp_19006 = add64(sizze_18984, tmp_19005);
            int64_t sizze_tmp_22854 = tmp_19006;
            
            sizze_18984 = sizze_tmp_22854;
        }
        loopres_18979 = sizze_18984;
        // probes/stdlib-fast-tests.fut:21:61-74
        
        bool loop_cond_19007 = slt64(min_res_18945, dz2080U_18411);
        
        if (memblock_set(ctx, &mem_param_tmp_22832, &ext_mem_22619, "ext_mem_22619") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22833, &ext_mem_22618, "ext_mem_22618") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22834, &ext_mem_22617, "ext_mem_22617") != 0)
            return 1;
        
        int64_t loop_dz2082U_tmp_22835 = reserve_res_18951;
        bool loop_while_tmp_22836 = loop_cond_19007;
        int64_t table_tmp_22840 = loopres_18979;
        int64_t next_tmp_22841 = min_res_18945;
        
        if (memblock_set(ctx, &mem_param_22586, &mem_param_tmp_22832, "mem_param_tmp_22832") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22589, &mem_param_tmp_22833, "mem_param_tmp_22833") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22592, &mem_param_tmp_22834, "mem_param_tmp_22834") != 0)
            return 1;
        loop_dz2082U_18937 = loop_dz2082U_tmp_22835;
        loop_while_18938 = loop_while_tmp_22836;
        table_18942 = table_tmp_22840;
        next_18943 = next_tmp_22841;
    }
    if (memblock_set(ctx, &ext_mem_22646, &mem_param_22586, "mem_param_22586") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_22645, &mem_param_22589, "mem_param_22589") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_22644, &mem_param_22592, "mem_param_22592") != 0)
        return 1;
    hashmap_reserved_res_18930 = loop_dz2082U_18937;
    hashmap_reserved_res_18931 = loop_while_18938;
    hashmap_reserved_res_18935 = table_18942;
    hashmap_reserved_res_18936 = next_18943;
    if (memblock_unref(ctx, &ext_mem_22581, "ext_mem_22581") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22583, "ext_mem_22583") != 0)
        return 1;
    // stdlib/hashmap.fut:11:26-28
    
    int64_t mask_21892 = sub64(hashmap_reserved_res_18930, (int64_t) 1);
    
    if (memblock_alloc(ctx, &mem_22648, bytes_22647, "mem_22648")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:28:6-28
    for (int64_t i_22565 = 0; i_22565 < dz2080U_18411; i_22565++) {
        int64_t eta_p_19009 = ((int64_t *) keys_mem_22579.mem)[i_22565];
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_21893 = lshr64(eta_p_19009, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_21894 = eta_p_19009 ^ zc_rhs_21893;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_21895 = mul64((int64_t) -4658895280553007687, zt_lhs_21894);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_21896 = lshr64(x_21895, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_21897 = x_21895 ^ zc_rhs_21896;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_21898 = mul64((int64_t) -7723592293110705685, zt_lhs_21897);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_21899 = lshr64(x_21898, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_21900 = x_21898 ^ zc_rhs_21899;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_21901 = mask_21892 & hash_res_21900;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21902 = ((bool *) ext_mem_22644.mem)[u64_arg0_21901];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21903;
        
        if (loop_cond_21902) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_21924 = ((int64_t *) ext_mem_22646.mem)[u64_arg0_21901];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21925 = loop_cond_t_res_21924 == eta_p_19009;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21926 = !loop_cond_t_res_21925;
            
            loop_cond_21903 = loop_cond_t_res_21926;
        } else {
            loop_cond_21903 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_21907;
        int64_t slot_res_21908;
        bool loop_while_21909;
        int64_t i_21910;
        
        loop_while_21909 = loop_cond_21903;
        i_21910 = u64_arg0_21901;
        while (loop_while_21909) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_21911 = add64((int64_t) 1, i_21910);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_21912 = mask_21892 & za_lhs_21911;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_21913 = ((bool *) ext_mem_22644.mem)[loopres_21912];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_21914;
            
            if (loop_cond_21913) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_21927 = ((int64_t *) ext_mem_22646.mem)[loopres_21912];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21928 = loop_cond_t_res_21927 == eta_p_19009;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21929 = !loop_cond_t_res_21928;
                
                loop_cond_21914 = loop_cond_t_res_21929;
            } else {
                loop_cond_21914 = 0;
            }
            
            bool loop_while_tmp_22858 = loop_cond_21914;
            int64_t i_tmp_22859 = loopres_21912;
            
            loop_while_21909 = loop_while_tmp_22858;
            i_21910 = i_tmp_22859;
        }
        slot_res_21907 = loop_while_21909;
        slot_res_21908 = i_21910;
        // stdlib/hashmap.fut:17:9-17
        
        bool x_19011 = sle64((int64_t) 0, slot_res_21908);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool y_19012 = slt64(slot_res_21908, hashmap_reserved_res_18930);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool bounds_check_19013 = x_19011 && y_19012;
        
        // stdlib/hashmap.fut:17:9-17
        
        bool index_certs_19014;
        
        if (!bounds_check_19013) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_21908, "] out of bounds for array of shape [", (long long) hashmap_reserved_res_18930, "].", "-> #0  stdlib/hashmap.fut:17:9-17\n   #1  probes/stdlib-fast-tests.fut:28:11-14\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:17:9-17
        
        bool get_res_19015 = ((bool *) ext_mem_22644.mem)[slot_res_21908];
        
        // stdlib/hashmap.fut:17:6-42
        
        int64_t get_res_19016;
        
        if (get_res_19015) {
            // stdlib/hashmap.fut:17:25-33
            
            int64_t get_res_t_res_21930 = ((int64_t *) ext_mem_22645.mem)[slot_res_21908];
            
            get_res_19016 = get_res_t_res_21930;
        } else {
            get_res_19016 = (int64_t) 0;
        }
        ((int64_t *) mem_22648.mem)[i_22565] = get_res_19016;
    }
    if (memblock_unref(ctx, &ext_mem_22644, "ext_mem_22644") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22645, "ext_mem_22645") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22646, "ext_mem_22646") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &mem_22648, "mem_22648") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22959, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_22648, "mem_22648") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22834, "mem_param_tmp_22834") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22833, "mem_param_tmp_22833") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22832, "mem_param_tmp_22832") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22844, "mem_param_tmp_22844") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22843, "mem_param_tmp_22843") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22842, "mem_param_tmp_22842") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22608, "ext_mem_22608") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22609, "ext_mem_22609") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22610, "ext_mem_22610") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22601, "mem_param_22601") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22598, "mem_param_22598") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22595, "mem_param_22595") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22617, "ext_mem_22617") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22618, "ext_mem_22618") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22619, "ext_mem_22619") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22592, "mem_param_22592") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22589, "mem_param_22589") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22586, "mem_param_22586") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22644, "ext_mem_22644") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22645, "ext_mem_22645") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22646, "ext_mem_22646") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22581, "ext_mem_22581") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22583, "ext_mem_22583") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_hashmap_test(struct futhark_context *ctx, struct memblock *mem_out_p_22960, struct memblock keys_mem_22579, int64_t dz2080U_17971)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_22629;
    
    mem_22629.references = NULL;
    
    struct memblock mem_param_tmp_22834;
    
    mem_param_tmp_22834.references = NULL;
    
    struct memblock mem_param_tmp_22833;
    
    mem_param_tmp_22833.references = NULL;
    
    struct memblock mem_param_tmp_22832;
    
    mem_param_tmp_22832.references = NULL;
    
    struct memblock ext_mem_22616;
    
    ext_mem_22616.references = NULL;
    
    struct memblock ext_mem_22617;
    
    ext_mem_22617.references = NULL;
    
    struct memblock ext_mem_22618;
    
    ext_mem_22618.references = NULL;
    
    struct memblock ext_mem_22598;
    
    ext_mem_22598.references = NULL;
    
    struct memblock ext_mem_22599;
    
    ext_mem_22599.references = NULL;
    
    struct memblock ext_mem_22600;
    
    ext_mem_22600.references = NULL;
    
    struct memblock ext_mem_22603;
    
    ext_mem_22603.references = NULL;
    
    struct memblock ext_mem_22606;
    
    ext_mem_22606.references = NULL;
    
    struct memblock ext_mem_22609;
    
    ext_mem_22609.references = NULL;
    
    struct memblock mem_param_22591;
    
    mem_param_22591.references = NULL;
    
    struct memblock mem_param_22588;
    
    mem_param_22588.references = NULL;
    
    struct memblock mem_param_22585;
    
    mem_param_22585.references = NULL;
    
    struct memblock ext_mem_22625;
    
    ext_mem_22625.references = NULL;
    
    struct memblock ext_mem_22626;
    
    ext_mem_22626.references = NULL;
    
    struct memblock ext_mem_22627;
    
    ext_mem_22627.references = NULL;
    
    struct memblock ext_mem_22580;
    
    ext_mem_22580.references = NULL;
    
    struct memblock ext_mem_22581;
    
    ext_mem_22581.references = NULL;
    
    struct memblock ext_mem_22582;
    
    ext_mem_22582.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t bytes_22628 = (int64_t) 8 * dz2080U_17971;
    
    // probes/stdlib-fast-tests.fut:13:28-35
    
    int64_t hashmap_test_res_18923;
    
    if (futrts_empty_8082(ctx, &ext_mem_22582, &ext_mem_22581, &ext_mem_22580, &hashmap_test_res_18923, (int64_t) 2) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:13:15-15:31
    
    int64_t hashmap_test_res_18924;
    int64_t hashmap_test_res_18928;
    int64_t loop_dz2081U_18930;
    int64_t table_18934;
    
    if (memblock_set(ctx, &mem_param_22585, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_22588, &ext_mem_22581, "ext_mem_22581") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_22591, &ext_mem_22580, "ext_mem_22580") != 0)
        return 1;
    loop_dz2081U_18930 = (int64_t) 2;
    table_18934 = hashmap_test_res_18923;
    for (int64_t i_18929 = 0; i_18929 < dz2080U_17971; i_18929++) {
        int64_t key_18935 = ((int64_t *) keys_mem_22579.mem)[i_18929];
        
        // stdlib/hashmap.fut:11:26-28
        
        int64_t mask_21861 = sub64(loop_dz2081U_18930, (int64_t) 1);
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_21862 = lshr64(key_18935, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_21863 = key_18935 ^ zc_rhs_21862;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_21864 = mul64((int64_t) -4658895280553007687, zt_lhs_21863);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_21865 = lshr64(x_21864, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_21866 = x_21864 ^ zc_rhs_21865;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_21867 = mul64((int64_t) -7723592293110705685, zt_lhs_21866);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_21868 = lshr64(x_21867, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_21869 = x_21867 ^ zc_rhs_21868;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_21870 = mask_21861 & hash_res_21869;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21871 = ((bool *) mem_param_22591.mem)[u64_arg0_21870];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21872;
        
        if (loop_cond_21871) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_21918 = ((int64_t *) mem_param_22585.mem)[u64_arg0_21870];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21919 = loop_cond_t_res_21918 == key_18935;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21920 = !loop_cond_t_res_21919;
            
            loop_cond_21872 = loop_cond_t_res_21920;
        } else {
            loop_cond_21872 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_21876;
        int64_t slot_res_21877;
        bool loop_while_21878;
        int64_t i_21879;
        
        loop_while_21878 = loop_cond_21872;
        i_21879 = u64_arg0_21870;
        while (loop_while_21878) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_21880 = add64((int64_t) 1, i_21879);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_21881 = mask_21861 & za_lhs_21880;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_21882 = ((bool *) mem_param_22591.mem)[loopres_21881];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_21883;
            
            if (loop_cond_21882) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_21921 = ((int64_t *) mem_param_22585.mem)[loopres_21881];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21922 = loop_cond_t_res_21921 == key_18935;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21923 = !loop_cond_t_res_21922;
                
                loop_cond_21883 = loop_cond_t_res_21923;
            } else {
                loop_cond_21883 = 0;
            }
            
            bool loop_while_tmp_22840 = loop_cond_21883;
            int64_t i_tmp_22841 = loopres_21881;
            
            loop_while_21878 = loop_while_tmp_22840;
            i_21879 = i_tmp_22841;
        }
        slot_res_21876 = loop_while_21878;
        slot_res_21877 = i_21879;
        // stdlib/hashmap.fut:17:9-17
        
        bool x_18937 = sle64((int64_t) 0, slot_res_21877);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool y_18938 = slt64(slot_res_21877, loop_dz2081U_18930);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool bounds_check_18939 = x_18937 && y_18938;
        
        // stdlib/hashmap.fut:17:9-17
        
        bool index_certs_18940;
        
        if (!bounds_check_18939) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_21877, "] out of bounds for array of shape [", (long long) loop_dz2081U_18930, "].", "-> #0  stdlib/hashmap.fut:17:9-17\n   #1  probes/stdlib-fast-tests.fut:14:17-30\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:17:9-17
        
        bool get_res_18941 = ((bool *) mem_param_22591.mem)[slot_res_21877];
        
        // stdlib/hashmap.fut:17:6-42
        
        int64_t get_res_18942;
        
        if (get_res_18941) {
            // stdlib/hashmap.fut:17:25-33
            
            int64_t get_res_t_res_21924 = ((int64_t *) mem_param_22588.mem)[slot_res_21877];
            
            get_res_18942 = get_res_t_res_21924;
        } else {
            get_res_18942 = (int64_t) 0;
        }
        // probes/stdlib-fast-tests.fut:15:28-30
        
        int64_t put_arg2_18948 = add64((int64_t) 1, get_res_18942);
        
        // stdlib/hashmap.fut:29:18-20
        
        int64_t zt_lhs_18949 = add64((int64_t) 1, table_18934);
        
        // stdlib/hashmap.fut:29:21-24
        
        int64_t zg_lhs_18950 = mul64((int64_t) 10, zt_lhs_18949);
        
        // stdlib/hashmap.fut:29:37-39
        
        int64_t zg_rhs_18951 = mul64((int64_t) 7, loop_dz2081U_18930);
        
        // stdlib/hashmap.fut:29:11-70
        
        bool cond_18952 = slt64(zg_rhs_18951, zg_lhs_18950);
        int64_t put_res_18953;
        int64_t put_res_18957;
        
        if (cond_18952) {
            // stdlib/hashmap.fut:28:10-29:61
            
            int64_t put_res_t_res_21925;
            int64_t put_res_t_res_21929;
            
            if (futrts_grow_11710(ctx, &ext_mem_22600, &ext_mem_22599, &ext_mem_22598, &put_res_t_res_21925, &put_res_t_res_21929, mem_param_22585, mem_param_22588, mem_param_22591, loop_dz2081U_18930, table_18934) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_22609, &ext_mem_22600, "ext_mem_22600") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22606, &ext_mem_22599, "ext_mem_22599") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22603, &ext_mem_22598, "ext_mem_22598") != 0)
                return 1;
            put_res_18953 = put_res_t_res_21925;
            put_res_18957 = put_res_t_res_21929;
        } else {
            if (memblock_set(ctx, &ext_mem_22609, &mem_param_22585, "mem_param_22585") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22606, &mem_param_22588, "mem_param_22588") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22603, &mem_param_22591, "mem_param_22591") != 0)
                return 1;
            put_res_18953 = loop_dz2081U_18930;
            put_res_18957 = table_18934;
        }
        // stdlib/hashmap.fut:29:7-30:25
        
        int64_t put_res_18966;
        
        if (futrts_put_raw_11712(ctx, &ext_mem_22618, &ext_mem_22617, &ext_mem_22616, &put_res_18966, ext_mem_22609, ext_mem_22606, ext_mem_22603, put_res_18953, put_res_18957, key_18935, put_arg2_18948) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_22603, "ext_mem_22603") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22606, "ext_mem_22606") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22609, "ext_mem_22609") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22832, &ext_mem_22618, "ext_mem_22618") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22833, &ext_mem_22617, "ext_mem_22617") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22834, &ext_mem_22616, "ext_mem_22616") != 0)
            return 1;
        
        int64_t loop_dz2081U_tmp_22835 = put_res_18953;
        int64_t table_tmp_22839 = put_res_18966;
        
        if (memblock_set(ctx, &mem_param_22585, &mem_param_tmp_22832, "mem_param_tmp_22832") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22588, &mem_param_tmp_22833, "mem_param_tmp_22833") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22591, &mem_param_tmp_22834, "mem_param_tmp_22834") != 0)
            return 1;
        loop_dz2081U_18930 = loop_dz2081U_tmp_22835;
        table_18934 = table_tmp_22839;
    }
    if (memblock_set(ctx, &ext_mem_22627, &mem_param_22585, "mem_param_22585") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_22626, &mem_param_22588, "mem_param_22588") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_22625, &mem_param_22591, "mem_param_22591") != 0)
        return 1;
    hashmap_test_res_18924 = loop_dz2081U_18930;
    hashmap_test_res_18928 = table_18934;
    if (memblock_unref(ctx, &ext_mem_22580, "ext_mem_22580") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22581, "ext_mem_22581") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    // stdlib/hashmap.fut:11:26-28
    
    int64_t mask_21892 = sub64(hashmap_test_res_18924, (int64_t) 1);
    
    if (memblock_alloc(ctx, &mem_22629, bytes_22628, "mem_22629")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:16:6-26
    for (int64_t i_22565 = 0; i_22565 < dz2080U_17971; i_22565++) {
        int64_t eta_p_18968 = ((int64_t *) keys_mem_22579.mem)[i_22565];
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_21893 = lshr64(eta_p_18968, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_21894 = eta_p_18968 ^ zc_rhs_21893;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_21895 = mul64((int64_t) -4658895280553007687, zt_lhs_21894);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_21896 = lshr64(x_21895, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_21897 = x_21895 ^ zc_rhs_21896;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_21898 = mul64((int64_t) -7723592293110705685, zt_lhs_21897);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_21899 = lshr64(x_21898, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_21900 = x_21898 ^ zc_rhs_21899;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_21901 = mask_21892 & hash_res_21900;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21902 = ((bool *) ext_mem_22625.mem)[u64_arg0_21901];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_21903;
        
        if (loop_cond_21902) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_21930 = ((int64_t *) ext_mem_22627.mem)[u64_arg0_21901];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21931 = loop_cond_t_res_21930 == eta_p_18968;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21932 = !loop_cond_t_res_21931;
            
            loop_cond_21903 = loop_cond_t_res_21932;
        } else {
            loop_cond_21903 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_21907;
        int64_t slot_res_21908;
        bool loop_while_21909;
        int64_t i_21910;
        
        loop_while_21909 = loop_cond_21903;
        i_21910 = u64_arg0_21901;
        while (loop_while_21909) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_21911 = add64((int64_t) 1, i_21910);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_21912 = mask_21892 & za_lhs_21911;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_21913 = ((bool *) ext_mem_22625.mem)[loopres_21912];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_21914;
            
            if (loop_cond_21913) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_21933 = ((int64_t *) ext_mem_22627.mem)[loopres_21912];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21934 = loop_cond_t_res_21933 == eta_p_18968;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_21935 = !loop_cond_t_res_21934;
                
                loop_cond_21914 = loop_cond_t_res_21935;
            } else {
                loop_cond_21914 = 0;
            }
            
            bool loop_while_tmp_22843 = loop_cond_21914;
            int64_t i_tmp_22844 = loopres_21912;
            
            loop_while_21909 = loop_while_tmp_22843;
            i_21910 = i_tmp_22844;
        }
        slot_res_21907 = loop_while_21909;
        slot_res_21908 = i_21910;
        // stdlib/hashmap.fut:17:9-17
        
        bool x_18970 = sle64((int64_t) 0, slot_res_21908);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool y_18971 = slt64(slot_res_21908, hashmap_test_res_18924);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool bounds_check_18972 = x_18970 && y_18971;
        
        // stdlib/hashmap.fut:17:9-17
        
        bool index_certs_18973;
        
        if (!bounds_check_18972) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_21908, "] out of bounds for array of shape [", (long long) hashmap_test_res_18924, "].", "-> #0  stdlib/hashmap.fut:17:9-17\n   #1  probes/stdlib-fast-tests.fut:16:11-14\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:17:9-17
        
        bool get_res_18974 = ((bool *) ext_mem_22625.mem)[slot_res_21908];
        
        // stdlib/hashmap.fut:17:6-42
        
        int64_t get_res_18975;
        
        if (get_res_18974) {
            // stdlib/hashmap.fut:17:25-33
            
            int64_t get_res_t_res_21936 = ((int64_t *) ext_mem_22626.mem)[slot_res_21908];
            
            get_res_18975 = get_res_t_res_21936;
        } else {
            get_res_18975 = (int64_t) 0;
        }
        ((int64_t *) mem_22629.mem)[i_22565] = get_res_18975;
    }
    if (memblock_unref(ctx, &ext_mem_22625, "ext_mem_22625") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22626, "ext_mem_22626") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22627, "ext_mem_22627") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &mem_22629, "mem_22629") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22960, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_22629, "mem_22629") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22834, "mem_param_tmp_22834") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22833, "mem_param_tmp_22833") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22832, "mem_param_tmp_22832") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22616, "ext_mem_22616") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22617, "ext_mem_22617") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22618, "ext_mem_22618") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22598, "ext_mem_22598") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22599, "ext_mem_22599") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22600, "ext_mem_22600") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22603, "ext_mem_22603") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22606, "ext_mem_22606") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22609, "ext_mem_22609") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22591, "mem_param_22591") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22588, "mem_param_22588") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22585, "mem_param_22585") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22625, "ext_mem_22625") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22626, "ext_mem_22626") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22627, "ext_mem_22627") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22580, "ext_mem_22580") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22581, "ext_mem_22581") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_regex_test(struct futhark_context *ctx, struct memblock *mem_out_p_22961, int64_t *out_prim_out_22962, int64_t *out_prim_out_22963, struct memblock pattern_mem_22579, struct memblock text_mem_22580, struct memblock replacement_mem_22581, int64_t dz2080U_17516, int64_t dz2081U_17517, int64_t dz2082U_17518)
{
    (void) ctx;
    
    int err = 0;
    int64_t mem_22650_cached_sizze_22964 = 0;
    unsigned char *mem_22650 = NULL;
    struct memblock mem_22665;
    
    mem_22665.references = NULL;
    
    struct memblock ext_mem_22605;
    
    ext_mem_22605.references = NULL;
    
    struct memblock mem_22604;
    
    mem_22604.references = NULL;
    
    struct memblock mem_22606;
    
    mem_22606.references = NULL;
    
    struct memblock ext_mem_22607;
    
    ext_mem_22607.references = NULL;
    
    struct memblock mem_22624;
    
    mem_22624.references = NULL;
    
    struct memblock ext_mem_22633;
    
    ext_mem_22633.references = NULL;
    
    struct memblock mem_param_tmp_22848;
    
    mem_param_tmp_22848.references = NULL;
    
    struct memblock mem_param_tmp_22852;
    
    mem_param_tmp_22852.references = NULL;
    
    struct memblock ext_mem_22617;
    
    ext_mem_22617.references = NULL;
    
    struct memblock mem_param_22614;
    
    mem_param_22614.references = NULL;
    
    struct memblock ext_mem_22620;
    
    ext_mem_22620.references = NULL;
    
    struct memblock mem_param_22611;
    
    mem_param_22611.references = NULL;
    
    struct memblock ext_mem_22623;
    
    ext_mem_22623.references = NULL;
    
    struct memblock mem_22608;
    
    mem_22608.references = NULL;
    
    struct memblock ext_mem_22636;
    
    ext_mem_22636.references = NULL;
    
    struct memblock mem_22590;
    
    mem_22590.references = NULL;
    
    struct memblock mem_22588;
    
    mem_22588.references = NULL;
    
    struct memblock mem_22586;
    
    mem_22586.references = NULL;
    
    struct memblock mem_22584;
    
    mem_22584.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22832;
    int64_t prim_out_22833;
    
    // stdlib/regex.fut:26:28-30
    
    int64_t sizze_21480 = add64((int64_t) 1, dz2080U_17516);
    int64_t binop_x_22582 = (int64_t) 8 * sizze_21480;
    int64_t bytes_22583 = (int64_t) 32 * sizze_21480;
    
    if (memblock_alloc(ctx, &mem_22584, bytes_22583, "mem_22584")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-fast-tests.fut:10:12-27
    for (int64_t nest_i_22834 = 0; nest_i_22834 < sizze_21480; nest_i_22834++) {
        for (int64_t nest_i_22835 = 0; nest_i_22835 < (int64_t) 4; nest_i_22835++) {
            ((int64_t *) mem_22584.mem)[nest_i_22834 * (int64_t) 4 + nest_i_22835] = (int64_t) 0;
        }
    }
    if (memblock_alloc(ctx, &mem_22586, binop_x_22582, "mem_22586")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:47-66
    for (int64_t nest_i_22836 = 0; nest_i_22836 < sizze_21480; nest_i_22836++) {
        ((int64_t *) mem_22586.mem)[nest_i_22836] = (int64_t) 1;
    }
    if (memblock_alloc(ctx, &mem_22588, binop_x_22582, "mem_22588")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:67-86
    for (int64_t nest_i_22837 = 0; nest_i_22837 < sizze_21480; nest_i_22837++) {
        ((int64_t *) mem_22588.mem)[nest_i_22837] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_22590, binop_x_22582, "mem_22590")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:87-106
    for (int64_t nest_i_22838 = 0; nest_i_22838 < sizze_21480; nest_i_22838++) {
        ((int64_t *) mem_22590.mem)[nest_i_22838] = (int64_t) 0;
    }
    // stdlib/regex.fut:28:5-49:95
    
    bool loop_cond_21485 = slt64((int64_t) 0, dz2080U_17516);
    
    // stdlib/regex.fut:28:5-49:95
    
    bool compile_res_21486;
    int64_t compile_res_21487;
    int64_t compile_res_21492;
    int64_t compile_res_21493;
    bool loop_while_21494;
    int64_t p_21495;
    int64_t used_21500;
    int64_t branch_21501;
    
    loop_while_21494 = loop_cond_21485;
    p_21495 = (int64_t) 0;
    used_21500 = (int64_t) 0;
    branch_21501 = (int64_t) 0;
    while (loop_while_21494) {
        // stdlib/regex.fut:31:12-22
        
        bool x_21502 = sle64((int64_t) 0, p_21495);
        
        // stdlib/regex.fut:31:12-22
        
        bool y_21503 = slt64(p_21495, dz2080U_17516);
        
        // stdlib/regex.fut:31:12-22
        
        bool bounds_check_21504 = x_21502 && y_21503;
        
        // stdlib/regex.fut:31:12-22
        
        bool index_certs_21505;
        
        if (!bounds_check_21504) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) p_21495, "] out of bounds for array of shape [", (long long) dz2080U_17516, "].", "-> #0  stdlib/regex.fut:31:12-22\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:31:12-22
        
        int8_t zeze_lhs_21506 = ((int8_t *) pattern_mem_22579.mem)[p_21495];
        
        // stdlib/regex.fut:31:9-49:95
        
        bool cond_21507 = zeze_lhs_21506 == (int8_t) 124;
        
        // stdlib/regex.fut:36:29-42:57
        
        bool cond_21527 = zeze_lhs_21506 == (int8_t) 91;
        
        // stdlib/regex.fut:31:9-49:95
        
        int64_t loopres_21508;
        int64_t loopres_21513;
        int64_t loopres_21514;
        
        if (cond_21507) {
            // stdlib/regex.fut:32:15-34
            
            bool x_22129 = sle64((int64_t) 0, branch_21501);
            
            // stdlib/regex.fut:32:15-34
            
            bool y_22130 = slt64(branch_21501, sizze_21480);
            
            // stdlib/regex.fut:32:15-34
            
            bool bounds_check_22131 = x_22129 && y_22130;
            
            // stdlib/regex.fut:32:15-34
            
            bool index_certs_22132;
            
            if (!bounds_check_22131) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) branch_21501, "] out of bounds for array of shape [", (long long) sizze_21480, "].", "-> #0  stdlib/regex.fut:32:15-34\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:33:28-30
            
            int64_t tmp_22134 = add64((int64_t) 1, branch_21501);
            
            // stdlib/regex.fut:33:15-38
            
            bool x_22135 = sle64((int64_t) 0, tmp_22134);
            
            // stdlib/regex.fut:33:15-38
            
            bool y_22136 = slt64(tmp_22134, sizze_21480);
            
            // stdlib/regex.fut:33:15-38
            
            bool bounds_check_22137 = x_22135 && y_22136;
            
            // stdlib/regex.fut:33:15-38
            
            bool index_certs_22138;
            
            if (!bounds_check_22137) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_22134, "] out of bounds for array of shape [", (long long) sizze_21480, "].", "-> #0  stdlib/regex.fut:33:15-38\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:32:15-34
            ((int64_t *) mem_22590.mem)[branch_21501] = used_21500;
            // stdlib/regex.fut:33:15-38
            ((int64_t *) mem_22588.mem)[tmp_22134] = used_21500;
            // stdlib/regex.fut:34:16-18
            
            int64_t tmp_22140 = add64((int64_t) 1, p_21495);
            
            loopres_21508 = tmp_22140;
            loopres_21513 = used_21500;
            loopres_21514 = tmp_22134;
        } else {
            // stdlib/regex.fut:15:23-25
            
            int64_t zl_lhs_22176 = add64((int64_t) 1, p_21495);
            
            // stdlib/regex.fut:15:18-68
            
            bool cond_22177 = slt64(zl_lhs_22176, dz2080U_17516);
            
            // stdlib/regex.fut:15:46-62
            
            bool x_22179 = sle64((int64_t) 0, zl_lhs_22176);
            
            // stdlib/regex.fut:15:46-62
            
            bool bounds_check_22180 = cond_22177 && x_22179;
            bool loop_not_taken_22810 = !cond_22177;
            bool protect_assert_disj_22811 = bounds_check_22180 || loop_not_taken_22810;
            bool loop_not_taken_22816 = !cond_21527;
            bool protect_assert_disj_22817 = protect_assert_disj_22811 || loop_not_taken_22816;
            
            // stdlib/regex.fut:15:46-62
            
            bool index_certs_22181;
            
            if (!protect_assert_disj_22817) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_22176, "] out of bounds for array of shape [", (long long) dz2080U_17516, "].", "-> #0  stdlib/regex.fut:15:46-62\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-fast-tests.fut:10:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:47:15-33
            
            bool x_21623 = sle64((int64_t) 0, used_21500);
            
            // stdlib/regex.fut:47:15-33
            
            bool y_21624 = slt64(used_21500, sizze_21480);
            
            // stdlib/regex.fut:47:15-33
            
            bool bounds_check_21625 = x_21623 && y_21624;
            
            // stdlib/regex.fut:47:15-33
            
            bool index_certs_21626;
            
            if (!bounds_check_21625) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) used_21500, "] out of bounds for array of shape [", (long long) sizze_21480, "].", "-> #0  stdlib/regex.fut:47:15-33\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:37:18-42:57
            
            bool cond_21600 = zeze_lhs_21506 == (int8_t) 46;
            bool x_22819 = !cond_21600;
            bool protect_cond_conj_22827 = loop_not_taken_22816 && x_22819;
            
            // stdlib/regex.fut:41:33-50
            
            int8_t loopres_f_res_f_res_f_res_21606;
            int64_t loopres_f_res_f_res_f_res_21607;
            
            if (protect_cond_conj_22827) {
                int8_t x_22821;
                int64_t x_22822;
                
                if (futrts_literal_11671(ctx, &x_22821, &x_22822, pattern_mem_22579, dz2080U_17516, p_21495) != 0) {
                    err = 1;
                    goto cleanup;
                }
                loopres_f_res_f_res_f_res_21606 = x_22821;
                loopres_f_res_f_res_f_res_21607 = x_22822;
            } else {
                loopres_f_res_f_res_f_res_21606 = (int8_t) 0;
                loopres_f_res_f_res_f_res_21607 = (int64_t) 0;
            }
            
            bool protect_cond_conj_22818 = cond_21527 && cond_22177;
            
            // stdlib/regex.fut:15:46-62
            
            int8_t zeze_lhs_22182;
            
            if (protect_cond_conj_22818) {
                int8_t x_22812 = ((int8_t *) pattern_mem_22579.mem)[zl_lhs_22176];
                
                zeze_lhs_22182 = x_22812;
            } else {
                zeze_lhs_22182 = (int8_t) 0;
            }
            // stdlib/regex.fut:15:63-68
            
            bool negative_t_res_22183 = zeze_lhs_22182 == (int8_t) 94;
            bool x_22828 = cond_22177 && negative_t_res_22183;
            
            // stdlib/regex.fut:36:54-74
            
            int64_t zp_rhs_22184 = btoi_bool_i64(x_22828);
            
            // stdlib/regex.fut:16:22-49
            
            int64_t start_22185 = add64(zl_lhs_22176, zp_rhs_22184);
            
            // stdlib/regex.fut:17:18-23:21
            
            bool cond_22187 = slt64(start_22185, dz2080U_17516);
            int64_t loopres_f_res_21529;
            
            if (cond_21527) {
                if (memblock_alloc(ctx, &mem_22608, (int64_t) 32, "mem_22608")) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/regex.fut:17:35-51
                for (int64_t nest_i_22847 = 0; nest_i_22847 < (int64_t) 4; nest_i_22847++) {
                    ((int64_t *) mem_22608.mem)[nest_i_22847] = (int64_t) 0;
                }
                // stdlib/regex.fut:17:18-23:21
                
                bool class_mask_res_22188;
                int64_t class_mask_res_22190;
                bool loop_while_22191;
                int64_t p_22193;
                
                if (memblock_set(ctx, &mem_param_22611, &mem_22608, "mem_22608") != 0)
                    return 1;
                loop_while_22191 = cond_22187;
                p_22193 = start_22185;
                while (loop_while_22191) {
                    // stdlib/regex.fut:19:23-40
                    
                    int8_t loopres_22194;
                    int64_t loopres_22195;
                    
                    if (futrts_literal_11671(ctx, &loopres_22194, &loopres_22195, pattern_mem_22579, dz2080U_17516, p_22193) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:20:23-25
                    
                    int64_t zl_lhs_22196 = add64((int64_t) 1, loopres_22195);
                    
                    // stdlib/regex.fut:20:19-90
                    
                    bool cond_22197 = slt64(zl_lhs_22196, dz2080U_17516);
                    
                    // stdlib/regex.fut:20:19-90
                    
                    bool cond_22198;
                    
                    if (cond_22197) {
                        // stdlib/regex.fut:20:46-59
                        
                        bool x_22199 = sle64((int64_t) 0, loopres_22195);
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool y_22200 = slt64(loopres_22195, dz2080U_17516);
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool bounds_check_22201 = x_22199 && y_22200;
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool index_certs_22202;
                        
                        if (!bounds_check_22201) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_22195, "] out of bounds for array of shape [", (long long) dz2080U_17516, "].", "-> #0  stdlib/regex.fut:20:46-59\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-fast-tests.fut:10:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:20:46-59
                        
                        int8_t zeze_lhs_22203 = ((int8_t *) pattern_mem_22579.mem)[loopres_22195];
                        
                        // stdlib/regex.fut:20:60-65
                        
                        bool cond_t_res_22204 = zeze_lhs_22203 == (int8_t) 45;
                        
                        cond_22198 = cond_t_res_22204;
                    } else {
                        cond_22198 = 0;
                    }
                    // stdlib/regex.fut:20:19-90
                    
                    bool range_22205;
                    
                    if (cond_22198) {
                        // stdlib/regex.fut:20:69-84
                        
                        bool x_22206 = sle64((int64_t) 0, zl_lhs_22196);
                        
                        // stdlib/regex.fut:20:69-84
                        
                        bool bounds_check_22207 = cond_22197 && x_22206;
                        
                        // stdlib/regex.fut:20:69-84
                        
                        bool index_certs_22208;
                        
                        if (!bounds_check_22207) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_22196, "] out of bounds for array of shape [", (long long) dz2080U_17516, "].", "-> #0  stdlib/regex.fut:20:69-84\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-fast-tests.fut:10:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:20:69-84
                        
                        int8_t znze_lhs_22209 = ((int8_t *) pattern_mem_22579.mem)[zl_lhs_22196];
                        
                        // stdlib/regex.fut:20:85-90
                        
                        bool range_t_res_22210 = znze_lhs_22209 == (int8_t) 93;
                        
                        // stdlib/regex.fut:20:85-90
                        
                        bool range_t_res_22211 = !range_t_res_22210;
                        
                        range_22205 = range_t_res_22211;
                    } else {
                        range_22205 = 0;
                    }
                    // stdlib/regex.fut:21:23-76
                    
                    int8_t loopres_22212;
                    int64_t loopres_22213;
                    
                    if (range_22205) {
                        // stdlib/regex.fut:21:37-60
                        
                        int8_t loopres_t_res_22214;
                        int64_t loopres_t_res_22215;
                        
                        if (futrts_literal_11671(ctx, &loopres_t_res_22214, &loopres_t_res_22215, pattern_mem_22579, dz2080U_17516, zl_lhs_22196) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        loopres_22212 = loopres_t_res_22214;
                        loopres_22213 = loopres_t_res_22215;
                    } else {
                        loopres_22212 = loopres_22194;
                        loopres_22213 = loopres_22195;
                    }
                    // stdlib/regex.fut:22:48-57
                    
                    int64_t u8_res_22216 = zext_i8_i64(loopres_22194);
                    
                    // stdlib/regex.fut:22:60-69
                    
                    int64_t u8_res_22217 = zext_i8_i64(loopres_22212);
                    
                    // stdlib/regex.fut:22:69-71
                    
                    int64_t mask_22218 = add64((int64_t) 1, u8_res_22217);
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool bounds_invalid_upwards_22219 = slt64(mask_22218, u8_res_22216);
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool valid_22221 = !bounds_invalid_upwards_22219;
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool range_valid_c_22222;
                    
                    if (!valid_22221) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) u8_res_22216, "..<", (long long) mask_22218, " is invalid.", "-> #0  stdlib/regex.fut:22:18-103\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-fast-tests.fut:10:12-27\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:22:18-103
                    
                    int64_t distance_22220 = sub64(mask_22218, u8_res_22216);
                    
                    // stdlib/regex.fut:22:18-103
                    if (memblock_set(ctx, &mem_param_22614, &mem_param_22611, "mem_param_22611") != 0)
                        return 1;
                    for (int64_t i_22224 = 0; i_22224 < distance_22220; i_22224++) {
                        // stdlib/regex.fut:36:54-74
                        
                        int64_t index_primexp_22226 = add64(u8_res_22216, i_22224);
                        
                        // stdlib/regex.fut:22:90-102
                        
                        int8_t unsign_arg0_22227 = zext_i64_i8(index_primexp_22226);
                        
                        // stdlib/regex.fut:22:75-102
                        if (futrts_mask_set_8529(ctx, &ext_mem_22617, mem_param_22614, unsign_arg0_22227) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        if (memblock_set(ctx, &mem_param_tmp_22852, &ext_mem_22617, "ext_mem_22617") != 0)
                            return 1;
                        if (memblock_set(ctx, &mem_param_22614, &mem_param_tmp_22852, "mem_param_tmp_22852") != 0)
                            return 1;
                    }
                    if (memblock_set(ctx, &ext_mem_22620, &mem_param_22614, "mem_param_22614") != 0)
                        return 1;
                    // stdlib/regex.fut:18:11-65
                    
                    bool cond_22229 = slt64(loopres_22213, dz2080U_17516);
                    
                    // stdlib/regex.fut:18:11-65
                    
                    bool loop_cond_22230;
                    
                    if (cond_22229) {
                        // stdlib/regex.fut:18:34-44
                        
                        bool x_22231 = sle64((int64_t) 0, loopres_22213);
                        
                        // stdlib/regex.fut:18:34-44
                        
                        bool bounds_check_22232 = cond_22229 && x_22231;
                        
                        // stdlib/regex.fut:18:34-44
                        
                        bool index_certs_22233;
                        
                        if (!bounds_check_22232) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_22213, "] out of bounds for array of shape [", (long long) dz2080U_17516, "].", "-> #0  stdlib/regex.fut:18:34-44\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-fast-tests.fut:10:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:18:34-44
                        
                        int8_t znze_lhs_22234 = ((int8_t *) pattern_mem_22579.mem)[loopres_22213];
                        
                        // stdlib/regex.fut:18:34-64
                        
                        bool cond_22235 = znze_lhs_22234 == (int8_t) 93;
                        
                        // stdlib/regex.fut:18:34-64
                        
                        bool cond_22236 = !cond_22235;
                        
                        // stdlib/regex.fut:18:56-64
                        
                        bool loop_cond_t_res_f_res_22237 = loopres_22213 == start_22185;
                        
                        // stdlib/regex.fut:36:54-74
                        
                        bool y_22238 = cond_22235 && loop_cond_t_res_f_res_22237;
                        
                        // stdlib/regex.fut:36:54-74
                        
                        bool loop_cond_t_res_22239 = cond_22236 || y_22238;
                        
                        loop_cond_22230 = loop_cond_t_res_22239;
                    } else {
                        loop_cond_22230 = 0;
                    }
                    if (memblock_set(ctx, &mem_param_tmp_22848, &ext_mem_22620, "ext_mem_22620") != 0)
                        return 1;
                    
                    bool loop_while_tmp_22849 = loop_cond_22230;
                    int64_t p_tmp_22851 = loopres_22213;
                    
                    if (memblock_set(ctx, &mem_param_22611, &mem_param_tmp_22848, "mem_param_tmp_22848") != 0)
                        return 1;
                    loop_while_22191 = loop_while_tmp_22849;
                    p_22193 = p_tmp_22851;
                }
                if (memblock_set(ctx, &ext_mem_22623, &mem_param_22611, "mem_param_22611") != 0)
                    return 1;
                class_mask_res_22188 = loop_while_22191;
                class_mask_res_22190 = p_22193;
                if (memblock_unref(ctx, &mem_22608, "mem_22608") != 0)
                    return 1;
                if (x_22828) {
                    if (memblock_alloc(ctx, &mem_22624, (int64_t) 32, "mem_22624")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:24:24-56
                    for (int64_t i_22565 = 0; i_22565 < (int64_t) 4; i_22565++) {
                        int64_t eta_p_22242 = ((int64_t *) ext_mem_22623.mem)[i_22565];
                        
                        // stdlib/regex.fut:24:37-50
                        
                        int64_t lifted_lambda_res_22243 = (int64_t) -1 ^ eta_p_22242;
                        
                        ((int64_t *) mem_22624.mem)[i_22565] = lifted_lambda_res_22243;
                    }
                    if (memblock_set(ctx, &ext_mem_22633, &mem_22624, "mem_22624") != 0)
                        return 1;
                } else if (memblock_set(ctx, &ext_mem_22633, &ext_mem_22623, "ext_mem_22623") != 0)
                    return 1;
                if (memblock_unref(ctx, &ext_mem_22623, "ext_mem_22623") != 0)
                    return 1;
                // stdlib/regex.fut:24:68-70
                
                int64_t tmp_22247 = add64((int64_t) 1, class_mask_res_22190);
                
                if (memblock_set(ctx, &ext_mem_22636, &ext_mem_22633, "ext_mem_22633") != 0)
                    return 1;
                loopres_f_res_21529 = tmp_22247;
            } else {
                int64_t loopres_f_res_f_res_21602;
                
                if (cond_21600) {
                    loopres_f_res_f_res_21602 = zl_lhs_22176;
                } else {
                    loopres_f_res_f_res_21602 = loopres_f_res_f_res_f_res_21607;
                }
                if (cond_21600) {
                    if (memblock_alloc(ctx, &mem_22606, (int64_t) 32, "mem_22606")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:38:26-49
                    for (int64_t nest_i_22855 = 0; nest_i_22855 < (int64_t) 4; nest_i_22855++) {
                        ((int64_t *) mem_22606.mem)[nest_i_22855] = (int64_t) -1;
                    }
                    // stdlib/regex.fut:39:19-51
                    ((int64_t *) mem_22606.mem)[(int64_t) 0] = (int64_t) -1025;
                    if (memblock_set(ctx, &ext_mem_22607, &mem_22606, "mem_22606") != 0)
                        return 1;
                } else {
                    if (memblock_alloc(ctx, &mem_22604, (int64_t) 32, "mem_22604")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:42:32-48
                    for (int64_t nest_i_22856 = 0; nest_i_22856 < (int64_t) 4; nest_i_22856++) {
                        ((int64_t *) mem_22604.mem)[nest_i_22856] = (int64_t) 0;
                    }
                    // stdlib/regex.fut:42:22-51
                    if (futrts_mask_set_8529(ctx, &ext_mem_22605, mem_22604, loopres_f_res_f_res_f_res_21606) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &mem_22604, "mem_22604") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_22607, &ext_mem_22605, "ext_mem_22605") != 0)
                        return 1;
                }
                if (memblock_set(ctx, &ext_mem_22636, &ext_mem_22607, "ext_mem_22607") != 0)
                    return 1;
                loopres_f_res_21529 = loopres_f_res_f_res_21602;
            }
            // stdlib/regex.fut:43:28-46:54
            
            bool cond_21610 = sle64(dz2080U_17516, loopres_f_res_21529);
            
            // stdlib/regex.fut:43:28-46:54
            
            int64_t quantifier_21611;
            
            if (cond_21610) {
                quantifier_21611 = (int64_t) 1;
            } else {
                // stdlib/regex.fut:44:21-34
                
                bool x_21612 = sle64((int64_t) 0, loopres_f_res_21529);
                
                // stdlib/regex.fut:44:21-34
                
                bool y_21613 = slt64(loopres_f_res_21529, dz2080U_17516);
                
                // stdlib/regex.fut:44:21-34
                
                bool bounds_check_21614 = x_21612 && y_21613;
                
                // stdlib/regex.fut:44:21-34
                
                bool index_certs_21615;
                
                if (!bounds_check_21614) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_f_res_21529, "] out of bounds for array of shape [", (long long) dz2080U_17516, "].", "-> #0  stdlib/regex.fut:44:21-34\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:44:21-34
                
                int8_t zeze_lhs_21616 = ((int8_t *) pattern_mem_22579.mem)[loopres_f_res_21529];
                
                // stdlib/regex.fut:44:18-46:54
                
                bool cond_21617 = zeze_lhs_21616 == (int8_t) 42;
                
                // stdlib/regex.fut:44:18-46:54
                
                int64_t quantifier_f_res_21618;
                
                if (cond_21617) {
                    quantifier_f_res_21618 = (int64_t) 0;
                } else {
                    // stdlib/regex.fut:45:18-46:54
                    
                    bool cond_21619 = zeze_lhs_21616 == (int8_t) 43;
                    
                    // stdlib/regex.fut:45:18-46:54
                    
                    int64_t quantifier_f_res_f_res_21620;
                    
                    if (cond_21619) {
                        quantifier_f_res_f_res_21620 = (int64_t) 2;
                    } else {
                        // stdlib/regex.fut:46:18-54
                        
                        bool cond_21621 = zeze_lhs_21616 == (int8_t) 63;
                        
                        // stdlib/regex.fut:46:18-54
                        
                        int64_t quantifier_f_res_f_res_f_res_21622;
                        
                        if (cond_21621) {
                            quantifier_f_res_f_res_f_res_21622 = (int64_t) 3;
                        } else {
                            quantifier_f_res_f_res_f_res_21622 = (int64_t) 1;
                        }
                        quantifier_f_res_f_res_21620 = quantifier_f_res_f_res_f_res_21622;
                    }
                    quantifier_f_res_21618 = quantifier_f_res_f_res_21620;
                }
                quantifier_21611 = quantifier_f_res_21618;
            }
            // stdlib/regex.fut:47:15-33
            // stdlib/regex.fut:47:15-33
            lmad_copy_8b(ctx, 1, (uint64_t *) mem_22584.mem, used_21500 * (int64_t) 4, (int64_t []) {(int64_t) 1}, (uint64_t *) ext_mem_22636.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {(int64_t) 4});
            if (memblock_unref(ctx, &ext_mem_22636, "ext_mem_22636") != 0)
                return 1;
            // stdlib/regex.fut:48:15-41
            ((int64_t *) mem_22586.mem)[used_21500] = quantifier_21611;
            // stdlib/regex.fut:49:21-53
            
            bool cond_21629 = quantifier_21611 == (int64_t) 1;
            
            // probes/stdlib-fast-tests.fut:10:12-27
            
            bool cond_neg_21630 = !cond_21629;
            
            // probes/stdlib-fast-tests.fut:10:12-27
            
            int64_t zp_rhs_21631 = btoi_bool_i64(cond_neg_21630);
            
            // stdlib/regex.fut:49:19-53
            
            int64_t tmp_21632 = add64(loopres_f_res_21529, zp_rhs_21631);
            
            // stdlib/regex.fut:49:85-87
            
            int64_t tmp_21633 = add64((int64_t) 1, used_21500);
            
            loopres_21508 = tmp_21632;
            loopres_21513 = tmp_21633;
            loopres_21514 = branch_21501;
        }
        // stdlib/regex.fut:30:15-31
        
        bool loop_cond_21634 = slt64(loopres_21508, dz2080U_17516);
        bool loop_while_tmp_22839 = loop_cond_21634;
        int64_t p_tmp_22840 = loopres_21508;
        int64_t used_tmp_22845 = loopres_21513;
        int64_t branch_tmp_22846 = loopres_21514;
        
        loop_while_21494 = loop_while_tmp_22839;
        p_21495 = p_tmp_22840;
        used_21500 = used_tmp_22845;
        branch_21501 = branch_tmp_22846;
    }
    compile_res_21486 = loop_while_21494;
    compile_res_21487 = p_21495;
    compile_res_21492 = used_21500;
    compile_res_21493 = branch_21501;
    // stdlib/regex.fut:51:47-57
    
    int64_t dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635 = add64((int64_t) 1, compile_res_21493);
    
    // stdlib/regex.fut:50:7-28
    
    bool x_21636 = sle64((int64_t) 0, compile_res_21493);
    
    // stdlib/regex.fut:50:7-28
    
    bool y_21637 = slt64(compile_res_21493, sizze_21480);
    
    // stdlib/regex.fut:50:7-28
    
    bool bounds_check_21638 = x_21636 && y_21637;
    
    // stdlib/regex.fut:50:7-28
    
    bool index_certs_21639;
    
    if (!bounds_check_21638) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) compile_res_21493, "] out of bounds for array of shape [", (long long) sizze_21480, "].", "-> #0  stdlib/regex.fut:50:7-28\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:50:7-28
    ((int64_t *) mem_22590.mem)[compile_res_21493] = compile_res_21492;
    // stdlib/regex.fut:51:7-22
    
    bool empty_slice_21641 = compile_res_21492 == (int64_t) 0;
    
    // stdlib/regex.fut:51:7-22
    
    int64_t m_21642 = sub64(compile_res_21492, (int64_t) 1);
    
    // stdlib/regex.fut:51:7-22
    
    bool zzero_leq_i_p_m_t_s_21643 = sle64((int64_t) 0, m_21642);
    
    // stdlib/regex.fut:51:7-22
    
    bool i_p_m_t_s_leq_w_21644 = slt64(m_21642, sizze_21480);
    
    // stdlib/regex.fut:51:7-22
    
    bool i_lte_j_21645 = sle64((int64_t) 0, compile_res_21492);
    
    // stdlib/regex.fut:51:7-22
    
    bool y_21646 = zzero_leq_i_p_m_t_s_21643 && i_p_m_t_s_leq_w_21644;
    
    // stdlib/regex.fut:51:7-22
    
    bool forwards_ok_21647 = i_lte_j_21645 && y_21646;
    
    // stdlib/regex.fut:51:7-22
    
    bool ok_or_empty_21648 = empty_slice_21641 || forwards_ok_21647;
    
    // stdlib/regex.fut:51:7-22
    
    bool index_certs_21649;
    
    if (!ok_or_empty_21648) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) compile_res_21492, "] out of bounds for array of shape [", (long long) sizze_21480, "].", "-> #0  stdlib/regex.fut:51:7-22\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:23-40
    
    bool ok_or_empty_21651 = empty_slice_21641 || y_21646;
    
    // stdlib/regex.fut:51:23-40
    
    bool index_certs_21652;
    
    if (!ok_or_empty_21651) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) compile_res_21492, "] out of bounds for array of shape [", (long long) sizze_21480, "].", "-> #0  stdlib/regex.fut:51:23-40\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:41-65
    
    bool empty_slice_21654 = dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635 == (int64_t) 0;
    
    // stdlib/regex.fut:51:41-65
    
    bool i_lte_j_21655 = sle64((int64_t) 0, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635);
    
    // stdlib/regex.fut:51:41-65
    
    bool forwards_ok_21656 = bounds_check_21638 && i_lte_j_21655;
    
    // stdlib/regex.fut:51:41-65
    
    bool ok_or_empty_21657 = empty_slice_21654 || forwards_ok_21656;
    
    // stdlib/regex.fut:51:41-65
    
    bool index_certs_21658;
    
    if (!ok_or_empty_21657) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635, "] out of bounds for array of shape [", (long long) sizze_21480, "].", "-> #0  stdlib/regex.fut:51:41-65\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:66-88
    
    bool ok_or_empty_21660 = bounds_check_21638 || empty_slice_21654;
    
    // stdlib/regex.fut:51:66-88
    
    bool index_certs_21661;
    
    if (!ok_or_empty_21660) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635, "] out of bounds for array of shape [", (long long) sizze_21480, "].", "-> #0  stdlib/regex.fut:51:66-88\n   #1  probes/stdlib-fast-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:94:15-22
    
    bool y_21663 = slt64((int64_t) 0, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635);
    
    // stdlib/regex.fut:94:15-22
    
    bool index_certs_21664;
    
    if (!y_21663) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635, "].", "-> #0  stdlib/regex.fut:94:15-22\n   #1  probes/stdlib-fast-tests.fut:10:7-11:20\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:102:42-44
    
    int64_t zt_lhs_21771 = add64((int64_t) 1, dz2081U_17517);
    
    // stdlib/regex.fut:102:45-64
    
    int64_t zp_rhs_21772 = mul64(dz2082U_17518, zt_lhs_21771);
    
    // stdlib/regex.fut:102:29-64
    
    int64_t capacity_21773 = add64(dz2081U_17517, zp_rhs_21772);
    
    if (memblock_alloc(ctx, &mem_22665, capacity_21773, "mem_22665")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:103:63-85
    for (int64_t nest_i_22857 = 0; nest_i_22857 < capacity_21773; nest_i_22857++) {
        ((int8_t *) mem_22665.mem)[nest_i_22857] = (int8_t) 0;
    }
    // stdlib/regex.fut:103:30-119:72
    
    bool replace_res_21775;
    int64_t replace_res_21776;
    int64_t replace_res_21778;
    bool loop_while_21779;
    int64_t position_21780;
    int64_t used_21782;
    
    loop_while_21779 = 1;
    position_21780 = (int64_t) 0;
    used_21782 = (int64_t) 0;
    while (loop_while_21779) {
        // stdlib/regex.fut:101:14-105:47
        
        int64_t loopres_21783;
        int64_t loopres_21784;
        
        if (futrts_find_11687(ctx, &loopres_21783, &loopres_21784, mem_22584, mem_22586, mem_22588, mem_22590, text_mem_22580, compile_res_21492, compile_res_21492, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635, dz2081U_17517, position_21780) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/regex.fut:106:20-59
        
        bool cond_21785 = slt64(loopres_21784, (int64_t) 0);
        
        // stdlib/regex.fut:106:20-59
        
        int64_t finish_21786;
        
        if (cond_21785) {
            finish_21786 = dz2081U_17517;
        } else {
            finish_21786 = loopres_21783;
        }
        // stdlib/regex.fut:107:17-109:15
        
        bool bounds_invalid_upwards_21787 = slt64(finish_21786, position_21780);
        
        // stdlib/regex.fut:107:17-109:15
        
        bool valid_21789 = !bounds_invalid_upwards_21787;
        
        // stdlib/regex.fut:107:17-109:15
        
        bool range_valid_c_21790;
        
        if (!valid_21789) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) position_21780, "..<", (long long) finish_21786, " is invalid.", "-> #0  stdlib/regex.fut:107:17-109:15\n   #1  probes/stdlib-fast-tests.fut:10:7-11:48\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:107:17-109:15
        
        int64_t distance_21788 = sub64(finish_21786, position_21780);
        
        // stdlib/regex.fut:107:17-109:15
        for (int64_t i_21792 = 0; i_21792 < distance_21788; i_21792++) {
            // probes/stdlib-fast-tests.fut:10:7-11:48
            
            int64_t index_primexp_21794 = add64(position_21780, i_21792);
            
            // stdlib/regex.fut:108:21-23
            
            int64_t zm_lhs_21795 = add64(used_21782, index_primexp_21794);
            
            // stdlib/regex.fut:108:23-32
            
            int64_t tmp_21796 = sub64(zm_lhs_21795, position_21780);
            
            // stdlib/regex.fut:108:36-43
            
            bool x_21797 = sle64((int64_t) 0, index_primexp_21794);
            
            // stdlib/regex.fut:108:36-43
            
            bool y_21798 = slt64(index_primexp_21794, dz2081U_17517);
            
            // stdlib/regex.fut:108:36-43
            
            bool bounds_check_21799 = x_21797 && y_21798;
            
            // stdlib/regex.fut:108:36-43
            
            bool index_certs_21800;
            
            if (!bounds_check_21799) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) index_primexp_21794, "] out of bounds for array of shape [", (long long) dz2081U_17517, "].", "-> #0  stdlib/regex.fut:108:36-43\n   #1  probes/stdlib-fast-tests.fut:10:7-11:48\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:108:13-43
            
            bool x_21802 = sle64((int64_t) 0, tmp_21796);
            
            // stdlib/regex.fut:108:13-43
            
            bool y_21803 = slt64(tmp_21796, capacity_21773);
            
            // stdlib/regex.fut:108:13-43
            
            bool bounds_check_21804 = x_21802 && y_21803;
            
            // stdlib/regex.fut:108:13-43
            
            bool index_certs_21805;
            
            if (!bounds_check_21804) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_21796, "] out of bounds for array of shape [", (long long) capacity_21773, "].", "-> #0  stdlib/regex.fut:108:13-43\n   #1  probes/stdlib-fast-tests.fut:10:7-11:48\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:108:36-43
            
            int8_t tmp_21801 = ((int8_t *) text_mem_22580.mem)[index_primexp_21794];
            
            // stdlib/regex.fut:108:13-43
            ((int8_t *) mem_22665.mem)[tmp_21796] = tmp_21801;
        }
        // stdlib/regex.fut:110:22-29
        
        int64_t zm_lhs_21807 = add64(used_21782, finish_21786);
        
        // stdlib/regex.fut:110:29-38
        
        int64_t used_21808 = sub64(zm_lhs_21807, position_21780);
        int64_t loopres_21809;
        int64_t loopres_21811;
        
        if (cond_21785) {
            loopres_21809 = zt_lhs_21771;
            loopres_21811 = used_21808;
        } else {
            // stdlib/regex.fut:112:25-114:23
            for (int64_t i_21813 = 0; i_21813 < dz2082U_17518; i_21813++) {
                // stdlib/regex.fut:113:29-31
                
                int64_t tmp_21815 = add64(used_21808, i_21813);
                
                // stdlib/regex.fut:113:21-49
                
                bool x_21817 = sle64((int64_t) 0, tmp_21815);
                
                // stdlib/regex.fut:113:21-49
                
                bool y_21818 = slt64(tmp_21815, capacity_21773);
                
                // stdlib/regex.fut:113:21-49
                
                bool bounds_check_21819 = x_21817 && y_21818;
                
                // stdlib/regex.fut:113:21-49
                
                bool index_certs_21820;
                
                if (!bounds_check_21819) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_21815, "] out of bounds for array of shape [", (long long) capacity_21773, "].", "-> #0  stdlib/regex.fut:113:21-49\n   #1  probes/stdlib-fast-tests.fut:10:7-11:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:113:35-49
                
                int8_t tmp_21816 = ((int8_t *) replacement_mem_22581.mem)[i_21813];
                
                // stdlib/regex.fut:113:21-49
                ((int8_t *) mem_22665.mem)[tmp_21815] = tmp_21816;
            }
            // stdlib/regex.fut:115:30-49
            
            int64_t used_21822 = add64(dz2082U_17518, used_21808);
            
            // stdlib/regex.fut:116:18-119:72
            
            bool cond_21823 = loopres_21783 == loopres_21784;
            
            // stdlib/regex.fut:116:43-56
            
            bool cond_t_res_21824 = slt64(loopres_21784, dz2081U_17517);
            
            // probes/stdlib-fast-tests.fut:10:7-11:48
            
            bool x_21825 = cond_21823 && cond_t_res_21824;
            
            // stdlib/regex.fut:116:18-119:72
            
            int64_t loopres_f_res_21826;
            int64_t loopres_f_res_21828;
            
            if (x_21825) {
                // stdlib/regex.fut:117:36-46
                
                bool x_22299 = sle64((int64_t) 0, loopres_21784);
                
                // stdlib/regex.fut:117:36-46
                
                bool bounds_check_22300 = cond_t_res_21824 && x_22299;
                
                // stdlib/regex.fut:117:36-46
                
                bool index_certs_22301;
                
                if (!bounds_check_22300) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_21784, "] out of bounds for array of shape [", (long long) dz2081U_17517, "].", "-> #0  stdlib/regex.fut:117:36-46\n   #1  probes/stdlib-fast-tests.fut:10:7-11:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:117:24-46
                
                bool x_22303 = sle64((int64_t) 0, used_21822);
                
                // stdlib/regex.fut:117:24-46
                
                bool y_22304 = slt64(used_21822, capacity_21773);
                
                // stdlib/regex.fut:117:24-46
                
                bool bounds_check_22305 = x_22303 && y_22304;
                
                // stdlib/regex.fut:117:24-46
                
                bool index_certs_22306;
                
                if (!bounds_check_22305) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) used_21822, "] out of bounds for array of shape [", (long long) capacity_21773, "].", "-> #0  stdlib/regex.fut:117:24-46\n   #1  probes/stdlib-fast-tests.fut:10:7-11:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:117:36-46
                
                int8_t tmp_22302 = ((int8_t *) text_mem_22580.mem)[loopres_21784];
                
                // stdlib/regex.fut:117:24-46
                ((int8_t *) mem_22665.mem)[used_21822] = tmp_22302;
                // stdlib/regex.fut:118:28-30
                
                int64_t tmp_22308 = add64((int64_t) 1, loopres_21784);
                
                // stdlib/regex.fut:118:39-41
                
                int64_t tmp_22309 = add64((int64_t) 1, used_21822);
                
                loopres_f_res_21826 = tmp_22308;
                loopres_f_res_21828 = tmp_22309;
            } else {
                // stdlib/regex.fut:119:24-62
                
                int64_t tmp_21840;
                
                if (cond_21823) {
                    // stdlib/regex.fut:119:50-52
                    
                    int64_t tmp_t_res_22310 = add64((int64_t) 1, loopres_21784);
                    
                    tmp_21840 = tmp_t_res_22310;
                } else {
                    tmp_21840 = loopres_21784;
                }
                loopres_f_res_21826 = tmp_21840;
                loopres_f_res_21828 = used_21822;
            }
            loopres_21809 = loopres_f_res_21826;
            loopres_21811 = loopres_f_res_21828;
        }
        // stdlib/regex.fut:104:20-34
        
        bool loop_cond_21842 = sle64(loopres_21809, dz2081U_17517);
        bool loop_while_tmp_22858 = loop_cond_21842;
        int64_t position_tmp_22859 = loopres_21809;
        int64_t used_tmp_22861 = loopres_21811;
        
        loop_while_21779 = loop_while_tmp_22858;
        position_21780 = position_tmp_22859;
        used_21782 = used_tmp_22861;
    }
    replace_res_21775 = loop_while_21779;
    replace_res_21776 = position_21780;
    replace_res_21778 = used_21782;
    // stdlib/regex.fut:120:6-19
    
    bool empty_slice_21843 = replace_res_21778 == (int64_t) 0;
    
    // stdlib/regex.fut:120:6-19
    
    int64_t m_21844 = sub64(replace_res_21778, (int64_t) 1);
    
    // stdlib/regex.fut:120:6-19
    
    bool zzero_leq_i_p_m_t_s_21845 = sle64((int64_t) 0, m_21844);
    
    // stdlib/regex.fut:120:6-19
    
    bool i_p_m_t_s_leq_w_21846 = slt64(m_21844, capacity_21773);
    
    // stdlib/regex.fut:120:6-19
    
    bool i_lte_j_21847 = sle64((int64_t) 0, replace_res_21778);
    
    // stdlib/regex.fut:120:6-19
    
    bool y_21848 = zzero_leq_i_p_m_t_s_21845 && i_p_m_t_s_leq_w_21846;
    
    // stdlib/regex.fut:120:6-19
    
    bool forwards_ok_21849 = i_lte_j_21847 && y_21848;
    
    // stdlib/regex.fut:120:6-19
    
    bool ok_or_empty_21850 = empty_slice_21843 || forwards_ok_21849;
    
    // stdlib/regex.fut:120:6-19
    
    bool index_certs_21851;
    
    if (!ok_or_empty_21850) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) replace_res_21778, "] out of bounds for array of shape [", (long long) capacity_21773, "].", "-> #0  stdlib/regex.fut:120:6-19\n   #1  probes/stdlib-fast-tests.fut:10:7-11:48\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    
    int64_t zm_lhs_21665 = ((int64_t *) mem_22590.mem)[(int64_t) 0];
    int64_t zm_rhs_21666 = ((int64_t *) mem_22588.mem)[(int64_t) 0];
    
    // stdlib/regex.fut:94:22-30
    
    int64_t width_21667 = sub64(zm_lhs_21665, zm_rhs_21666);
    
    // stdlib/regex.fut:95:15-103
    
    bool cond_21668 = slt64((int64_t) 0, width_21667);
    
    // stdlib/regex.fut:95:15-103
    
    bool cond_21669;
    
    if (cond_21668) {
        // stdlib/regex.fut:95:28-42
        
        bool defunc_0_reduce_res_22252;
        bool redout_22567 = 1;
        
        for (int64_t i_22568 = 0; i_22568 < compile_res_21492; i_22568++) {
            int64_t eta_p_22253 = ((int64_t *) mem_22586.mem)[i_22568];
            
            // stdlib/regex.fut:95:32-37
            
            bool lifted_lambda_res_22254 = eta_p_22253 == (int64_t) 1;
            
            // stdlib/regex.fut:95:28-42
            
            bool x_22257 = lifted_lambda_res_22254 && redout_22567;
            bool redout_tmp_22864 = x_22257;
            
            redout_22567 = redout_tmp_22864;
        }
        defunc_0_reduce_res_22252 = redout_22567;
        cond_21669 = defunc_0_reduce_res_22252;
    } else {
        cond_21669 = 0;
    }
    // stdlib/regex.fut:95:15-103
    
    bool fixed_21679;
    
    if (cond_21669) {
        // stdlib/regex.fut:95:46-103
        
        bool defunc_0_reduce_res_22262;
        bool redout_22569 = 1;
        
        for (int64_t i_22570 = 0; i_22570 < dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635; i_22570++) {
            int64_t zm_lhs_22268 = ((int64_t *) mem_22590.mem)[i_22570];
            int64_t zm_rhs_22269 = ((int64_t *) mem_22588.mem)[i_22570];
            
            // stdlib/regex.fut:95:64-72
            
            int64_t zeze_lhs_22270 = sub64(zm_lhs_22268, zm_rhs_22269);
            
            // stdlib/regex.fut:95:73-81
            
            bool lifted_lambda_res_22271 = zeze_lhs_22270 == width_21667;
            
            // stdlib/regex.fut:95:46-103
            
            bool x_22274 = lifted_lambda_res_22271 && redout_22569;
            bool redout_tmp_22865 = x_22274;
            
            redout_22569 = redout_tmp_22865;
        }
        defunc_0_reduce_res_22262 = redout_22569;
        fixed_21679 = defunc_0_reduce_res_22262;
    } else {
        fixed_21679 = 0;
    }
    // stdlib/regex.fut:96:6-100:13
    
    bool cond_21697 = !fixed_21679;
    
    // stdlib/regex.fut:97:88-94
    
    int64_t zp_lhs_21698 = sub64(dz2081U_17517, width_21667);
    
    // stdlib/regex.fut:97:94-96
    
    int64_t max_arg1_21699 = add64((int64_t) 1, zp_lhs_21698);
    
    // probes/stdlib-fast-tests.fut:10:7-11:20
    
    int64_t max_res_21700 = smax64((int64_t) 0, max_arg1_21699);
    
    // stdlib/regex.fut:96:6-100:13
    
    int64_t count_res_21701;
    
    if (cond_21697) {
        // stdlib/regex.fut:81:27-84:61
        
        bool count_general_res_22279;
        int64_t count_general_res_22280;
        int64_t count_general_res_22281;
        bool loop_while_22282;
        int64_t position_22283;
        int64_t count_22284;
        
        loop_while_22282 = 1;
        position_22283 = (int64_t) 0;
        count_22284 = (int64_t) 0;
        while (loop_while_22282) {
            // stdlib/regex.fut:80:20-82:45
            
            int64_t loopres_22285;
            int64_t loopres_22286;
            
            if (futrts_find_11687(ctx, &loopres_22285, &loopres_22286, mem_22584, mem_22586, mem_22588, mem_22590, text_mem_22580, compile_res_21492, compile_res_21492, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635, dz2081U_17517, position_22283) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/regex.fut:83:8-84:61
            
            bool cond_22287 = slt64(loopres_22286, (int64_t) 0);
            
            // stdlib/regex.fut:83:8-84:61
            
            int64_t loopres_22288;
            int64_t loopres_22289;
            
            if (cond_22287) {
                loopres_22288 = zt_lhs_21771;
                loopres_22289 = count_22284;
            } else {
                // stdlib/regex.fut:84:14-52
                
                bool cond_22291 = loopres_22286 == loopres_22285;
                
                // stdlib/regex.fut:84:14-52
                
                int64_t tmp_22292;
                
                if (cond_22291) {
                    // stdlib/regex.fut:84:40-42
                    
                    int64_t tmp_t_res_22293 = add64((int64_t) 1, loopres_22286);
                    
                    tmp_22292 = tmp_t_res_22293;
                } else {
                    tmp_22292 = loopres_22286;
                }
                // stdlib/regex.fut:84:58-60
                
                int64_t tmp_22294 = add64((int64_t) 1, count_22284);
                
                loopres_22288 = tmp_22292;
                loopres_22289 = tmp_22294;
            }
            // stdlib/regex.fut:81:78-92
            
            bool loop_cond_22295 = sle64(loopres_22288, dz2081U_17517);
            bool loop_while_tmp_22866 = loop_cond_22295;
            int64_t position_tmp_22867 = loopres_22288;
            int64_t count_tmp_22868 = loopres_22289;
            
            loop_while_22282 = loop_while_tmp_22866;
            position_22283 = position_tmp_22867;
            count_22284 = count_tmp_22868;
        }
        count_general_res_22279 = loop_while_22282;
        count_general_res_22280 = position_22283;
        count_general_res_22281 = count_22284;
        count_res_21701 = count_general_res_22281;
    } else {
        // stdlib/regex.fut:97:60-97
        
        bool bounds_invalid_upwards_21719 = slt64(max_res_21700, (int64_t) 0);
        
        // stdlib/regex.fut:97:60-97
        
        bool valid_21720 = !bounds_invalid_upwards_21719;
        
        // stdlib/regex.fut:97:60-97
        
        bool range_valid_c_21721;
        
        if (!valid_21720) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) (int64_t) 0, "..", (long long) (int64_t) 1, "..<", (long long) max_res_21700, " is invalid.", "-> #0  stdlib/regex.fut:97:60-97\n   #1  probes/stdlib-fast-tests.fut:10:7-11:20\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        if (mem_22650_cached_sizze_22964 < max_res_21700) {
            err = lexical_realloc(ctx, &mem_22650, &mem_22650_cached_sizze_22964, max_res_21700);
            if (err != FUTHARK_SUCCESS)
                goto cleanup;
        }
        // stdlib/regex.fut:97:19-99
        for (int64_t i_22573 = 0; i_22573 < max_res_21700; i_22573++) {
            // stdlib/regex.fut:87:3-92:12
            
            bool fixed_match_res_21725;
            bool found_21727 = 0;
            
            for (int64_t branch_21726 = 0; branch_21726 < dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_21635; branch_21726++) {
                // stdlib/regex.fut:88:5-92:12
                
                bool loopres_21728;
                
                if (found_21727) {
                    loopres_21728 = 1;
                } else {
                    int64_t first_21729 = ((int64_t *) mem_22588.mem)[branch_21726];
                    
                    // stdlib/regex.fut:90:21-91:80
                    
                    bool loopres_f_res_21731;
                    int64_t loopres_f_res_21732;
                    bool loopres_f_res_21733;
                    bool loop_while_21734;
                    int64_t i_21735;
                    bool ok_21736;
                    
                    loop_while_21734 = cond_21668;
                    i_21735 = (int64_t) 0;
                    ok_21736 = 1;
                    while (loop_while_21734) {
                        // stdlib/regex.fut:91:11-13
                        
                        int64_t tmp_21737 = add64((int64_t) 1, i_21735);
                        
                        // stdlib/regex.fut:91:75-77
                        
                        int64_t token_accepts_arg2_21738 = add64(i_21735, i_22573);
                        
                        // stdlib/regex.fut:91:62-78
                        
                        int8_t token_accepts_arg2_21739 = ((int8_t *) text_mem_22580.mem)[token_accepts_arg2_21738];
                        
                        // stdlib/regex.fut:91:47-49
                        
                        int64_t token_accepts_arg1_21740 = add64(first_21729, i_21735);
                        
                        // stdlib/regex.fut:55:26-34
                        
                        int64_t u8_res_21741 = zext_i8_i64(token_accepts_arg2_21739);
                        
                        // stdlib/regex.fut:55:34-37
                        
                        int64_t zgzg_lhs_21742 = sdiv64(u8_res_21741, (int64_t) 64);
                        int64_t zgzg_lhs_21743 = ((int64_t *) mem_22584.mem)[token_accepts_arg1_21740 * (int64_t) 4 + zgzg_lhs_21742];
                        
                        // stdlib/regex.fut:55:51-54
                        
                        int8_t u8_arg0_21744 = umod8(token_accepts_arg2_21739, (int8_t) 64);
                        
                        // stdlib/regex.fut:55:42-54
                        
                        int64_t u8_res_21745 = zext_i8_i64(u8_arg0_21744);
                        
                        // stdlib/regex.fut:55:39-55
                        
                        int64_t za_lhs_21746 = lshr64(zgzg_lhs_21743, u8_res_21745);
                        
                        // stdlib/regex.fut:55:56-58
                        
                        int64_t znze_lhs_21747 = (int64_t) 1 & za_lhs_21746;
                        
                        // stdlib/regex.fut:55:60-64
                        
                        bool token_accepts_res_21748 = znze_lhs_21747 == (int64_t) 0;
                        
                        // stdlib/regex.fut:55:60-64
                        
                        bool token_accepts_res_21749 = !token_accepts_res_21748;
                        
                        // stdlib/regex.fut:90:53-68
                        
                        bool cond_21750 = slt64(tmp_21737, width_21667);
                        
                        // stdlib/regex.fut:93:12-97:57
                        
                        bool x_21751 = token_accepts_res_21749 && cond_21750;
                        bool loop_while_tmp_22871 = x_21751;
                        int64_t i_tmp_22872 = tmp_21737;
                        bool ok_tmp_22873 = token_accepts_res_21749;
                        
                        loop_while_21734 = loop_while_tmp_22871;
                        i_21735 = i_tmp_22872;
                        ok_21736 = ok_tmp_22873;
                    }
                    loopres_f_res_21731 = loop_while_21734;
                    loopres_f_res_21732 = i_21735;
                    loopres_f_res_21733 = ok_21736;
                    loopres_21728 = loopres_f_res_21733;
                }
                
                bool found_tmp_22870 = loopres_21728;
                
                found_21727 = found_tmp_22870;
            }
            fixed_match_res_21725 = found_21727;
            ((bool *) mem_22650)[i_22573] = fixed_match_res_21725;
        }
        // stdlib/regex.fut:98:22-99:60
        
        bool loop_cond_21753 = slt64((int64_t) 0, max_res_21700);
        
        // stdlib/regex.fut:98:22-99:60
        
        bool count_res_f_res_21754;
        int64_t count_res_f_res_21755;
        int64_t count_res_f_res_21756;
        bool loop_while_21757;
        int64_t i_21758;
        int64_t total_21759;
        
        loop_while_21757 = loop_cond_21753;
        i_21758 = (int64_t) 0;
        total_21759 = (int64_t) 0;
        while (loop_while_21757) {
            // stdlib/regex.fut:99:7-60
            
            bool x_21760 = sle64((int64_t) 0, i_21758);
            
            // stdlib/regex.fut:99:7-60
            
            bool y_21761 = slt64(i_21758, max_res_21700);
            
            // stdlib/regex.fut:99:7-60
            
            bool bounds_check_21762 = x_21760 && y_21761;
            
            // stdlib/regex.fut:99:7-60
            
            bool index_certs_21763;
            
            if (!bounds_check_21762) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_21758, "] out of bounds for array of shape [", (long long) max_res_21700, "].", "-> #0  stdlib/regex.fut:99:7-60\n   #1  probes/stdlib-fast-tests.fut:10:7-11:20\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:99:7-60
            
            bool cond_21764 = ((bool *) mem_22650)[i_21758];
            
            // stdlib/regex.fut:99:7-60
            
            int64_t loopres_21765;
            int64_t loopres_21766;
            
            if (cond_21764) {
                // stdlib/regex.fut:99:28-34
                
                int64_t tmp_22296 = add64(width_21667, i_21758);
                
                // stdlib/regex.fut:99:40-42
                
                int64_t tmp_22297 = add64((int64_t) 1, total_21759);
                
                loopres_21765 = tmp_22296;
                loopres_21766 = tmp_22297;
            } else {
                // stdlib/regex.fut:99:51-53
                
                int64_t tmp_21769 = add64((int64_t) 1, i_21758);
                
                loopres_21765 = tmp_21769;
                loopres_21766 = total_21759;
            }
            // stdlib/regex.fut:98:59-75
            
            bool loop_cond_21770 = slt64(loopres_21765, max_res_21700);
            bool loop_while_tmp_22874 = loop_cond_21770;
            int64_t i_tmp_22875 = loopres_21765;
            int64_t total_tmp_22876 = loopres_21766;
            
            loop_while_21757 = loop_while_tmp_22874;
            i_21758 = i_tmp_22875;
            total_21759 = total_tmp_22876;
        }
        count_res_f_res_21754 = loop_while_21757;
        count_res_f_res_21755 = i_21758;
        count_res_f_res_21756 = total_21759;
        count_res_21701 = count_res_f_res_21756;
    }
    if (memblock_unref(ctx, &mem_22584, "mem_22584") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_22586, "mem_22586") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_22588, "mem_22588") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_22590, "mem_22590") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &mem_22665, "mem_22665") != 0)
        return 1;
    prim_out_22832 = replace_res_21778;
    prim_out_22833 = count_res_21701;
    if (memblock_set(ctx, &*mem_out_p_22961, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    *out_prim_out_22962 = prim_out_22832;
    *out_prim_out_22963 = prim_out_22833;
    
  cleanup:
    {
        free(mem_22650);
        if (memblock_unref(ctx, &mem_22665, "mem_22665") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22605, "ext_mem_22605") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22604, "mem_22604") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22606, "mem_22606") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22607, "ext_mem_22607") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22624, "mem_22624") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22633, "ext_mem_22633") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22848, "mem_param_tmp_22848") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22852, "mem_param_tmp_22852") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22617, "ext_mem_22617") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22614, "mem_param_22614") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22620, "ext_mem_22620") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22611, "mem_param_22611") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22623, "ext_mem_22623") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22608, "mem_22608") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22636, "ext_mem_22636") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22590, "mem_22590") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22588, "mem_22588") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22586, "mem_22586") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22584, "mem_22584") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_wrap_test(struct futhark_context *ctx, struct memblock *mem_out_p_22965, int64_t *out_prim_out_22966, struct memblock text_mem_22579, int64_t dz2080U_18542, int64_t width_18543)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_22580;
    
    mem_22580.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22832;
    
    // stdlib/common.fut:22:17-20
    
    bool tmp_19099 = slt64((int64_t) 0, width_18543);
    
    // stdlib/common.fut:23:38-44
    
    int64_t zm_lhs_19100 = add64(dz2080U_18542, width_18543);
    
    // stdlib/common.fut:23:44-46
    
    int64_t zs_lhs_19101 = sub64(zm_lhs_19100, (int64_t) 1);
    
    // stdlib/common.fut:23:47-53
    
    bool zzero_19102 = width_18543 == (int64_t) 0;
    
    // stdlib/common.fut:23:47-53
    
    bool nonzzero_19103 = !zzero_19102;
    
    // stdlib/common.fut:23:47-53
    
    bool nonzzero_cert_19104;
    
    if (!nonzzero_19103) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/common.fut:23:47-53\n   #1  probes/stdlib-fast-tests.fut:29:43-58\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/common.fut:23:47-53
    
    int64_t zp_rhs_19105 = sdiv64(zs_lhs_19101, width_18543);
    
    // stdlib/common.fut:23:25-53
    
    int64_t sizze_19106 = add64(dz2080U_18542, zp_rhs_19105);
    
    // stdlib/common.fut:26:77-86
    
    bool bounds_invalid_upwards_19107 = slt64(sizze_19106, (int64_t) 0);
    
    // stdlib/common.fut:26:77-86
    
    bool valid_19108 = !bounds_invalid_upwards_19107;
    
    // stdlib/common.fut:26:77-86
    
    bool range_valid_c_19109;
    
    if (!valid_19108) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) (int64_t) 0, "..", (long long) (int64_t) 1, "..<", (long long) sizze_19106, " is invalid.", "-> #0  stdlib/common.fut:26:77-86\n   #1  probes/stdlib-fast-tests.fut:29:43-58\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/common.fut:22:3-26:87
    
    bool assert_c_19111;
    
    if (!tmp_19099) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "Assertion is false: (width > 0)", "-> #0  stdlib/common.fut:22:3-26:87\n   #1  probes/stdlib-fast-tests.fut:29:43-58\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/common.fut:25:21-23
    
    int64_t zs_rhs_19112 = add64((int64_t) 1, width_18543);
    
    // stdlib/common.fut:25:14-23
    
    bool zzero_19113 = zs_rhs_19112 == (int64_t) 0;
    
    // stdlib/common.fut:25:14-23
    
    bool nonzzero_19114 = !zzero_19113;
    
    // stdlib/common.fut:25:14-23
    
    bool nonzzero_cert_19115;
    
    if (!nonzzero_19114) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/common.fut:25:14-23\n   #1  probes/stdlib-fast-tests.fut:29:43-58\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    if (memblock_alloc(ctx, &mem_22580, sizze_19106, "mem_22580")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/common.fut:22:3-26:87
    for (int64_t i_22565 = 0; i_22565 < sizze_19106; i_22565++) {
        // stdlib/common.fut:25:14-23
        
        int64_t zt_lhs_19118 = sdiv64(i_22565, zs_rhs_19112);
        
        // stdlib/common.fut:25:24-30
        
        int64_t zp_lhs_19119 = mul64(width_18543, zt_lhs_19118);
        
        // stdlib/common.fut:25:32-41
        
        int64_t zp_rhs_19120 = smod64(i_22565, zs_rhs_19112);
        
        // stdlib/common.fut:25:30-41
        
        int64_t j_19121 = add64(zp_lhs_19119, zp_rhs_19120);
        
        // stdlib/common.fut:26:8-74
        
        bool cond_19122 = zp_rhs_19120 == width_18543;
        
        // stdlib/common.fut:26:37-51
        
        bool cond_f_res_19123 = sle64(dz2080U_18542, j_19121);
        
        // probes/stdlib-fast-tests.fut:29:43-58
        
        bool x_19124 = !cond_19122;
        
        // probes/stdlib-fast-tests.fut:29:43-58
        
        bool y_19125 = cond_f_res_19123 && x_19124;
        
        // probes/stdlib-fast-tests.fut:29:43-58
        
        bool cond_19126 = cond_19122 || y_19125;
        
        // stdlib/common.fut:26:8-74
        
        int8_t lifted_lambda_res_19127;
        
        if (cond_19126) {
            lifted_lambda_res_19127 = (int8_t) 10;
        } else {
            // stdlib/common.fut:26:67-74
            
            bool x_19128 = sle64((int64_t) 0, j_19121);
            
            // stdlib/common.fut:26:67-74
            
            bool y_19129 = slt64(j_19121, dz2080U_18542);
            
            // stdlib/common.fut:26:67-74
            
            bool bounds_check_19130 = x_19128 && y_19129;
            
            // stdlib/common.fut:26:67-74
            
            bool index_certs_19131;
            
            if (!bounds_check_19130) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) j_19121, "] out of bounds for array of shape [", (long long) dz2080U_18542, "].", "-> #0  stdlib/common.fut:26:67-74\n   #1  probes/stdlib-fast-tests.fut:29:43-58\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/common.fut:26:67-74
            
            int8_t lifted_lambda_res_f_res_19132 = ((int8_t *) text_mem_22579.mem)[j_19121];
            
            lifted_lambda_res_19127 = lifted_lambda_res_f_res_19132;
        }
        ((int8_t *) mem_22580.mem)[i_22565] = lifted_lambda_res_19127;
    }
    if (memblock_set(ctx, &mem_out_22831, &mem_22580, "mem_22580") != 0)
        return 1;
    prim_out_22832 = sizze_19106;
    if (memblock_set(ctx, &*mem_out_p_22965, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    *out_prim_out_22966 = prim_out_22832;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_22580, "mem_22580") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_find_11687(struct futhark_context *ctx, int64_t *out_prim_out_22967, int64_t *out_prim_out_22968, struct memblock z30U_mem_22579, struct memblock z31U_mem_22580, struct memblock z32U_mem_22581, struct memblock z33U_mem_22582, struct memblock text_mem_22583, int64_t dz2080U_16672, int64_t dz2081U_16673, int64_t dz2082U_16674, int64_t dz2083U_16675, int64_t dz2080U_16676, int64_t start_16682)
{
    (void) ctx;
    
    int err = 0;
    int64_t prim_out_22831;
    int64_t prim_out_22832;
    
    // stdlib/regex.fut:69:24-78:56
    
    bool cond_16688 = sle64(start_16682, dz2080U_16676);
    
    // stdlib/regex.fut:69:24-78:56
    
    bool find_res_16691;
    int64_t find_res_16692;
    int64_t find_res_16693;
    bool loop_while_16694;
    int64_t position_16695;
    int64_t end_16696;
    
    loop_while_16694 = cond_16688;
    position_16695 = start_16682;
    end_16696 = (int64_t) -1;
    while (loop_while_16694) {
        // stdlib/regex.fut:71:17-77:82
        
        int64_t end_16699;
        int64_t result_16701 = (int64_t) -1;
        
        for (int64_t branch_16700 = 0; branch_16700 < dz2082U_16674; branch_16700++) {
            // stdlib/regex.fut:72:9-77:82
            
            bool cond_16702 = sle64((int64_t) 0, result_16701);
            
            // stdlib/regex.fut:72:9-77:82
            
            int64_t loopres_16703;
            
            if (cond_16702) {
                loopres_16703 = result_16701;
            } else {
                // stdlib/regex.fut:74:25-37
                
                bool y_16711 = slt64(branch_16700, dz2083U_16675);
                
                // stdlib/regex.fut:74:25-37
                
                bool index_certs_16713;
                
                if (!y_16711) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) branch_16700, "] out of bounds for array of shape [", (long long) dz2083U_16675, "].", "-> #0  stdlib/regex.fut:74:25-37\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:73:26-38
                
                int64_t first_16708 = ((int64_t *) z32U_mem_22581.mem)[branch_16700];
                
                // stdlib/regex.fut:74:25-37
                
                int64_t last_16714 = ((int64_t *) z33U_mem_22582.mem)[branch_16700];
                
                // stdlib/regex.fut:75:29-76:81
                
                bool cond_16716 = first_16708 == last_16714;
                
                // stdlib/regex.fut:75:29-76:81
                
                bool cond_16717;
                
                if (cond_16716) {
                    cond_16717 = 1;
                } else {
                    // stdlib/regex.fut:75:46-57
                    
                    bool x_16718 = sle64((int64_t) 0, first_16708);
                    
                    // stdlib/regex.fut:75:46-57
                    
                    bool y_16719 = slt64(first_16708, dz2081U_16673);
                    
                    // stdlib/regex.fut:75:46-57
                    
                    bool bounds_check_16720 = x_16718 && y_16719;
                    
                    // stdlib/regex.fut:75:46-57
                    
                    bool index_certs_16721;
                    
                    if (!bounds_check_16720) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) first_16708, "] out of bounds for array of shape [", (long long) dz2081U_16673, "].", "-> #0  stdlib/regex.fut:75:46-57\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:75:46-57
                    
                    int64_t zeze_lhs_16722 = ((int64_t *) z31U_mem_22580.mem)[first_16708];
                    
                    // stdlib/regex.fut:75:58-62
                    
                    bool cond_f_res_16724 = zeze_lhs_16722 == (int64_t) 0;
                    
                    cond_16717 = cond_f_res_16724;
                }
                // stdlib/regex.fut:75:29-76:81
                
                bool cond_16725;
                
                if (cond_16717) {
                    cond_16725 = 1;
                } else {
                    // stdlib/regex.fut:75:66-77
                    
                    bool x_16726 = sle64((int64_t) 0, first_16708);
                    
                    // stdlib/regex.fut:75:66-77
                    
                    bool y_16727 = slt64(first_16708, dz2081U_16673);
                    
                    // stdlib/regex.fut:75:66-77
                    
                    bool bounds_check_16728 = x_16726 && y_16727;
                    
                    // stdlib/regex.fut:75:66-77
                    
                    bool index_certs_16729;
                    
                    if (!bounds_check_16728) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) first_16708, "] out of bounds for array of shape [", (long long) dz2081U_16673, "].", "-> #0  stdlib/regex.fut:75:66-77\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:75:66-77
                    
                    int64_t zeze_lhs_16730 = ((int64_t *) z31U_mem_22580.mem)[first_16708];
                    
                    // stdlib/regex.fut:75:78-82
                    
                    bool cond_f_res_16732 = zeze_lhs_16730 == (int64_t) 3;
                    
                    cond_16725 = cond_f_res_16732;
                }
                // stdlib/regex.fut:75:29-76:81
                
                bool possible_16733;
                
                if (cond_16725) {
                    possible_16733 = 1;
                } else {
                    // stdlib/regex.fut:76:17-80
                    
                    bool cond_16736 = slt64(position_16695, dz2080U_16676);
                    
                    // stdlib/regex.fut:76:17-80
                    
                    bool possible_f_res_16737;
                    
                    if (cond_16736) {
                        // stdlib/regex.fut:76:66-80
                        
                        bool x_21918 = sle64((int64_t) 0, position_16695);
                        
                        // stdlib/regex.fut:76:66-80
                        
                        bool bounds_check_21919 = cond_16736 && x_21918;
                        
                        // stdlib/regex.fut:76:66-80
                        
                        bool index_certs_21920;
                        
                        if (!bounds_check_21919) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) position_16695, "] out of bounds for array of shape [", (long long) dz2080U_16676, "].", "-> #0  stdlib/regex.fut:76:66-80\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:76:66-80
                        
                        int8_t token_accepts_arg2_21921 = ((int8_t *) text_mem_22583.mem)[position_16695];
                        
                        // stdlib/regex.fut:55:26-34
                        
                        int64_t u8_res_21922 = zext_i8_i64(token_accepts_arg2_21921);
                        
                        // stdlib/regex.fut:55:34-37
                        
                        int64_t zgzg_lhs_21923 = sdiv64(u8_res_21922, (int64_t) 64);
                        
                        // stdlib/regex.fut:55:15-38
                        
                        int64_t zgzg_lhs_21924 = ((int64_t *) z30U_mem_22579.mem)[first_16708 * (int64_t) 4 + zgzg_lhs_21923];
                        
                        // stdlib/regex.fut:55:51-54
                        
                        int8_t u8_arg0_21925 = umod8(token_accepts_arg2_21921, (int8_t) 64);
                        
                        // stdlib/regex.fut:55:42-54
                        
                        int64_t u8_res_21926 = zext_i8_i64(u8_arg0_21925);
                        
                        // stdlib/regex.fut:55:39-55
                        
                        int64_t za_lhs_21927 = lshr64(zgzg_lhs_21924, u8_res_21926);
                        
                        // stdlib/regex.fut:55:56-58
                        
                        int64_t znze_lhs_21928 = (int64_t) 1 & za_lhs_21927;
                        
                        // stdlib/regex.fut:55:60-64
                        
                        bool token_accepts_res_21929 = znze_lhs_21928 == (int64_t) 0;
                        
                        // stdlib/regex.fut:55:60-64
                        
                        bool token_accepts_res_21930 = !token_accepts_res_21929;
                        
                        possible_f_res_16737 = token_accepts_res_21930;
                    } else {
                        possible_f_res_16737 = 0;
                    }
                    possible_16733 = possible_f_res_16737;
                }
                // stdlib/regex.fut:77:17-82
                
                int64_t loopres_f_res_16746;
                
                if (possible_16733) {
                    // stdlib/regex.fut:68:11-77:74
                    
                    int64_t loopres_f_res_t_res_21931;
                    
                    if (futrts_match_branch_11689(ctx, &loopres_f_res_t_res_21931, z30U_mem_22579, z31U_mem_22580, z32U_mem_22581, z33U_mem_22582, text_mem_22583, dz2080U_16672, dz2081U_16673, dz2082U_16674, dz2083U_16675, dz2080U_16676, first_16708, last_16714, position_16695) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    loopres_f_res_16746 = loopres_f_res_t_res_21931;
                } else {
                    loopres_f_res_16746 = (int64_t) -1;
                }
                loopres_16703 = loopres_f_res_16746;
            }
            
            int64_t result_tmp_22836 = loopres_16703;
            
            result_16701 = result_tmp_22836;
        }
        end_16699 = result_16701;
        // stdlib/regex.fut:78:11-51
        
        bool cond_16750 = slt64(end_16699, (int64_t) 0);
        
        // stdlib/regex.fut:78:11-51
        
        int64_t tmp_16751;
        
        if (cond_16750) {
            // stdlib/regex.fut:78:35-37
            
            int64_t tmp_t_res_21932 = add64((int64_t) 1, position_16695);
            
            tmp_16751 = tmp_t_res_21932;
        } else {
            tmp_16751 = position_16695;
        }
        // stdlib/regex.fut:70:11-45
        
        bool cond_16758 = sle64(tmp_16751, dz2080U_16676);
        bool x_18550 = cond_16750 && cond_16758;
        bool loop_while_tmp_22833 = x_18550;
        int64_t position_tmp_22834 = tmp_16751;
        int64_t end_tmp_22835 = end_16699;
        
        loop_while_16694 = loop_while_tmp_22833;
        position_16695 = position_tmp_22834;
        end_16696 = end_tmp_22835;
    }
    find_res_16691 = loop_while_16694;
    find_res_16692 = position_16695;
    find_res_16693 = end_16696;
    prim_out_22831 = find_res_16692;
    prim_out_22832 = find_res_16693;
    *out_prim_out_22967 = prim_out_22831;
    *out_prim_out_22968 = prim_out_22832;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_grow_11710(struct futhark_context *ctx, struct memblock *mem_out_p_22969, struct memblock *mem_out_p_22970, struct memblock *mem_out_p_22971, int64_t *out_prim_out_22972, int64_t *out_prim_out_22973, struct memblock z30U_mem_22579, struct memblock z30U_mem_22580, struct memblock z30U_mem_22581, int64_t dz2080U_17759, int64_t z31U_17763)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_param_tmp_22838;
    
    mem_param_tmp_22838.references = NULL;
    
    struct memblock mem_param_tmp_22837;
    
    mem_param_tmp_22837.references = NULL;
    
    struct memblock mem_param_tmp_22836;
    
    mem_param_tmp_22836.references = NULL;
    
    struct memblock ext_mem_22600;
    
    ext_mem_22600.references = NULL;
    
    struct memblock ext_mem_22601;
    
    ext_mem_22601.references = NULL;
    
    struct memblock ext_mem_22602;
    
    ext_mem_22602.references = NULL;
    
    struct memblock ext_mem_22605;
    
    ext_mem_22605.references = NULL;
    
    struct memblock ext_mem_22608;
    
    ext_mem_22608.references = NULL;
    
    struct memblock ext_mem_22611;
    
    ext_mem_22611.references = NULL;
    
    struct memblock mem_param_22593;
    
    mem_param_22593.references = NULL;
    
    struct memblock mem_param_22590;
    
    mem_param_22590.references = NULL;
    
    struct memblock mem_param_22587;
    
    mem_param_22587.references = NULL;
    
    struct memblock ext_mem_22618;
    
    ext_mem_22618.references = NULL;
    
    struct memblock ext_mem_22619;
    
    ext_mem_22619.references = NULL;
    
    struct memblock ext_mem_22620;
    
    ext_mem_22620.references = NULL;
    
    struct memblock ext_mem_22582;
    
    ext_mem_22582.references = NULL;
    
    struct memblock ext_mem_22583;
    
    ext_mem_22583.references = NULL;
    
    struct memblock ext_mem_22584;
    
    ext_mem_22584.references = NULL;
    
    struct memblock mem_out_22833;
    
    mem_out_22833.references = NULL;
    
    struct memblock mem_out_22832;
    
    mem_out_22832.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22834;
    int64_t prim_out_22835;
    
    // stdlib/hashmap.fut:26:26-40
    
    int64_t dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_17769 = mul64((int64_t) 2, dz2080U_17759);
    
    // stdlib/hashmap.fut:26:19-40
    
    int64_t grow_res_17778;
    
    if (futrts_empty_8082(ctx, &ext_mem_22584, &ext_mem_22583, &ext_mem_22582, &grow_res_17778, dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_17769) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:26:6-27:73
    
    int64_t grow_res_17796;
    int64_t fresh_17801;
    
    if (memblock_set(ctx, &mem_param_22587, &ext_mem_22584, "ext_mem_22584") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_22590, &ext_mem_22583, "ext_mem_22583") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_22593, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    fresh_17801 = grow_res_17778;
    for (int64_t i_17797 = 0; i_17797 < dz2080U_17759; i_17797++) {
        // stdlib/hashmap.fut:27:11-19
        
        bool loopres_17808 = ((bool *) z30U_mem_22581.mem)[i_17797];
        
        // stdlib/hashmap.fut:27:52-60
        
        int64_t loopres_t_res_21918;
        
        if (loopres_17808) {
            int64_t x_22810 = ((int64_t *) z30U_mem_22580.mem)[i_17797];
            
            loopres_t_res_21918 = x_22810;
        } else {
            loopres_t_res_21918 = (int64_t) 0;
        }
        // stdlib/hashmap.fut:27:41-49
        
        int64_t loopres_t_res_21919;
        
        if (loopres_17808) {
            int64_t x_22812 = ((int64_t *) z30U_mem_22579.mem)[i_17797];
            
            loopres_t_res_21919 = x_22812;
        } else {
            loopres_t_res_21919 = (int64_t) 0;
        }
        
        int64_t loopres_17815;
        
        if (loopres_17808) {
            // stdlib/hashmap.fut:27:27-62
            
            int64_t loopres_t_res_21923;
            
            if (futrts_put_raw_11712(ctx, &ext_mem_22602, &ext_mem_22601, &ext_mem_22600, &loopres_t_res_21923, mem_param_22587, mem_param_22590, mem_param_22593, dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_17769, fresh_17801, loopres_t_res_21919, loopres_t_res_21918) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_22611, &ext_mem_22602, "ext_mem_22602") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22608, &ext_mem_22601, "ext_mem_22601") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22605, &ext_mem_22600, "ext_mem_22600") != 0)
                return 1;
            loopres_17815 = loopres_t_res_21923;
        } else {
            if (memblock_set(ctx, &ext_mem_22611, &mem_param_22587, "mem_param_22587") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22608, &mem_param_22590, "mem_param_22590") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_22605, &mem_param_22593, "mem_param_22593") != 0)
                return 1;
            loopres_17815 = fresh_17801;
        }
        if (memblock_set(ctx, &mem_param_tmp_22836, &ext_mem_22611, "ext_mem_22611") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22837, &ext_mem_22608, "ext_mem_22608") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_22838, &ext_mem_22605, "ext_mem_22605") != 0)
            return 1;
        
        int64_t fresh_tmp_22842 = loopres_17815;
        
        if (memblock_set(ctx, &mem_param_22587, &mem_param_tmp_22836, "mem_param_tmp_22836") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22590, &mem_param_tmp_22837, "mem_param_tmp_22837") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_22593, &mem_param_tmp_22838, "mem_param_tmp_22838") != 0)
            return 1;
        fresh_17801 = fresh_tmp_22842;
    }
    if (memblock_set(ctx, &ext_mem_22620, &mem_param_22587, "mem_param_22587") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_22619, &mem_param_22590, "mem_param_22590") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_22618, &mem_param_22593, "mem_param_22593") != 0)
        return 1;
    grow_res_17796 = fresh_17801;
    if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22583, "ext_mem_22583") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_22584, "ext_mem_22584") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &ext_mem_22620, "ext_mem_22620") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22832, &ext_mem_22619, "ext_mem_22619") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22833, &ext_mem_22618, "ext_mem_22618") != 0)
        return 1;
    prim_out_22834 = dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_17769;
    prim_out_22835 = grow_res_17796;
    if (memblock_set(ctx, &*mem_out_p_22969, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22970, &mem_out_22832, "mem_out_22832") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22971, &mem_out_22833, "mem_out_22833") != 0)
        return 1;
    *out_prim_out_22972 = prim_out_22834;
    *out_prim_out_22973 = prim_out_22835;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_param_tmp_22838, "mem_param_tmp_22838") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22837, "mem_param_tmp_22837") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_22836, "mem_param_tmp_22836") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22600, "ext_mem_22600") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22601, "ext_mem_22601") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22602, "ext_mem_22602") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22605, "ext_mem_22605") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22608, "ext_mem_22608") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22611, "ext_mem_22611") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22593, "mem_param_22593") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22590, "mem_param_22590") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_22587, "mem_param_22587") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22618, "ext_mem_22618") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22619, "ext_mem_22619") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22620, "ext_mem_22620") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22582, "ext_mem_22582") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22583, "ext_mem_22583") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_22584, "ext_mem_22584") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22833, "mem_out_22833") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22832, "mem_out_22832") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_integer_6855(struct futhark_context *ctx, struct memblock *mem_out_p_22974, int64_t *out_prim_out_22975, bool *out_prim_out_22976, int64_t x_15288)
{
    (void) ctx;
    
    int err = 0;
    struct memblock ext_mem_22580;
    
    ext_mem_22580.references = NULL;
    
    struct memblock mem_22579;
    
    mem_22579.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22832;
    bool prim_out_22833;
    
    // stdlib/bigint-fast.fut:9:28-51
    
    bool cond_15289 = slt64(x_15288, (int64_t) 0);
    
    // stdlib/bigint-fast.fut:9:28-51
    
    int64_t i64_arg0_15290;
    
    if (cond_15289) {
        // stdlib/bigint-fast.fut:9:42-44
        
        int64_t i64_arg0_t_res_21918 = -x_15288;
        
        i64_arg0_15290 = i64_arg0_t_res_21918;
    } else {
        i64_arg0_15290 = x_15288;
    }
    // stdlib/bigint-fast.fut:10:25-42
    
    int32_t u64_res_18733 = zext_i64_i32(i64_arg0_15290);
    
    // stdlib/bigint-fast.fut:10:62-67
    
    int64_t u64_arg0_15297 = lshr64(i64_arg0_15290, (int64_t) 32);
    
    // stdlib/bigint-fast.fut:10:43-67
    
    int32_t u64_res_18735 = zext_i64_i32(u64_arg0_15297);
    
    if (memblock_alloc(ctx, &mem_22579, (int64_t) 8, "mem_22579")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:10:6-69
    ((int32_t *) mem_22579.mem)[(int64_t) 0] = u64_res_18733;
    ((int32_t *) mem_22579.mem)[(int64_t) 1] = u64_res_18735;
    // stdlib/bigint-fast.fut:10:6-69
    
    int64_t integer_res_15304;
    bool integer_res_15305;
    
    if (futrts_normalise_11622(ctx, &ext_mem_22580, &integer_res_15304, &integer_res_15305, mem_22579, (int64_t) 2, cond_15289) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &mem_22579, "mem_22579") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &ext_mem_22580, "ext_mem_22580") != 0)
        return 1;
    prim_out_22832 = integer_res_15304;
    prim_out_22833 = integer_res_15305;
    if (memblock_set(ctx, &*mem_out_p_22974, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    *out_prim_out_22975 = prim_out_22832;
    *out_prim_out_22976 = prim_out_22833;
    
  cleanup:
    {
        if (memblock_unref(ctx, &ext_mem_22580, "ext_mem_22580") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22579, "mem_22579") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_literal_11671(struct futhark_context *ctx, int8_t *out_prim_out_22977, int64_t *out_prim_out_22978, struct memblock pattern_mem_22579, int64_t dz2080U_15985, int64_t p_15987)
{
    (void) ctx;
    
    int err = 0;
    int8_t prim_out_22831;
    int64_t prim_out_22832;
    
    // stdlib/regex.fut:8:6-16
    
    bool x_15988 = sle64((int64_t) 0, p_15987);
    
    // stdlib/regex.fut:8:6-16
    
    bool y_15989 = slt64(p_15987, dz2080U_15985);
    
    // stdlib/regex.fut:8:6-16
    
    bool bounds_check_15990 = x_15988 && y_15989;
    
    // stdlib/regex.fut:8:6-16
    
    bool index_certs_15991;
    
    if (!bounds_check_15990) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) p_15987, "] out of bounds for array of shape [", (long long) dz2080U_15985, "].", "-> #0  stdlib/regex.fut:8:6-16\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:8:6-16
    
    int8_t zeze_lhs_15992 = ((int8_t *) pattern_mem_22579.mem)[p_15987];
    
    // stdlib/regex.fut:8:3-9:24
    
    bool cond_15994 = zeze_lhs_15992 == (int8_t) 92;
    
    // stdlib/regex.fut:8:27-29
    
    int64_t zl_lhs_15996 = add64((int64_t) 1, p_15987);
    
    // stdlib/regex.fut:8:30-46
    
    bool cond_t_res_16000 = slt64(zl_lhs_15996, dz2080U_15985);
    bool x_21853 = cond_15994 && cond_t_res_16000;
    
    // stdlib/regex.fut:8:3-9:24
    
    int8_t literal_res_16001;
    int64_t literal_res_16002;
    
    if (x_21853) {
        // stdlib/regex.fut:8:61-73
        
        bool x_21918 = sle64((int64_t) 0, zl_lhs_15996);
        
        // stdlib/regex.fut:8:61-73
        
        bool bounds_check_21919 = cond_t_res_16000 && x_21918;
        
        // stdlib/regex.fut:8:61-73
        
        bool index_certs_21920;
        
        if (!bounds_check_21919) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_15996, "] out of bounds for array of shape [", (long long) dz2080U_15985, "].", "-> #0  stdlib/regex.fut:8:61-73\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:8:61-73
        
        int8_t escaped_arg0_21921 = ((int8_t *) pattern_mem_22579.mem)[zl_lhs_15996];
        
        // stdlib/regex.fut:6:25-100
        
        bool cond_21922 = escaped_arg0_21921 == (int8_t) 110;
        
        // stdlib/regex.fut:6:25-100
        
        int8_t escaped_res_21923;
        
        if (cond_21922) {
            escaped_res_21923 = (int8_t) 10;
        } else {
            // stdlib/regex.fut:6:50-100
            
            bool cond_21924 = escaped_arg0_21921 == (int8_t) 114;
            
            // stdlib/regex.fut:6:50-100
            
            int8_t escaped_res_f_res_21925;
            
            if (cond_21924) {
                escaped_res_f_res_21925 = (int8_t) 13;
            } else {
                // stdlib/regex.fut:6:75-100
                
                bool cond_21926 = escaped_arg0_21921 == (int8_t) 116;
                
                // stdlib/regex.fut:6:75-100
                
                int8_t escaped_res_f_res_f_res_21927;
                
                if (cond_21926) {
                    escaped_res_f_res_f_res_21927 = (int8_t) 9;
                } else {
                    escaped_res_f_res_f_res_21927 = escaped_arg0_21921;
                }
                escaped_res_f_res_21925 = escaped_res_f_res_f_res_21927;
            }
            escaped_res_21923 = escaped_res_f_res_21925;
        }
        // stdlib/regex.fut:8:75-77
        
        int64_t tmp_21928 = add64((int64_t) 2, p_15987);
        
        literal_res_16001 = escaped_res_21923;
        literal_res_16002 = tmp_21928;
    } else {
        literal_res_16001 = zeze_lhs_15992;
        literal_res_16002 = zl_lhs_15996;
    }
    prim_out_22831 = literal_res_16001;
    prim_out_22832 = literal_res_16002;
    *out_prim_out_22977 = prim_out_22831;
    *out_prim_out_22978 = prim_out_22832;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_mask_set_8529(struct futhark_context *ctx, struct memblock *mem_out_p_22979, struct memblock mask_mem_22579, int8_t c_16038)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    // stdlib/regex.fut:11:14-22
    
    int64_t u8_res_18608 = zext_i8_i64(c_16038);
    
    // stdlib/regex.fut:11:22-25
    
    int64_t word_16044 = sdiv64(u8_res_18608, (int64_t) 64);
    
    // stdlib/regex.fut:12:20-30
    
    bool x_16046 = sle64((int64_t) 0, word_16044);
    
    // stdlib/regex.fut:12:20-30
    
    bool y_16047 = slt64(word_16044, (int64_t) 4);
    
    // stdlib/regex.fut:12:20-30
    
    bool bounds_check_16048 = x_16046 && y_16047;
    
    // stdlib/regex.fut:12:20-30
    
    bool index_certs_16049;
    
    if (!bounds_check_16048) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) word_16044, "] out of bounds for array of shape [", (long long) (int64_t) 4, "].", "-> #0  stdlib/regex.fut:12:20-30\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:12:20-30
    
    int64_t zb_lhs_16050 = ((int64_t *) mask_mem_22579.mem)[word_16044];
    
    // stdlib/regex.fut:12:51-54
    
    int8_t u8_arg0_16055 = umod8(c_16038, (int8_t) 64);
    
    // stdlib/regex.fut:12:42-54
    
    int64_t u8_res_18732 = zext_i8_i64(u8_arg0_16055);
    
    // stdlib/regex.fut:12:39-55
    
    int64_t zb_rhs_16059 = shl64((int64_t) 1, u8_res_18732);
    
    // stdlib/regex.fut:12:31-55
    
    int64_t tmp_16061 = zb_lhs_16050 | zb_rhs_16059;
    
    // stdlib/regex.fut:12:7-56
    ((int64_t *) mask_mem_22579.mem)[word_16044] = tmp_16061;
    if (memblock_set(ctx, &mem_out_22831, &mask_mem_22579, "mask_mem_22579") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22979, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_match_branch_11689(struct futhark_context *ctx, int64_t *out_prim_out_22980, struct memblock z30U_mem_22579, struct memblock z31U_mem_22580, struct memblock z32U_mem_22581, struct memblock z33U_mem_22582, struct memblock text_mem_22583, int64_t dz2080U_16580, int64_t dz2081U_16581, int64_t dz2082U_16582, int64_t dz2083U_16583, int64_t dz2080U_16584, int64_t token_16589, int64_t last_16590, int64_t position_16592)
{
    (void) ctx;
    
    int err = 0;
    int64_t prim_out_22831;
    
    // stdlib/regex.fut:57:3-67:14
    
    bool cond_16593 = token_16589 == last_16590;
    
    // stdlib/regex.fut:57:3-67:14
    
    int64_t match_branch_res_16594;
    
    if (cond_16593) {
        match_branch_res_16594 = position_16592;
    } else {
        // stdlib/regex.fut:59:16-27
        
        bool x_16595 = sle64((int64_t) 0, token_16589);
        
        // stdlib/regex.fut:59:16-27
        
        bool y_16596 = slt64(token_16589, dz2081U_16581);
        
        // stdlib/regex.fut:59:16-27
        
        bool bounds_check_16597 = x_16595 && y_16596;
        
        // stdlib/regex.fut:59:16-27
        
        bool index_certs_16598;
        
        if (!bounds_check_16597) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) token_16589, "] out of bounds for array of shape [", (long long) dz2081U_16581, "].", "-> #0  stdlib/regex.fut:59:16-27\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:59:16-27
        
        int64_t kind_16599 = ((int64_t *) z31U_mem_22580.mem)[token_16589];
        
        // stdlib/regex.fut:60:19-58
        
        bool cond_16601 = kind_16599 == (int64_t) 0;
        
        // stdlib/regex.fut:60:40-44
        
        bool cond_f_res_16603 = kind_16599 == (int64_t) 3;
        bool x_18548 = !cond_16601;
        bool y_18549 = cond_f_res_16603 && x_18548;
        bool cond_16602 = cond_16601 || y_18549;
        bool cond_neg_18550 = !cond_16602;
        int64_t minimum_16604 = btoi_bool_i64(cond_neg_18550);
        
        // stdlib/regex.fut:61:17-99
        
        bool cond_16606 = kind_16599 == (int64_t) 1;
        bool x_18552 = !cond_16606;
        bool y_18553 = cond_f_res_16603 && x_18552;
        bool cond_16607 = cond_16606 || y_18553;
        
        // stdlib/regex.fut:61:17-99
        
        int64_t limit_16609;
        
        if (cond_16607) {
            // stdlib/regex.fut:61:65-67
            
            int64_t min_arg0_21918 = add64((int64_t) 1, position_16592);
            
            // stdlib/regex.fut:61:48-81
            
            int64_t min_res_21919 = smin64(dz2080U_16584, min_arg0_21918);
            
            limit_16609 = min_res_21919;
        } else {
            limit_16609 = dz2080U_16584;
        }
        // stdlib/regex.fut:62:15-97
        
        bool cond_16618 = slt64(position_16592, limit_16609);
        
        // stdlib/regex.fut:62:15-97
        
        bool loop_cond_16619;
        
        if (cond_16618) {
            // stdlib/regex.fut:62:79-88
            
            bool x_21920 = sle64((int64_t) 0, position_16592);
            
            // stdlib/regex.fut:62:79-88
            
            bool y_21921 = slt64(position_16592, dz2080U_16584);
            
            // stdlib/regex.fut:62:79-88
            
            bool bounds_check_21922 = x_21920 && y_21921;
            
            // stdlib/regex.fut:62:79-88
            
            bool index_certs_21923;
            
            if (!bounds_check_21922) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) position_16592, "] out of bounds for array of shape [", (long long) dz2080U_16584, "].", "-> #0  stdlib/regex.fut:62:79-88\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:62:79-88
            
            int8_t token_accepts_arg2_21924 = ((int8_t *) text_mem_22583.mem)[position_16592];
            
            // stdlib/regex.fut:55:26-34
            
            int64_t u8_res_21925 = zext_i8_i64(token_accepts_arg2_21924);
            
            // stdlib/regex.fut:55:34-37
            
            int64_t zgzg_lhs_21926 = sdiv64(u8_res_21925, (int64_t) 64);
            
            // stdlib/regex.fut:55:15-38
            
            int64_t zgzg_lhs_21927 = ((int64_t *) z30U_mem_22579.mem)[token_16589 * (int64_t) 4 + zgzg_lhs_21926];
            
            // stdlib/regex.fut:55:51-54
            
            int8_t u8_arg0_21928 = umod8(token_accepts_arg2_21924, (int8_t) 64);
            
            // stdlib/regex.fut:55:42-54
            
            int64_t u8_res_21929 = zext_i8_i64(u8_arg0_21928);
            
            // stdlib/regex.fut:55:39-55
            
            int64_t za_lhs_21930 = lshr64(zgzg_lhs_21927, u8_res_21929);
            
            // stdlib/regex.fut:55:56-58
            
            int64_t znze_lhs_21931 = (int64_t) 1 & za_lhs_21930;
            
            // stdlib/regex.fut:55:60-64
            
            bool token_accepts_res_21932 = znze_lhs_21931 == (int64_t) 0;
            
            // stdlib/regex.fut:55:60-64
            
            bool token_accepts_res_21933 = !token_accepts_res_21932;
            
            loop_cond_16619 = token_accepts_res_21933;
        } else {
            loop_cond_16619 = 0;
        }
        // stdlib/regex.fut:62:15-97
        
        bool end_16627;
        int64_t end_16628;
        bool loop_while_16629;
        int64_t end_16630;
        
        loop_while_16629 = loop_cond_16619;
        end_16630 = position_16592;
        while (loop_while_16629) {
            // stdlib/regex.fut:62:95-97
            
            int64_t loopres_16631 = add64((int64_t) 1, end_16630);
            
            // stdlib/regex.fut:62:41-88
            
            bool cond_16633 = slt64(loopres_16631, limit_16609);
            
            // stdlib/regex.fut:62:41-88
            
            bool loop_cond_16634;
            
            if (cond_16633) {
                // stdlib/regex.fut:62:79-88
                
                bool x_21934 = sle64((int64_t) 0, loopres_16631);
                
                // stdlib/regex.fut:62:79-88
                
                bool y_21935 = slt64(loopres_16631, dz2080U_16584);
                
                // stdlib/regex.fut:62:79-88
                
                bool bounds_check_21936 = x_21934 && y_21935;
                
                // stdlib/regex.fut:62:79-88
                
                bool index_certs_21937;
                
                if (!bounds_check_21936) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_16631, "] out of bounds for array of shape [", (long long) dz2080U_16584, "].", "-> #0  stdlib/regex.fut:62:79-88\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:62:79-88
                
                int8_t token_accepts_arg2_21938 = ((int8_t *) text_mem_22583.mem)[loopres_16631];
                
                // stdlib/regex.fut:55:26-34
                
                int64_t u8_res_21939 = zext_i8_i64(token_accepts_arg2_21938);
                
                // stdlib/regex.fut:55:34-37
                
                int64_t zgzg_lhs_21940 = sdiv64(u8_res_21939, (int64_t) 64);
                
                // stdlib/regex.fut:55:15-38
                
                int64_t zgzg_lhs_21941 = ((int64_t *) z30U_mem_22579.mem)[token_16589 * (int64_t) 4 + zgzg_lhs_21940];
                
                // stdlib/regex.fut:55:51-54
                
                int8_t u8_arg0_21942 = umod8(token_accepts_arg2_21938, (int8_t) 64);
                
                // stdlib/regex.fut:55:42-54
                
                int64_t u8_res_21943 = zext_i8_i64(u8_arg0_21942);
                
                // stdlib/regex.fut:55:39-55
                
                int64_t za_lhs_21944 = lshr64(zgzg_lhs_21941, u8_res_21943);
                
                // stdlib/regex.fut:55:56-58
                
                int64_t znze_lhs_21945 = (int64_t) 1 & za_lhs_21944;
                
                // stdlib/regex.fut:55:60-64
                
                bool token_accepts_res_21946 = znze_lhs_21945 == (int64_t) 0;
                
                // stdlib/regex.fut:55:60-64
                
                bool token_accepts_res_21947 = !token_accepts_res_21946;
                
                loop_cond_16634 = token_accepts_res_21947;
            } else {
                loop_cond_16634 = 0;
            }
            
            bool loop_while_tmp_22832 = loop_cond_16634;
            int64_t end_tmp_22833 = loopres_16631;
            
            loop_while_16629 = loop_while_tmp_22832;
            end_16630 = end_tmp_22833;
        }
        end_16627 = loop_while_16629;
        end_16628 = end_16630;
        // stdlib/regex.fut:63:27-66:28
        
        int64_t zgze_rhs_16646 = add64(position_16592, minimum_16604);
        
        // stdlib/regex.fut:63:27-66:28
        
        bool cond_16648 = sle64(zgze_rhs_16646, end_16628);
        
        // stdlib/regex.fut:65:44-46
        
        int64_t match_branch_arg1_16657 = add64((int64_t) 1, token_16589);
        
        // stdlib/regex.fut:63:27-66:28
        
        bool match_branch_res_f_res_16651;
        int64_t match_branch_res_f_res_16652;
        int64_t match_branch_res_f_res_16653;
        bool loop_while_16654;
        int64_t trial_16655;
        int64_t result_16656;
        
        loop_while_16654 = cond_16648;
        trial_16655 = end_16628;
        result_16656 = (int64_t) -1;
        while (loop_while_16654) {
            // stdlib/regex.fut:56:19-65:63
            
            int64_t result_16659;
            
            if (futrts_match_branch_11689(ctx, &result_16659, z30U_mem_22579, z31U_mem_22580, z32U_mem_22581, z33U_mem_22582, text_mem_22583, dz2080U_16580, dz2081U_16581, dz2082U_16582, dz2083U_16583, dz2080U_16584, match_branch_arg1_16657, last_16590, trial_16655) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/regex.fut:66:18-20
            
            int64_t tmp_16661 = sub64(trial_16655, (int64_t) 1);
            
            // stdlib/regex.fut:64:13-52
            
            bool cond_16667 = sle64(zgze_rhs_16646, tmp_16661);
            
            // stdlib/regex.fut:64:49-52
            
            bool loop_cond_t_res_16669 = slt64(result_16659, (int64_t) 0);
            bool x_18557 = cond_16667 && loop_cond_t_res_16669;
            bool loop_while_tmp_22834 = x_18557;
            int64_t trial_tmp_22835 = tmp_16661;
            int64_t result_tmp_22836 = result_16659;
            
            loop_while_16654 = loop_while_tmp_22834;
            trial_16655 = trial_tmp_22835;
            result_16656 = result_tmp_22836;
        }
        match_branch_res_f_res_16651 = loop_while_16654;
        match_branch_res_f_res_16652 = trial_16655;
        match_branch_res_f_res_16653 = result_16656;
        match_branch_res_16594 = match_branch_res_f_res_16653;
    }
    prim_out_22831 = match_branch_res_16594;
    *out_prim_out_22980 = prim_out_22831;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_mul_abs_11639(struct futhark_context *ctx, struct memblock *mem_out_p_22981, int64_t *out_prim_out_22982, struct memblock a_mem_22579, int64_t dz2080U_15143, int32_t b_15145)
{
    (void) ctx;
    
    int err = 0;
    struct memblock ext_mem_22590;
    
    ext_mem_22590.references = NULL;
    
    struct memblock mem_22581;
    
    mem_22581.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22832;
    
    // stdlib/bigint-fast.fut:44:62-72
    
    int64_t dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_15147 = add64((int64_t) 1, dz2080U_15143);
    int64_t bytes_22580 = (int64_t) 4 * dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_15147;
    
    // stdlib/bigint-fast.fut:48:7-36
    
    bool y_15192 = slt64(dz2080U_15143, dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_15147);
    
    // stdlib/bigint-fast.fut:48:7-36
    
    bool index_certs_15194;
    
    if (!y_15192) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) dz2080U_15143, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_15147, "].", "-> #0  stdlib/bigint-fast.fut:48:7-36\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    if (memblock_alloc(ctx, &mem_22581, bytes_22580, "mem_22581")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:44:31-47:27
    for (int64_t nest_i_22833 = 0; nest_i_22833 < dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_15147; nest_i_22833++) {
        ((int32_t *) mem_22581.mem)[nest_i_22833] = 0;
    }
    
    int64_t u32_res_18732 = zext_i32_i64(b_15145);
    
    // stdlib/bigint-fast.fut:44:31-47:27
    
    int64_t mul_abs_res_15159;
    int64_t carry_15162 = (int64_t) 0;
    
    for (int64_t i_15160 = 0; i_15160 < dz2080U_15143; i_15160++) {
        // stdlib/bigint-fast.fut:45:27-31
        
        int32_t u32_arg0_15166 = ((int32_t *) a_mem_22579.mem)[i_15160];
        
        // stdlib/bigint-fast.fut:45:19-31
        
        int64_t u32_res_18734 = zext_i32_i64(u32_arg0_15166);
        
        // stdlib/bigint-fast.fut:45:31-41
        
        int64_t zp_lhs_15172 = mul64(u32_res_18732, u32_res_18734);
        
        // stdlib/bigint-fast.fut:45:41-47
        
        int64_t product_15174 = add64(carry_15162, zp_lhs_15172);
        
        // stdlib/bigint-fast.fut:46:18-33
        
        int32_t u64_res_18736 = zext_i64_i32(product_15174);
        
        // stdlib/bigint-fast.fut:46:9-33
        ((int32_t *) mem_22581.mem)[i_15160] = u64_res_18736;
        // stdlib/bigint-fast.fut:47:21-26
        
        int64_t tmp_15183 = lshr64(product_15174, (int64_t) 32);
        int64_t carry_tmp_22835 = tmp_15183;
        
        carry_15162 = carry_tmp_22835;
    }
    mul_abs_res_15159 = carry_15162;
    // stdlib/bigint-fast.fut:48:23-36
    
    int32_t u64_res_18738 = zext_i64_i32(mul_abs_res_15159);
    
    // stdlib/bigint-fast.fut:48:7-36
    ((int32_t *) mem_22581.mem)[dz2080U_15143] = u64_res_18738;
    // stdlib/bigint-fast.fut:49:7-26
    
    int64_t mul_abs_res_15197;
    bool mul_abs_res_15198;
    
    if (futrts_normalise_11622(ctx, &ext_mem_22590, &mul_abs_res_15197, &mul_abs_res_15198, mem_22581, dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_15147, 0) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &mem_22581, "mem_22581") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22831, &ext_mem_22590, "ext_mem_22590") != 0)
        return 1;
    prim_out_22832 = mul_abs_res_15197;
    if (memblock_set(ctx, &*mem_out_p_22981, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    *out_prim_out_22982 = prim_out_22832;
    
  cleanup:
    {
        if (memblock_unref(ctx, &ext_mem_22590, "ext_mem_22590") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_22581, "mem_22581") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_normalise_11622(struct futhark_context *ctx, struct memblock *mem_out_p_22983, int64_t *out_prim_out_22984, bool *out_prim_out_22985, struct memblock a_mem_22579, int64_t dz2080U_14751, bool negative_14752)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22832;
    bool prim_out_22833;
    
    // stdlib/bigint-fast.fut:5:24-75
    
    bool cond_14757 = slt64((int64_t) 1, dz2080U_14751);
    
    // stdlib/bigint-fast.fut:5:60-62
    
    int64_t zeze_lhs_14759 = sub64(dz2080U_14751, (int64_t) 1);
    
    // stdlib/bigint-fast.fut:5:57-63
    
    int32_t zeze_lhs_14764;
    
    if (cond_14757) {
        int32_t x_21918 = ((int32_t *) a_mem_22579.mem)[zeze_lhs_14759];
        
        zeze_lhs_14764 = x_21918;
    } else {
        zeze_lhs_14764 = 0;
    }
    // stdlib/bigint-fast.fut:5:64-68
    
    bool loop_cond_t_res_14766 = zeze_lhs_14764 == 0;
    bool x_18551 = cond_14757 && loop_cond_t_res_14766;
    
    // stdlib/bigint-fast.fut:5:24-75
    
    bool used_14767;
    int64_t used_14768;
    bool loop_while_14769;
    int64_t k_14770;
    
    loop_while_14769 = x_18551;
    k_14770 = dz2080U_14751;
    while (loop_while_14769) {
        // stdlib/bigint-fast.fut:5:73-75
        
        int64_t loopres_14771 = sub64(k_14770, (int64_t) 1);
        
        // stdlib/bigint-fast.fut:5:48-68
        
        bool cond_14773 = slt64((int64_t) 1, loopres_14771);
        
        // stdlib/bigint-fast.fut:5:48-68
        
        bool loop_cond_14774;
        
        if (cond_14773) {
            // stdlib/bigint-fast.fut:5:60-62
            
            int64_t zeze_lhs_21919 = sub64(loopres_14771, (int64_t) 1);
            
            // stdlib/bigint-fast.fut:5:57-63
            
            int32_t zeze_lhs_21920 = ((int32_t *) a_mem_22579.mem)[zeze_lhs_21919];
            
            // stdlib/bigint-fast.fut:5:64-68
            
            bool loop_cond_t_res_21921 = zeze_lhs_21920 == 0;
            
            loop_cond_14774 = loop_cond_t_res_21921;
        } else {
            loop_cond_14774 = 0;
        }
        
        bool loop_while_tmp_22834 = loop_cond_14774;
        int64_t k_tmp_22835 = loopres_14771;
        
        loop_while_14769 = loop_while_tmp_22834;
        k_14770 = k_tmp_22835;
    }
    used_14767 = loop_while_14769;
    used_14768 = k_14770;
    // stdlib/bigint-fast.fut:6:15-26
    
    bool empty_slice_18612 = used_14768 == (int64_t) 0;
    
    // stdlib/bigint-fast.fut:6:15-26
    
    int64_t m_18613 = sub64(used_14768, (int64_t) 1);
    
    // stdlib/bigint-fast.fut:6:15-26
    
    bool zzero_leq_i_p_m_t_s_18614 = sle64((int64_t) 0, m_18613);
    
    // stdlib/bigint-fast.fut:6:15-26
    
    bool i_p_m_t_s_leq_w_18615 = slt64(m_18613, dz2080U_14751);
    
    // stdlib/bigint-fast.fut:6:15-26
    
    bool i_lte_j_18616 = sle64((int64_t) 0, used_14768);
    
    // stdlib/bigint-fast.fut:6:15-26
    
    bool y_18617 = zzero_leq_i_p_m_t_s_18614 && i_p_m_t_s_leq_w_18615;
    
    // stdlib/bigint-fast.fut:6:15-26
    
    bool forwards_ok_18618 = i_lte_j_18616 && y_18617;
    
    // stdlib/bigint-fast.fut:6:15-26
    
    bool ok_or_empty_18619 = empty_slice_18612 || forwards_ok_18618;
    
    // stdlib/bigint-fast.fut:6:15-26
    
    bool index_certs_18620;
    
    if (!ok_or_empty_18619) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) used_14768, "] out of bounds for array of shape [", (long long) dz2080U_14751, "].", "-> #0  stdlib/bigint-fast.fut:6:15-26\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint-fast.fut:7:7-48
    
    bool tmp_14786;
    
    if (negative_14752) {
        // stdlib/bigint-fast.fut:7:21-47
        
        bool cond_21926 = used_14768 == (int64_t) 1;
        
        // stdlib/bigint-fast.fut:7:21-47
        
        bool tmp_21927;
        
        if (cond_21926) {
            // stdlib/bigint-fast.fut:7:34-42
            
            bool y_21928 = slt64((int64_t) 0, used_14768);
            
            // stdlib/bigint-fast.fut:7:34-42
            
            bool index_certs_21929;
            
            if (!y_21928) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) used_14768, "].", "-> #0  stdlib/bigint-fast.fut:7:34-42\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            
            int32_t zeze_lhs_21930 = ((int32_t *) a_mem_22579.mem)[(int64_t) 0];
            
            // stdlib/bigint-fast.fut:7:43-47
            
            bool tmp_t_res_21931 = zeze_lhs_21930 == 0;
            
            tmp_21927 = tmp_t_res_21931;
        } else {
            tmp_21927 = 0;
        }
        // stdlib/bigint-fast.fut:7:19-48
        
        bool tmp_t_res_21932 = !tmp_21927;
        
        tmp_14786 = tmp_t_res_21932;
    } else {
        tmp_14786 = 0;
    }
    if (memblock_set(ctx, &mem_out_22831, &a_mem_22579, "a_mem_22579") != 0)
        return 1;
    prim_out_22832 = used_14768;
    prim_out_22833 = tmp_14786;
    if (memblock_set(ctx, &*mem_out_p_22983, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    *out_prim_out_22984 = prim_out_22832;
    *out_prim_out_22985 = prim_out_22833;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_put_raw_11712(struct futhark_context *ctx, struct memblock *mem_out_p_22986, struct memblock *mem_out_p_22987, struct memblock *mem_out_p_22988, int64_t *out_prim_out_22989, struct memblock z30U_mem_22579, struct memblock z30U_mem_22580, struct memblock z30U_mem_22581, int64_t dz2080U_17722, int64_t z31U_17726, int64_t key_17727, int64_t value_17728)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_22833;
    
    mem_out_22833.references = NULL;
    
    struct memblock mem_out_22832;
    
    mem_out_22832.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    int64_t prim_out_22834;
    
    // stdlib/hashmap.fut:11:26-28
    
    int64_t mask_21861 = sub64(dz2080U_17722, (int64_t) 1);
    
    // stdlib/hashmap.fut:7:19-24
    
    int64_t zc_rhs_21862 = lshr64(key_17727, (int64_t) 30);
    
    // stdlib/hashmap.fut:7:14-24
    
    int64_t zt_lhs_21863 = key_17727 ^ zc_rhs_21862;
    
    // stdlib/hashmap.fut:7:26-48
    
    int64_t x_21864 = mul64((int64_t) -4658895280553007687, zt_lhs_21863);
    
    // stdlib/hashmap.fut:8:19-24
    
    int64_t zc_rhs_21865 = lshr64(x_21864, (int64_t) 27);
    
    // stdlib/hashmap.fut:8:14-24
    
    int64_t zt_lhs_21866 = x_21864 ^ zc_rhs_21865;
    
    // stdlib/hashmap.fut:8:26-48
    
    int64_t x_21867 = mul64((int64_t) -7723592293110705685, zt_lhs_21866);
    
    // stdlib/hashmap.fut:9:13-18
    
    int64_t zc_rhs_21868 = lshr64(x_21867, (int64_t) 31);
    
    // stdlib/hashmap.fut:9:8-18
    
    int64_t hash_res_21869 = x_21867 ^ zc_rhs_21868;
    
    // stdlib/hashmap.fut:12:43-57
    
    int64_t u64_arg0_21870 = mask_21861 & hash_res_21869;
    
    // stdlib/hashmap.fut:12:16-13:57
    
    bool loop_cond_21871 = ((bool *) z30U_mem_22581.mem)[u64_arg0_21870];
    
    // stdlib/hashmap.fut:12:16-13:57
    
    bool loop_cond_21872;
    
    if (loop_cond_21871) {
        // stdlib/hashmap.fut:13:26-34
        
        int64_t loop_cond_t_res_21918 = ((int64_t *) z30U_mem_22579.mem)[u64_arg0_21870];
        
        // stdlib/hashmap.fut:13:37-43
        
        bool loop_cond_t_res_21919 = loop_cond_t_res_21918 == key_17727;
        
        // stdlib/hashmap.fut:13:37-43
        
        bool loop_cond_t_res_21920 = !loop_cond_t_res_21919;
        
        loop_cond_21872 = loop_cond_t_res_21920;
    } else {
        loop_cond_21872 = 0;
    }
    // stdlib/hashmap.fut:12:16-13:57
    
    bool slot_res_21876;
    int64_t slot_res_21877;
    bool loop_while_21878;
    int64_t i_21879;
    
    loop_while_21878 = loop_cond_21872;
    i_21879 = u64_arg0_21870;
    while (loop_while_21878) {
        // stdlib/hashmap.fut:13:49-51
        
        int64_t za_lhs_21880 = add64((int64_t) 1, i_21879);
        
        // stdlib/hashmap.fut:13:52-57
        
        int64_t loopres_21881 = mask_21861 & za_lhs_21880;
        
        // stdlib/hashmap.fut:13:12-20
        
        bool loop_cond_21882 = ((bool *) z30U_mem_22581.mem)[loopres_21881];
        
        // stdlib/hashmap.fut:13:12-43
        
        bool loop_cond_21883;
        
        if (loop_cond_21882) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_21921 = ((int64_t *) z30U_mem_22579.mem)[loopres_21881];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21922 = loop_cond_t_res_21921 == key_17727;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_21923 = !loop_cond_t_res_21922;
            
            loop_cond_21883 = loop_cond_t_res_21923;
        } else {
            loop_cond_21883 = 0;
        }
        
        bool loop_while_tmp_22835 = loop_cond_21883;
        int64_t i_tmp_22836 = loopres_21881;
        
        loop_while_21878 = loop_while_tmp_22835;
        i_21879 = i_tmp_22836;
    }
    slot_res_21876 = loop_while_21878;
    slot_res_21877 = i_21879;
    // stdlib/hashmap.fut:21:27-35
    
    bool x_17735 = sle64((int64_t) 0, slot_res_21877);
    
    // stdlib/hashmap.fut:21:27-35
    
    bool y_17736 = slt64(slot_res_21877, dz2080U_17722);
    
    // stdlib/hashmap.fut:21:27-35
    
    bool bounds_check_17737 = x_17735 && y_17736;
    
    // stdlib/hashmap.fut:21:27-35
    
    bool index_certs_17738;
    
    if (!bounds_check_17737) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_21877, "] out of bounds for array of shape [", (long long) dz2080U_17722, "].", "-> #0  stdlib/hashmap.fut:21:27-35\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/hashmap.fut:21:27-35
    
    bool put_raw_res_17741 = ((bool *) z30U_mem_22581.mem)[slot_res_21877];
    bool cond_neg_18547 = !put_raw_res_17741;
    int64_t zp_rhs_17745 = btoi_bool_i64(cond_neg_18547);
    
    // stdlib/hashmap.fut:21:21-51
    
    int64_t count_17747 = add64(z31U_17726, zp_rhs_17745);
    
    // stdlib/hashmap.fut:22:7-34
    ((int64_t *) z30U_mem_22579.mem)[slot_res_21877] = key_17727;
    // stdlib/hashmap.fut:22:7-34
    ((int64_t *) z30U_mem_22580.mem)[slot_res_21877] = value_17728;
    // stdlib/hashmap.fut:22:7-34
    ((bool *) z30U_mem_22581.mem)[slot_res_21877] = 1;
    if (memblock_set(ctx, &mem_out_22831, &z30U_mem_22579, "z30U_mem_22579") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22832, &z30U_mem_22580, "z30U_mem_22580") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_22833, &z30U_mem_22581, "z30U_mem_22581") != 0)
        return 1;
    prim_out_22834 = count_17747;
    if (memblock_set(ctx, &*mem_out_p_22986, &mem_out_22831, "mem_out_22831") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22987, &mem_out_22832, "mem_out_22832") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_22988, &mem_out_22833, "mem_out_22833") != 0)
        return 1;
    *out_prim_out_22989 = prim_out_22834;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_22833, "mem_out_22833") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22832, "mem_out_22832") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_22831, "mem_out_22831") != 0)
            return 1;
    }
    return err;
}

int futhark_entry_arithmetic(struct futhark_context *ctx, struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 **out, const bool in0, const struct futhark_u32_1d *in1, const bool in2, const struct futhark_u32_1d *in3, const uint32_t in4)
{
    int64_t dz2080U_15866 = (int64_t) 0;
    int64_t dz2081U_15867 = (int64_t) 0;
    bool negative_a_15868 = 0;
    bool negative_b_15870 = 0;
    int32_t factor_15872 = 0;
    int64_t prim_out_22835 = (int64_t) 0;
    int64_t prim_out_22836 = (int64_t) 0;
    int64_t prim_out_22837 = (int64_t) 0;
    int64_t prim_out_22838 = (int64_t) 0;
    bool prim_out_22839 = 0;
    bool prim_out_22840 = 0;
    bool prim_out_22841 = 0;
    bool prim_out_22842 = 0;
    int64_t prim_out_22843 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_22834;
    
    mem_out_22834.references = NULL;
    
    struct memblock mem_out_22833;
    
    mem_out_22833.references = NULL;
    
    struct memblock mem_out_22832;
    
    mem_out_22832.references = NULL;
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    struct memblock b_mem_22580;
    
    b_mem_22580.references = NULL;
    
    struct memblock a_mem_22579;
    
    a_mem_22579.references = NULL;
    negative_a_15868 = in0;
    a_mem_22579 = in1->mem;
    dz2080U_15866 = in1->shape[0];
    negative_b_15870 = in2;
    b_mem_22580 = in3->mem;
    dz2081U_15867 = in3->shape[0];
    factor_15872 = in4;
    if (!(dz2080U_15866 == in1->shape[0] && dz2081U_15867 == in3->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_arithmetic(ctx, &mem_out_22831, &mem_out_22832, &mem_out_22833, &mem_out_22834, &prim_out_22835, &prim_out_22836, &prim_out_22837, &prim_out_22838, &prim_out_22839, &prim_out_22840, &prim_out_22841, &prim_out_22842, &prim_out_22843, a_mem_22579, b_mem_22580, dz2080U_15866, dz2081U_15867, negative_a_15868, negative_b_15870, factor_15872);
        if (ret == 0) {
            assert((*out = (struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *) malloc(sizeof(struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64))) != NULL);
            (*out)->v0 = prim_out_22839;
            assert(((*out)->v1 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v1->mem = mem_out_22831;
            (*out)->v1->shape[0] = prim_out_22835;
            (*out)->v2 = prim_out_22840;
            assert(((*out)->v3 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v3->mem = mem_out_22832;
            (*out)->v3->shape[0] = prim_out_22836;
            (*out)->v4 = prim_out_22841;
            assert(((*out)->v5 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v5->mem = mem_out_22833;
            (*out)->v5->shape[0] = prim_out_22837;
            (*out)->v6 = prim_out_22842;
            assert(((*out)->v7 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v7->mem = mem_out_22834;
            (*out)->v7->shape[0] = prim_out_22838;
            (*out)->v8 = prim_out_22843;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_hashmap_increment(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0, const struct futhark_i64_1d *in1)
{
    int64_t dz2080U_18149 = (int64_t) 0;
    int64_t dz2081U_18150 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    struct memblock deltas_mem_22580;
    
    deltas_mem_22580.references = NULL;
    
    struct memblock keys_mem_22579;
    
    keys_mem_22579.references = NULL;
    keys_mem_22579 = in0->mem;
    dz2080U_18149 = in0->shape[0];
    deltas_mem_22580 = in1->mem;
    dz2081U_18150 = in1->shape[0];
    if (!(dz2080U_18149 == in0->shape[0] && dz2081U_18150 == in1->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_hashmap_increment(ctx, &mem_out_22831, keys_mem_22579, deltas_mem_22580, dz2080U_18149, dz2081U_18150);
        if (ret == 0) {
            assert((*out = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d))) != NULL);
            (*out)->mem = mem_out_22831;
            (*out)->shape[0] = dz2080U_18149;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_hashmap_reserved(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0, const struct futhark_i64_1d *in1, const int64_t in2)
{
    int64_t dz2080U_18411 = (int64_t) 0;
    int64_t dz2081U_18412 = (int64_t) 0;
    int64_t batch_18415 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    struct memblock deltas_mem_22580;
    
    deltas_mem_22580.references = NULL;
    
    struct memblock keys_mem_22579;
    
    keys_mem_22579.references = NULL;
    keys_mem_22579 = in0->mem;
    dz2080U_18411 = in0->shape[0];
    deltas_mem_22580 = in1->mem;
    dz2081U_18412 = in1->shape[0];
    batch_18415 = in2;
    if (!(dz2080U_18411 == in0->shape[0] && dz2081U_18412 == in1->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_hashmap_reserved(ctx, &mem_out_22831, keys_mem_22579, deltas_mem_22580, dz2080U_18411, dz2081U_18412, batch_18415);
        if (ret == 0) {
            assert((*out = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d))) != NULL);
            (*out)->mem = mem_out_22831;
            (*out)->shape[0] = dz2080U_18411;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_hashmap_test(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0)
{
    int64_t dz2080U_17971 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    struct memblock keys_mem_22579;
    
    keys_mem_22579.references = NULL;
    keys_mem_22579 = in0->mem;
    dz2080U_17971 = in0->shape[0];
    if (!(dz2080U_17971 == in0->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_hashmap_test(ctx, &mem_out_22831, keys_mem_22579, dz2080U_17971);
        if (ret == 0) {
            assert((*out = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d))) != NULL);
            (*out)->mem = mem_out_22831;
            (*out)->shape[0] = dz2080U_17971;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_regex_test(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 **out, const struct futhark_u8_1d *in0, const struct futhark_u8_1d *in1, const struct futhark_u8_1d *in2)
{
    int64_t dz2080U_17516 = (int64_t) 0;
    int64_t dz2081U_17517 = (int64_t) 0;
    int64_t dz2082U_17518 = (int64_t) 0;
    int64_t prim_out_22832 = (int64_t) 0;
    int64_t prim_out_22833 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    struct memblock replacement_mem_22581;
    
    replacement_mem_22581.references = NULL;
    
    struct memblock text_mem_22580;
    
    text_mem_22580.references = NULL;
    
    struct memblock pattern_mem_22579;
    
    pattern_mem_22579.references = NULL;
    pattern_mem_22579 = in0->mem;
    dz2080U_17516 = in0->shape[0];
    text_mem_22580 = in1->mem;
    dz2081U_17517 = in1->shape[0];
    replacement_mem_22581 = in2->mem;
    dz2082U_17518 = in2->shape[0];
    if (!(dz2080U_17516 == in0->shape[0] && (dz2081U_17517 == in1->shape[0] && dz2082U_17518 == in2->shape[0]))) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_regex_test(ctx, &mem_out_22831, &prim_out_22832, &prim_out_22833, pattern_mem_22579, text_mem_22580, replacement_mem_22581, dz2080U_17516, dz2081U_17517, dz2082U_17518);
        if (ret == 0) {
            assert((*out = (struct futhark_opaque_tup2_i64_arr1d_u8 *) malloc(sizeof(struct futhark_opaque_tup2_i64_arr1d_u8))) != NULL);
            (*out)->v0 = prim_out_22833;
            assert(((*out)->v1 = (struct futhark_u8_1d *) malloc(sizeof(struct futhark_u8_1d))) != NULL);
            (*out)->v1->mem = mem_out_22831;
            (*out)->v1->shape[0] = prim_out_22832;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_wrap_test(struct futhark_context *ctx, struct futhark_u8_1d **out, const int64_t in0, const struct futhark_u8_1d *in1)
{
    int64_t dz2080U_18542 = (int64_t) 0;
    int64_t width_18543 = (int64_t) 0;
    int64_t prim_out_22832 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_22831;
    
    mem_out_22831.references = NULL;
    
    struct memblock text_mem_22579;
    
    text_mem_22579.references = NULL;
    width_18543 = in0;
    text_mem_22579 = in1->mem;
    dz2080U_18542 = in1->shape[0];
    if (!(dz2080U_18542 == in1->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_wrap_test(ctx, &mem_out_22831, &prim_out_22832, text_mem_22579, dz2080U_18542, width_18543);
        if (ret == 0) {
            assert((*out = (struct futhark_u8_1d *) malloc(sizeof(struct futhark_u8_1d))) != NULL);
            (*out)->mem = mem_out_22831;
            (*out)->shape[0] = prim_out_22832;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
  
