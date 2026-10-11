
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
static int32_t arithmeticzistatic_array_realtype_21092[2] = { 1,0};
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

FUTHARK_FUN_ATTR int futrts_empty_7765(struct futhark_context *ctx, struct memblock *mem_out_p_21075, struct memblock *mem_out_p_21076, struct memblock *mem_out_p_21077, int64_t *out_prim_out_21078, int64_t capacity_16772);
FUTHARK_FUN_ATTR int futrts_entry_arithmetic(struct futhark_context *ctx, struct memblock *mem_out_p_21079, struct memblock *mem_out_p_21080, struct memblock *mem_out_p_21081, struct memblock *mem_out_p_21082, int64_t *out_prim_out_21083, int64_t *out_prim_out_21084, int64_t *out_prim_out_21085, int64_t *out_prim_out_21086, bool *out_prim_out_21087, bool *out_prim_out_21088, bool *out_prim_out_21089, bool *out_prim_out_21090, int64_t *out_prim_out_21091, struct memblock a_mem_20795, struct memblock b_mem_20796, int64_t dz2080U_15061, int64_t dz2081U_15062, bool negative_a_15063, bool negative_b_15065, int32_t factor_15067);
FUTHARK_FUN_ATTR int futrts_entry_hashmap_increment(struct futhark_context *ctx, struct memblock *mem_out_p_21093, struct memblock keys_mem_20795, struct memblock deltas_mem_20796, int64_t dz2080U_17344, int64_t dz2081U_17345);
FUTHARK_FUN_ATTR int futrts_entry_hashmap_reserved(struct futhark_context *ctx, struct memblock *mem_out_p_21094, struct memblock keys_mem_20795, struct memblock deltas_mem_20796, int64_t dz2080U_17606, int64_t dz2081U_17607, int64_t batch_17610);
FUTHARK_FUN_ATTR int futrts_entry_hashmap_test(struct futhark_context *ctx, struct memblock *mem_out_p_21095, struct memblock keys_mem_20795, int64_t dz2080U_17166);
FUTHARK_FUN_ATTR int futrts_entry_regex_test(struct futhark_context *ctx, struct memblock *mem_out_p_21096, int64_t *out_prim_out_21097, int64_t *out_prim_out_21098, struct memblock pattern_mem_20795, struct memblock text_mem_20796, struct memblock replacement_mem_20797, int64_t dz2080U_16711, int64_t dz2081U_16712, int64_t dz2082U_16713);
FUTHARK_FUN_ATTR int futrts_entry_wrap_test(struct futhark_context *ctx, struct memblock *mem_out_p_21100, int64_t *out_prim_out_21101, struct memblock text_mem_20795, int64_t dz2080U_17737, int64_t width_17738);
FUTHARK_FUN_ATTR int futrts_find_11321(struct futhark_context *ctx, int64_t *out_prim_out_21102, int64_t *out_prim_out_21103, struct memblock z30U_mem_20795, struct memblock z31U_mem_20796, struct memblock z32U_mem_20797, struct memblock z33U_mem_20798, struct memblock text_mem_20799, int64_t dz2080U_15867, int64_t dz2081U_15868, int64_t dz2082U_15869, int64_t dz2083U_15870, int64_t dz2080U_15871, int64_t start_15877);
FUTHARK_FUN_ATTR int futrts_grow_11344(struct futhark_context *ctx, struct memblock *mem_out_p_21104, struct memblock *mem_out_p_21105, struct memblock *mem_out_p_21106, int64_t *out_prim_out_21107, int64_t *out_prim_out_21108, struct memblock z30U_mem_20795, struct memblock z30U_mem_20796, struct memblock z30U_mem_20797, int64_t dz2080U_16954, int64_t z31U_16958);
FUTHARK_FUN_ATTR int futrts_literal_11305(struct futhark_context *ctx, int8_t *out_prim_out_21109, int64_t *out_prim_out_21110, struct memblock pattern_mem_20795, int64_t dz2080U_15180, int64_t p_15182);
FUTHARK_FUN_ATTR int futrts_mask_set_8212(struct futhark_context *ctx, struct memblock *mem_out_p_21111, struct memblock mask_mem_20795, int8_t c_15233);
FUTHARK_FUN_ATTR int futrts_match_branch_11323(struct futhark_context *ctx, int64_t *out_prim_out_21112, struct memblock z30U_mem_20795, struct memblock z31U_mem_20796, struct memblock z32U_mem_20797, struct memblock z33U_mem_20798, struct memblock text_mem_20799, int64_t dz2080U_15775, int64_t dz2081U_15776, int64_t dz2082U_15777, int64_t dz2083U_15778, int64_t dz2080U_15779, int64_t token_15784, int64_t last_15785, int64_t position_15787);
FUTHARK_FUN_ATTR int futrts_mul_abs_11275(struct futhark_context *ctx, struct memblock *mem_out_p_21113, int64_t *out_prim_out_21114, struct memblock a_mem_20795, int64_t dz2080U_14603, int32_t b_14605);
FUTHARK_FUN_ATTR int futrts_normalise_11258(struct futhark_context *ctx, struct memblock *mem_out_p_21115, int64_t *out_prim_out_21116, bool *out_prim_out_21117, struct memblock a_mem_20795, int64_t dz2080U_14197, bool negative_14198);
FUTHARK_FUN_ATTR int futrts_put_raw_11346(struct futhark_context *ctx, struct memblock *mem_out_p_21118, struct memblock *mem_out_p_21119, struct memblock *mem_out_p_21120, int64_t *out_prim_out_21121, struct memblock z30U_mem_20795, struct memblock z30U_mem_20796, struct memblock z30U_mem_20797, int64_t dz2080U_16917, int64_t z31U_16921, int64_t key_16922, int64_t value_16923);

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

FUTHARK_FUN_ATTR int futrts_empty_7765(struct futhark_context *ctx, struct memblock *mem_out_p_21075, struct memblock *mem_out_p_21076, struct memblock *mem_out_p_21077, int64_t *out_prim_out_21078, int64_t capacity_16772)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_20799;
    
    mem_20799.references = NULL;
    
    struct memblock mem_20798;
    
    mem_20798.references = NULL;
    
    struct memblock mem_20796;
    
    mem_20796.references = NULL;
    
    struct memblock mem_out_21000;
    
    mem_out_21000.references = NULL;
    
    struct memblock mem_out_20999;
    
    mem_out_20999.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t prim_out_21001;
    
    // stdlib/hashmap.fut:4:11-55
    
    bool cond_16773 = slt64((int64_t) 0, capacity_16772);
    
    // stdlib/hashmap.fut:4:47-49
    
    int64_t za_rhs_16775 = sub64(capacity_16772, (int64_t) 1);
    
    // stdlib/hashmap.fut:4:36-49
    
    int64_t zeze_lhs_16777 = capacity_16772 & za_rhs_16775;
    
    // stdlib/hashmap.fut:4:51-55
    
    bool tmp_t_res_16779 = zeze_lhs_16777 == (int64_t) 0;
    bool x_17742 = cond_16773 && tmp_t_res_16779;
    int64_t bytes_20795 = (int64_t) 8 * capacity_16772;
    
    // stdlib/hashmap.fut:4:3-5:46
    
    bool assert_c_16787;
    
    if (!x_17742) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "Assertion is false: (capacity > 0 && (capacity & (capacity - 1)) == 0)", "-> #0  stdlib/hashmap.fut:4:3-5:46\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    if (memblock_alloc(ctx, &mem_20796, bytes_20795, "mem_20796")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:5:4-40
    for (int64_t nest_i_21002 = 0; nest_i_21002 < capacity_16772; nest_i_21002++) {
        ((int64_t *) mem_20796.mem)[nest_i_21002] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_20798, bytes_20795, "mem_20798")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:5:4-40
    for (int64_t nest_i_21003 = 0; nest_i_21003 < capacity_16772; nest_i_21003++) {
        ((int64_t *) mem_20798.mem)[nest_i_21003] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_20799, capacity_16772, "mem_20799")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:5:4-40
    for (int64_t nest_i_21004 = 0; nest_i_21004 < capacity_16772; nest_i_21004++) {
        ((bool *) mem_20799.mem)[nest_i_21004] = 0;
    }
    if (memblock_set(ctx, &mem_out_20998, &mem_20796, "mem_20796") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20999, &mem_20798, "mem_20798") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_21000, &mem_20799, "mem_20799") != 0)
        return 1;
    prim_out_21001 = (int64_t) 0;
    if (memblock_set(ctx, &*mem_out_p_21075, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21076, &mem_out_20999, "mem_out_20999") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21077, &mem_out_21000, "mem_out_21000") != 0)
        return 1;
    *out_prim_out_21078 = prim_out_21001;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_20799, "mem_20799") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20798, "mem_20798") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20796, "mem_20796") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_21000, "mem_out_21000") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20999, "mem_out_20999") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_arithmetic(struct futhark_context *ctx, struct memblock *mem_out_p_21079, struct memblock *mem_out_p_21080, struct memblock *mem_out_p_21081, struct memblock *mem_out_p_21082, int64_t *out_prim_out_21083, int64_t *out_prim_out_21084, int64_t *out_prim_out_21085, int64_t *out_prim_out_21086, bool *out_prim_out_21087, bool *out_prim_out_21088, bool *out_prim_out_21089, bool *out_prim_out_21090, int64_t *out_prim_out_21091, struct memblock a_mem_20795, struct memblock b_mem_20796, int64_t dz2080U_15061, int64_t dz2081U_15062, bool negative_a_15063, bool negative_b_15065, int32_t factor_15067)
{
    (void) ctx;
    
    int err = 0;
    struct memblock ext_mem_20971;
    
    ext_mem_20971.references = NULL;
    
    struct memblock ext_mem_20944;
    
    ext_mem_20944.references = NULL;
    
    struct memblock ext_mem_20943;
    
    ext_mem_20943.references = NULL;
    
    struct memblock mem_20934;
    
    mem_20934.references = NULL;
    
    struct memblock ext_mem_20956;
    
    ext_mem_20956.references = NULL;
    
    struct memblock ext_mem_20955;
    
    ext_mem_20955.references = NULL;
    
    struct memblock mem_20946;
    
    mem_20946.references = NULL;
    
    struct memblock ext_mem_20957;
    
    ext_mem_20957.references = NULL;
    
    struct memblock ext_mem_20969;
    
    ext_mem_20969.references = NULL;
    
    struct memblock ext_mem_20968;
    
    ext_mem_20968.references = NULL;
    
    struct memblock mem_20959;
    
    mem_20959.references = NULL;
    
    struct memblock ext_mem_20970;
    
    ext_mem_20970.references = NULL;
    
    struct memblock ext_mem_20932;
    
    ext_mem_20932.references = NULL;
    
    struct memblock ext_mem_20972;
    
    ext_mem_20972.references = NULL;
    
    struct memblock ext_mem_20973;
    
    ext_mem_20973.references = NULL;
    
    struct memblock ext_mem_20974;
    
    ext_mem_20974.references = NULL;
    
    struct memblock mem_20931;
    
    mem_20931.references = NULL;
    
    struct memblock ext_mem_20930;
    
    ext_mem_20930.references = NULL;
    
    struct memblock ext_mem_20877;
    
    ext_mem_20877.references = NULL;
    
    struct memblock ext_mem_20876;
    
    ext_mem_20876.references = NULL;
    
    struct memblock ext_mem_20849;
    
    ext_mem_20849.references = NULL;
    
    struct memblock ext_mem_20848;
    
    ext_mem_20848.references = NULL;
    
    struct memblock mem_20839;
    
    mem_20839.references = NULL;
    
    struct memblock ext_mem_20861;
    
    ext_mem_20861.references = NULL;
    
    struct memblock ext_mem_20860;
    
    ext_mem_20860.references = NULL;
    
    struct memblock mem_20851;
    
    mem_20851.references = NULL;
    
    struct memblock ext_mem_20862;
    
    ext_mem_20862.references = NULL;
    
    struct memblock ext_mem_20874;
    
    ext_mem_20874.references = NULL;
    
    struct memblock ext_mem_20873;
    
    ext_mem_20873.references = NULL;
    
    struct memblock mem_20864;
    
    mem_20864.references = NULL;
    
    struct memblock ext_mem_20875;
    
    ext_mem_20875.references = NULL;
    
    struct memblock ext_mem_20810;
    
    ext_mem_20810.references = NULL;
    
    struct memblock ext_mem_20809;
    
    ext_mem_20809.references = NULL;
    
    struct memblock mem_20800;
    
    mem_20800.references = NULL;
    
    struct memblock ext_mem_20822;
    
    ext_mem_20822.references = NULL;
    
    struct memblock ext_mem_20821;
    
    ext_mem_20821.references = NULL;
    
    struct memblock mem_20812;
    
    mem_20812.references = NULL;
    
    struct memblock ext_mem_20823;
    
    ext_mem_20823.references = NULL;
    
    struct memblock ext_mem_20835;
    
    ext_mem_20835.references = NULL;
    
    struct memblock ext_mem_20834;
    
    ext_mem_20834.references = NULL;
    
    struct memblock mem_20825;
    
    mem_20825.references = NULL;
    
    struct memblock ext_mem_20836;
    
    ext_mem_20836.references = NULL;
    
    struct memblock mem_param_tmp_21012;
    
    mem_param_tmp_21012.references = NULL;
    
    struct memblock ext_mem_20921;
    
    ext_mem_20921.references = NULL;
    
    struct memblock mem_20912;
    
    mem_20912.references = NULL;
    
    struct memblock mem_param_tmp_21021;
    
    mem_param_tmp_21021.references = NULL;
    
    struct memblock ext_mem_20907;
    
    ext_mem_20907.references = NULL;
    
    struct memblock mem_20898;
    
    mem_20898.references = NULL;
    
    struct memblock mem_param_20896;
    
    mem_param_20896.references = NULL;
    
    struct memblock ext_mem_20910;
    
    ext_mem_20910.references = NULL;
    
    struct memblock ext_mem_20893;
    
    ext_mem_20893.references = NULL;
    
    struct memblock mem_20892;
    
    mem_20892.references = NULL;
    
    struct memblock ext_mem_20890;
    
    ext_mem_20890.references = NULL;
    
    struct memblock mem_param_20887;
    
    mem_param_20887.references = NULL;
    
    struct memblock ext_mem_20926;
    
    ext_mem_20926.references = NULL;
    
    struct memblock mem_20881;
    
    mem_20881.references = NULL;
    
    struct memblock ext_mem_20879;
    
    ext_mem_20879.references = NULL;
    
    struct memblock ext_mem_20878;
    
    ext_mem_20878.references = NULL;
    
    struct memblock ext_mem_20837;
    
    ext_mem_20837.references = NULL;
    
    struct memblock ext_mem_20798;
    
    ext_mem_20798.references = NULL;
    
    struct memblock ext_mem_20797;
    
    ext_mem_20797.references = NULL;
    
    struct memblock mem_out_21001;
    
    mem_out_21001.references = NULL;
    
    struct memblock mem_out_21000;
    
    mem_out_21000.references = NULL;
    
    struct memblock mem_out_20999;
    
    mem_out_20999.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t prim_out_21002;
    int64_t prim_out_21003;
    int64_t prim_out_21004;
    int64_t prim_out_21005;
    bool prim_out_21006;
    bool prim_out_21007;
    bool prim_out_21008;
    bool prim_out_21009;
    int64_t prim_out_21010;
    
    // probes/stdlib-tests.fut:6:11-33
    
    int64_t arithmetic_res_18973;
    bool arithmetic_res_18974;
    
    if (futrts_normalise_11258(ctx, &ext_mem_20797, &arithmetic_res_18973, &arithmetic_res_18974, a_mem_20795, dz2080U_15061, negative_a_15063) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:7:11-33
    
    int64_t arithmetic_res_18976;
    bool arithmetic_res_18977;
    
    if (futrts_normalise_11258(ctx, &ext_mem_20798, &arithmetic_res_18976, &arithmetic_res_18977, b_mem_20796, dz2081U_15062, negative_b_15065) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:38:3-40:39
    
    bool cond_18979 = arithmetic_res_18974 == arithmetic_res_18977;
    
    // probes/stdlib-tests.fut:6:7-8:10
    
    bool x_18980 = !cond_18979;
    
    // stdlib/bigint.fut:21:14-42
    
    int64_t max_res_19027 = smax64(arithmetic_res_18973, arithmetic_res_18976);
    
    // stdlib/bigint.fut:22:52-58
    
    int64_t dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19028 = add64((int64_t) 1, max_res_19027);
    
    // stdlib/bigint.fut:28:7-32
    
    bool x_19053 = sle64((int64_t) 0, max_res_19027);
    
    // stdlib/bigint.fut:28:7-32
    
    bool y_19054 = slt64(max_res_19027, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19028);
    
    // stdlib/bigint.fut:28:7-32
    
    bool bounds_check_19055 = x_19053 && y_19054;
    bool protect_assert_disj_19715 = x_18980 || bounds_check_19055;
    
    // stdlib/bigint.fut:28:7-32
    
    bool index_certs_19056;
    
    if (!protect_assert_disj_19715) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) max_res_19027, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19028, "].", "-> #0  stdlib/bigint.fut:28:7-32\n   #1  stdlib/bigint.fut:38:37-52\n   #2  probes/stdlib-tests.fut:6:7-8:10\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:41:34-54
    
    bool normalise_arg_19117 = !arithmetic_res_18977;
    
    // stdlib/bigint.fut:41:34-54
    
    int64_t negate_res_19118;
    bool negate_res_19119;
    
    if (futrts_normalise_11258(ctx, &ext_mem_20837, &negate_res_19118, &negate_res_19119, ext_mem_20798, arithmetic_res_18976, normalise_arg_19117) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:38:3-40:39
    
    bool cond_19121 = arithmetic_res_18974 == negate_res_19119;
    
    // stdlib/bigint.fut:42:15-63
    
    bool x_19122 = !cond_19121;
    
    // stdlib/bigint.fut:21:14-42
    
    int64_t max_res_19169 = smax64(arithmetic_res_18973, negate_res_19118);
    
    // stdlib/bigint.fut:22:52-58
    
    int64_t dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19170 = add64((int64_t) 1, max_res_19169);
    
    // stdlib/bigint.fut:28:7-32
    
    bool x_19195 = sle64((int64_t) 0, max_res_19169);
    
    // stdlib/bigint.fut:28:7-32
    
    bool y_19196 = slt64(max_res_19169, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19170);
    
    // stdlib/bigint.fut:28:7-32
    
    bool bounds_check_19197 = x_19195 && y_19196;
    bool protect_assert_disj_19711 = x_19122 || bounds_check_19197;
    
    // stdlib/bigint.fut:28:7-32
    
    bool index_certs_19198;
    
    if (!protect_assert_disj_19711) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) max_res_19169, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19170, "].", "-> #0  stdlib/bigint.fut:28:7-32\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:6:7-8:23\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:52:13-43
    
    bool cond_19264 = arithmetic_res_18976 == (int64_t) 1;
    
    // stdlib/bigint.fut:52:13-43
    
    bool tmp_19265;
    
    if (cond_19264) {
        // stdlib/bigint.fut:52:32-38
        
        bool y_20586 = slt64((int64_t) 0, arithmetic_res_18976);
        
        // stdlib/bigint.fut:52:32-38
        
        bool index_certs_20587;
        
        if (!y_20586) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) arithmetic_res_18976, "].", "-> #0  stdlib/bigint.fut:52:32-38\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint.fut:52:32-38
        
        int32_t zeze_lhs_20588 = ((int32_t *) ext_mem_20798.mem)[(int64_t) 0];
        
        // stdlib/bigint.fut:52:39-43
        
        bool tmp_t_res_20589 = zeze_lhs_20588 == 0;
        
        tmp_19265 = tmp_t_res_20589;
    } else {
        tmp_19265 = 0;
    }
    // stdlib/bigint.fut:53:62-64
    
    int64_t u32_arg0_19270 = sub64(arithmetic_res_18976, (int64_t) 1);
    
    // stdlib/bigint.fut:53:48-65
    
    bool x_19271 = sle64((int64_t) 0, u32_arg0_19270);
    
    // stdlib/bigint.fut:53:48-65
    
    bool y_19272 = slt64(u32_arg0_19270, arithmetic_res_18976);
    
    // stdlib/bigint.fut:53:48-65
    
    bool bounds_check_19273 = x_19271 && y_19272;
    
    // stdlib/bigint.fut:53:48-65
    
    bool index_certs_19274;
    
    if (!bounds_check_19273) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u32_arg0_19270, "] out of bounds for array of shape [", (long long) arithmetic_res_18976, "].", "-> #0  stdlib/bigint.fut:53:48-65\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:53:48-65
    
    int32_t u32_arg0_19275 = ((int32_t *) ext_mem_20798.mem)[u32_arg0_19270];
    
    // stdlib/bigint.fut:53:40-65
    
    int64_t u32_res_19276 = zext_i32_i64(u32_arg0_19275);
    
    // stdlib/bigint.fut:53:65-67
    
    int64_t zs_rhs_19277 = add64((int64_t) 1, u32_res_19276);
    
    // stdlib/bigint.fut:53:38-67
    
    bool zzero_19278 = zs_rhs_19277 == (int64_t) 0;
    
    // stdlib/bigint.fut:53:38-67
    
    bool nonzzero_19279 = !zzero_19278;
    
    // stdlib/bigint.fut:53:38-67
    
    bool nonzzero_cert_19280;
    
    if (!nonzzero_19279) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/bigint.fut:53:38-67\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:53:38-67
    
    int64_t u64_arg0_19281 = udiv64((int64_t) 4294967296, zs_rhs_19277);
    
    // stdlib/bigint.fut:53:16-67
    
    int32_t u64_res_19282 = zext_i64_i32(u64_arg0_19281);
    
    // stdlib/bigint.fut:54:19-37
    
    int64_t numerator_19283;
    
    if (futrts_mul_abs_11275(ctx, &ext_mem_20878, &numerator_19283, ext_mem_20797, arithmetic_res_18973, u64_res_19282) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:55:17-35
    
    int64_t divisor_19285;
    
    if (futrts_mul_abs_11275(ctx, &ext_mem_20879, &divisor_19285, ext_mem_20798, arithmetic_res_18976, u64_res_19282) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:56:42-57
    
    int64_t zp_lhs_19287 = sub64(numerator_19283, divisor_19285);
    
    // stdlib/bigint.fut:56:57-59
    
    int64_t max_arg1_19288 = add64((int64_t) 1, zp_lhs_19287);
    
    // stdlib/bigint.fut:56:15-59
    
    int64_t max_res_19289 = smax64((int64_t) 1, max_arg1_19288);
    int64_t bytes_20880 = (int64_t) 4 * max_res_19289;
    
    if (memblock_alloc(ctx, &mem_20881, bytes_20880, "mem_20881")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:57:37-57
    for (int64_t nest_i_21011 = 0; nest_i_21011 < max_res_19289; nest_i_21011++) {
        ((int32_t *) mem_20881.mem)[nest_i_21011] = 0;
    }
    // stdlib/bigint.fut:57:23-69:15
    
    int64_t w_minus_1_19291 = sub64(max_res_19289, (int64_t) 1);
    
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool loop_nonempty_19292 = slt64((int64_t) 0, max_res_19289);
    
    // stdlib/bigint.fut:62:103-105
    
    int64_t u32_arg0_19293 = sub64(divisor_19285, (int64_t) 1);
    
    // stdlib/bigint.fut:62:81-106
    
    bool x_19294 = sle64((int64_t) 0, u32_arg0_19293);
    
    // stdlib/bigint.fut:62:81-106
    
    bool y_19295 = slt64(u32_arg0_19293, divisor_19285);
    
    // stdlib/bigint.fut:62:81-106
    
    bool bounds_check_19296 = x_19294 && y_19295;
    
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool loop_not_taken_19297 = !loop_nonempty_19292;
    
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool protect_assert_disj_19298 = bounds_check_19296 || loop_not_taken_19297;
    
    // stdlib/bigint.fut:62:81-106
    
    bool index_certs_19299;
    
    if (!protect_assert_disj_19298) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u32_arg0_19293, "] out of bounds for array of shape [", (long long) divisor_19285, "].", "-> #0  stdlib/bigint.fut:62:81-106\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:62:81-106
    
    int32_t u32_arg0_19300;
    
    if (loop_nonempty_19292) {
        // probes/stdlib-tests.fut:6:7-8:60
        
        int32_t x_20590 = ((int32_t *) ext_mem_20879.mem)[u32_arg0_19293];
        
        u32_arg0_19300 = x_20590;
    } else {
        u32_arg0_19300 = 0;
    }
    // probes/stdlib-tests.fut:6:7-8:60
    
    int64_t u32_res_19302 = zext_i32_i64(u32_arg0_19300);
    
    // stdlib/bigint.fut:62:72-106
    
    bool zzero_19303 = u32_res_19302 == (int64_t) 0;
    
    // stdlib/bigint.fut:62:72-106
    
    bool nonzzero_19304 = !zzero_19303;
    
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool protect_assert_disj_19305 = loop_not_taken_19297 || nonzzero_19304;
    
    // stdlib/bigint.fut:62:72-106
    
    bool nonzzero_cert_19306;
    
    if (!protect_assert_disj_19305) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/bigint.fut:62:72-106\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:57:23-69:15
    
    int64_t quotient_res_19307;
    int64_t loop_dz2081Uz2080U_19311;
    
    if (memblock_set(ctx, &mem_param_20887, &ext_mem_20878, "ext_mem_20878") != 0)
        return 1;
    loop_dz2081Uz2080U_19311 = numerator_19283;
    for (int64_t i_19310 = 0; i_19310 < max_res_19289; i_19310++) {
        // probes/stdlib-tests.fut:6:7-8:60
        
        int64_t binop_x_19314 = w_minus_1_19291 - i_19310;
        
        // stdlib/bigint.fut:59:38-44
        
        int64_t high_index_19315 = add64(divisor_19285, binop_x_19314);
        
        // stdlib/bigint.fut:60:18-76
        
        bool cond_19316 = slt64(high_index_19315, loop_dz2081Uz2080U_19311);
        
        // stdlib/bigint.fut:60:56-69
        
        bool x_19317 = sle64((int64_t) 0, high_index_19315);
        
        // stdlib/bigint.fut:60:56-69
        
        bool bounds_check_19318 = cond_19316 && x_19317;
        
        // probes/stdlib-tests.fut:6:7-8:60
        
        bool loop_not_taken_19319 = !cond_19316;
        
        // probes/stdlib-tests.fut:6:7-8:60
        
        bool protect_assert_disj_19320 = bounds_check_19318 || loop_not_taken_19319;
        
        // stdlib/bigint.fut:60:56-69
        
        bool index_certs_19321;
        
        if (!protect_assert_disj_19320) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) high_index_19315, "] out of bounds for array of shape [", (long long) loop_dz2081Uz2080U_19311, "].", "-> #0  stdlib/bigint.fut:60:56-69\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint.fut:60:56-69
        
        int32_t u32_arg0_19322;
        
        if (cond_19316) {
            // probes/stdlib-tests.fut:6:7-8:60
            
            int32_t x_20591 = ((int32_t *) mem_param_20887.mem)[high_index_19315];
            
            u32_arg0_19322 = x_20591;
        } else {
            u32_arg0_19322 = 0;
        }
        // stdlib/bigint.fut:60:48-69
        
        int64_t u32_res_19324 = zext_i32_i64(u32_arg0_19322);
        
        // stdlib/bigint.fut:60:18-76
        
        int64_t high_19325;
        
        if (cond_19316) {
            high_19325 = u32_res_19324;
        } else {
            high_19325 = (int64_t) 0;
        }
        // stdlib/bigint.fut:61:30-32
        
        int64_t zl_lhs_19326 = sub64(high_index_19315, (int64_t) 1);
        
        // stdlib/bigint.fut:61:17-79
        
        bool cond_19327 = slt64(zl_lhs_19326, loop_dz2081Uz2080U_19311);
        
        // stdlib/bigint.fut:61:57-72
        
        bool x_19328 = sle64((int64_t) 0, zl_lhs_19326);
        
        // stdlib/bigint.fut:61:57-72
        
        bool bounds_check_19329 = cond_19327 && x_19328;
        
        // probes/stdlib-tests.fut:6:7-8:60
        
        bool loop_not_taken_19330 = !cond_19327;
        
        // probes/stdlib-tests.fut:6:7-8:60
        
        bool protect_assert_disj_19331 = bounds_check_19329 || loop_not_taken_19330;
        
        // stdlib/bigint.fut:61:57-72
        
        bool index_certs_19332;
        
        if (!protect_assert_disj_19331) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_19326, "] out of bounds for array of shape [", (long long) loop_dz2081Uz2080U_19311, "].", "-> #0  stdlib/bigint.fut:61:57-72\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint.fut:61:57-72
        
        int32_t u32_arg0_19333;
        
        if (cond_19327) {
            // probes/stdlib-tests.fut:6:7-8:60
            
            int32_t x_20592 = ((int32_t *) mem_param_20887.mem)[zl_lhs_19326];
            
            u32_arg0_19333 = x_20592;
        } else {
            u32_arg0_19333 = 0;
        }
        // stdlib/bigint.fut:61:49-72
        
        int64_t u32_res_19335 = zext_i32_i64(u32_arg0_19333);
        
        // stdlib/bigint.fut:61:17-79
        
        int64_t low_19336;
        
        if (cond_19327) {
            low_19336 = u32_res_19335;
        } else {
            low_19336 = (int64_t) 0;
        }
        // stdlib/bigint.fut:62:61-66
        
        int64_t zb_lhs_19337 = shl64(high_19325, (int64_t) 32);
        
        // stdlib/bigint.fut:62:67-71
        
        int64_t zs_lhs_19338 = low_19336 | zb_lhs_19337;
        
        // stdlib/bigint.fut:62:72-106
        
        int64_t min_arg1_19339 = udiv64(zs_lhs_19338, u32_res_19302);
        
        // stdlib/bigint.fut:62:31-106
        
        int64_t unsign_arg0_19340 = umin64((int64_t) 4294967295, min_arg1_19339);
        
        // stdlib/bigint.fut:62:22-107
        
        int32_t u64_res_19341 = zext_i64_i32(unsign_arg0_19340);
        
        // stdlib/bigint.fut:64:63-87
        
        int64_t zpzp_rhs_19344;
        
        if (futrts_mul_abs_11275(ctx, &ext_mem_20890, &zpzp_rhs_19344, ext_mem_20879, divisor_19285, u64_res_19341) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:64:39-87
        
        int64_t conc_tmp_19346 = binop_x_19314 + zpzp_rhs_19344;
        int64_t bytes_20891 = (int64_t) 4 * conc_tmp_19346;
        int64_t bytes_20911 = (int64_t) 4 * loop_dz2081Uz2080U_19311;
        
        // stdlib/bigint.fut:68:11-27
        
        bool y_19466 = slt64(binop_x_19314, max_res_19289);
        
        // stdlib/bigint.fut:68:11-27
        
        bool index_certs_19467;
        
        if (!y_19466) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) binop_x_19314, "] out of bounds for array of shape [", (long long) max_res_19289, "].", "-> #0  stdlib/bigint.fut:68:11-27\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        if (memblock_alloc(ctx, &mem_20892, bytes_20891, "mem_20892")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:63:21-41
        for (int64_t nest_i_21016 = 0; nest_i_21016 < binop_x_19314; nest_i_21016++) {
            ((int32_t *) mem_20892.mem)[nest_i_21016] = 0;
        }
        // stdlib/bigint.fut:64:39-87
        
        int64_t tmp_offs_21017 = (int64_t) 0;
        
        if (!(tmp_offs_21017 == (int64_t) 0)) {
            // stdlib/bigint.fut:64:39-87
            lmad_copy_4b(ctx, 1, (uint32_t *) mem_20892.mem, tmp_offs_21017, (int64_t []) {(int64_t) 1}, (uint32_t *) mem_20892.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {binop_x_19314});
        }
        tmp_offs_21017 += binop_x_19314;
        // stdlib/bigint.fut:64:39-87
        lmad_copy_4b(ctx, 1, (uint32_t *) mem_20892.mem, tmp_offs_21017, (int64_t []) {(int64_t) 1}, (uint32_t *) ext_mem_20890.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {zpzp_rhs_19344});
        tmp_offs_21017 += zpzp_rhs_19344;
        if (memblock_unref(ctx, &ext_mem_20890, "ext_mem_20890") != 0)
            return 1;
        // stdlib/bigint.fut:64:22-87
        
        int64_t loopres_19348;
        bool loopres_19349;
        
        if (futrts_normalise_11258(ctx, &ext_mem_20893, &loopres_19348, &loopres_19349, mem_20892, conc_tmp_19346, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_20892, "mem_20892") != 0)
            return 1;
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_19351 = loopres_19348 == loop_dz2081Uz2080U_19311;
        
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_19352 = !cond_19351;
        
        // stdlib/bigint.fut:12:3-16:17
        
        int64_t compare_abs_res_19353;
        
        if (cond_19352) {
            // stdlib/bigint.fut:12:32-69
            
            bool cond_20593 = slt64(loopres_19348, loop_dz2081Uz2080U_19311);
            
            // stdlib/bigint.fut:12:32-69
            
            int64_t compare_abs_res_t_res_20594;
            
            if (cond_20593) {
                compare_abs_res_t_res_20594 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_20594 = (int64_t) 1;
            }
            compare_abs_res_19353 = compare_abs_res_t_res_20594;
        } else {
            // stdlib/bigint.fut:13:53-55
            
            int64_t tmp_19356 = sub64(loopres_19348, (int64_t) 1);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool cond_19357 = sle64((int64_t) 0, tmp_19356);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool compare_abs_res_f_res_19358;
            int64_t compare_abs_res_f_res_19359;
            int64_t compare_abs_res_f_res_19360;
            bool loop_while_19361;
            int64_t i_19362;
            int64_t result_19363;
            
            loop_while_19361 = cond_19357;
            i_19362 = tmp_19356;
            result_19363 = (int64_t) 0;
            while (loop_while_19361) {
                // stdlib/bigint.fut:15:20-24
                
                bool x_19365 = sle64((int64_t) 0, i_19362);
                
                // stdlib/bigint.fut:15:20-24
                
                bool y_19366 = slt64(i_19362, loopres_19348);
                
                // stdlib/bigint.fut:15:20-24
                
                bool bounds_check_19367 = x_19365 && y_19366;
                
                // stdlib/bigint.fut:15:20-24
                
                bool index_certs_19368;
                
                if (!bounds_check_19367) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19362, "] out of bounds for array of shape [", (long long) loopres_19348, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:65:29-66:77\n   #2  probes/stdlib-tests.fut:6:7-8:60\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:27-31
                
                bool y_19370 = slt64(i_19362, loop_dz2081Uz2080U_19311);
                
                // stdlib/bigint.fut:15:27-31
                
                bool bounds_check_19371 = x_19365 && y_19370;
                
                // stdlib/bigint.fut:15:27-31
                
                bool index_certs_19372;
                
                if (!bounds_check_19371) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19362, "] out of bounds for array of shape [", (long long) loop_dz2081Uz2080U_19311, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:65:29-66:77\n   #2  probes/stdlib-tests.fut:6:7-8:60\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:14-16
                
                int64_t tmp_19364 = sub64(i_19362, (int64_t) 1);
                
                // stdlib/bigint.fut:15:20-24
                
                int32_t zl_lhs_19369 = ((int32_t *) ext_mem_20893.mem)[i_19362];
                
                // stdlib/bigint.fut:15:27-31
                
                int32_t zl_rhs_19373 = ((int32_t *) mem_param_20887.mem)[i_19362];
                
                // stdlib/bigint.fut:15:17-73
                
                bool cond_19374 = ult32(zl_lhs_19369, zl_rhs_19373);
                
                // stdlib/bigint.fut:15:17-73
                
                int64_t tmp_19375;
                
                if (cond_19374) {
                    tmp_19375 = (int64_t) -1;
                } else {
                    // stdlib/bigint.fut:15:45-73
                    
                    bool cond_19376 = ult32(zl_rhs_19373, zl_lhs_19369);
                    
                    // stdlib/bigint.fut:65:29-66:77
                    
                    int64_t tmp_f_res_19377 = btoi_bool_i64(cond_19376);
                    
                    tmp_19375 = tmp_f_res_19377;
                }
                // stdlib/bigint.fut:14:16-37
                
                bool cond_19378 = sle64((int64_t) 0, tmp_19364);
                
                // stdlib/bigint.fut:14:33-37
                
                bool loop_cond_t_res_19379 = tmp_19375 == (int64_t) 0;
                
                // stdlib/bigint.fut:65:29-66:77
                
                bool x_19380 = cond_19378 && loop_cond_t_res_19379;
                bool loop_while_tmp_21018 = x_19380;
                int64_t i_tmp_21019 = tmp_19364;
                int64_t result_tmp_21020 = tmp_19375;
                
                loop_while_19361 = loop_while_tmp_21018;
                i_19362 = i_tmp_21019;
                result_19363 = result_tmp_21020;
            }
            compare_abs_res_f_res_19358 = loop_while_19361;
            compare_abs_res_f_res_19359 = i_19362;
            compare_abs_res_f_res_19360 = result_19363;
            compare_abs_res_19353 = compare_abs_res_f_res_19360;
        }
        // stdlib/bigint.fut:65:29-66:77
        
        bool loop_cond_19381 = slt64((int64_t) 0, compare_abs_res_19353);
        
        // stdlib/bigint.fut:65:29-66:77
        
        int64_t loopres_19382;
        bool loopres_19383;
        int32_t loopres_19384;
        int64_t loop_dz2083Uz2081U_19386;
        bool loop_while_19387;
        int32_t digit_19388;
        
        if (memblock_set(ctx, &mem_param_20896, &ext_mem_20893, "ext_mem_20893") != 0)
            return 1;
        loop_dz2083Uz2081U_19386 = loopres_19348;
        loop_while_19387 = loop_cond_19381;
        digit_19388 = u64_res_19341;
        while (loop_while_19387) {
            int64_t bytes_20897 = (int64_t) 4 * loop_dz2083Uz2081U_19386;
            
            // stdlib/bigint.fut:66:50-52
            
            int32_t tmp_19390 = sub32(digit_19388, 1);
            
            if (memblock_alloc(ctx, &mem_20898, bytes_20897, "mem_20898")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_21026 = 0; nest_i_21026 < loop_dz2083Uz2081U_19386; nest_i_21026++) {
                ((int32_t *) mem_20898.mem)[nest_i_21026] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_19393;
            int64_t borrow_19396 = (int64_t) 0;
            
            for (int64_t i_19394 = 0; i_19394 < loop_dz2083Uz2081U_19386; i_19394++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_19397 = ((int32_t *) mem_param_20896.mem)[i_19394];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_19398 = zext_i32_i64(u32_arg0_19397);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_19399 = slt64(i_19394, high_index_19315);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_19400;
                
                if (cond_19399) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_20597;
                    
                    if (!cond_19399) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19394, "] out of bounds for array of shape [", (long long) high_index_19315, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:66:53-76\n   #2  probes/stdlib-tests.fut:6:7-8:60\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    
                    bool index_concat_cmp_20598 = sle64(binop_x_19314, i_19394);
                    int32_t index_concat_branch_20599;
                    
                    if (index_concat_cmp_20598) {
                        int64_t index_concat_i_20600 = sub64(i_19394, binop_x_19314);
                        int32_t index_concat_20601 = ((int32_t *) ext_mem_20879.mem)[index_concat_i_20600];
                        
                        index_concat_branch_20599 = index_concat_20601;
                    } else {
                        index_concat_branch_20599 = 0;
                    }
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_20602 = zext_i32_i64(index_concat_branch_20599);
                    
                    zp_lhs_19400 = u32_res_20602;
                } else {
                    zp_lhs_19400 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_19404 = add64(borrow_19396, zp_lhs_19400);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_19405 = sub64(u32_res_19398, y_19404);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_19406 = zext_i64_i32(u64_arg0_19405);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_20898.mem)[i_19394] = u64_res_19406;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_19408 = ult64(u32_res_19398, y_19404);
                
                // stdlib/bigint.fut:66:53-76
                
                int64_t tmp_19409 = btoi_bool_i64(cond_19408);
                int64_t borrow_tmp_21028 = tmp_19409;
                
                borrow_19396 = borrow_tmp_21028;
            }
            sub_abs_res_19393 = borrow_19396;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_19410;
            bool sub_abs_res_19411;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20907, &sub_abs_res_19410, &sub_abs_res_19411, mem_20898, loop_dz2083Uz2081U_19386, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_20898, "mem_20898") != 0)
                return 1;
            // stdlib/bigint.fut:12:3-16:17
            
            bool cond_19413 = sub_abs_res_19410 == loop_dz2081Uz2080U_19311;
            
            // stdlib/bigint.fut:12:3-16:17
            
            bool cond_19414 = !cond_19413;
            
            // stdlib/bigint.fut:12:3-16:17
            
            int64_t compare_abs_res_19415;
            
            if (cond_19414) {
                // stdlib/bigint.fut:12:32-69
                
                bool cond_20603 = slt64(sub_abs_res_19410, loop_dz2081Uz2080U_19311);
                
                // stdlib/bigint.fut:12:32-69
                
                int64_t compare_abs_res_t_res_20604;
                
                if (cond_20603) {
                    compare_abs_res_t_res_20604 = (int64_t) -1;
                } else {
                    compare_abs_res_t_res_20604 = (int64_t) 1;
                }
                compare_abs_res_19415 = compare_abs_res_t_res_20604;
            } else {
                // stdlib/bigint.fut:13:53-55
                
                int64_t tmp_19418 = sub64(sub_abs_res_19410, (int64_t) 1);
                
                // stdlib/bigint.fut:13:26-15:74
                
                bool cond_19419 = sle64((int64_t) 0, tmp_19418);
                
                // stdlib/bigint.fut:13:26-15:74
                
                bool compare_abs_res_f_res_19420;
                int64_t compare_abs_res_f_res_19421;
                int64_t compare_abs_res_f_res_19422;
                bool loop_while_19423;
                int64_t i_19424;
                int64_t result_19425;
                
                loop_while_19423 = cond_19419;
                i_19424 = tmp_19418;
                result_19425 = (int64_t) 0;
                while (loop_while_19423) {
                    // stdlib/bigint.fut:15:20-24
                    
                    bool x_19427 = sle64((int64_t) 0, i_19424);
                    
                    // stdlib/bigint.fut:15:20-24
                    
                    bool y_19428 = slt64(i_19424, sub_abs_res_19410);
                    
                    // stdlib/bigint.fut:15:20-24
                    
                    bool bounds_check_19429 = x_19427 && y_19428;
                    
                    // stdlib/bigint.fut:15:20-24
                    
                    bool index_certs_19430;
                    
                    if (!bounds_check_19429) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19424, "] out of bounds for array of shape [", (long long) sub_abs_res_19410, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:66:15-36\n   #2  probes/stdlib-tests.fut:6:7-8:60\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:15:27-31
                    
                    bool y_19432 = slt64(i_19424, loop_dz2081Uz2080U_19311);
                    
                    // stdlib/bigint.fut:15:27-31
                    
                    bool bounds_check_19433 = x_19427 && y_19432;
                    
                    // stdlib/bigint.fut:15:27-31
                    
                    bool index_certs_19434;
                    
                    if (!bounds_check_19433) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19424, "] out of bounds for array of shape [", (long long) loop_dz2081Uz2080U_19311, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:66:15-36\n   #2  probes/stdlib-tests.fut:6:7-8:60\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:15:14-16
                    
                    int64_t tmp_19426 = sub64(i_19424, (int64_t) 1);
                    
                    // stdlib/bigint.fut:15:20-24
                    
                    int32_t zl_lhs_19431 = ((int32_t *) ext_mem_20907.mem)[i_19424];
                    
                    // stdlib/bigint.fut:15:27-31
                    
                    int32_t zl_rhs_19435 = ((int32_t *) mem_param_20887.mem)[i_19424];
                    
                    // stdlib/bigint.fut:15:17-73
                    
                    bool cond_19436 = ult32(zl_lhs_19431, zl_rhs_19435);
                    
                    // stdlib/bigint.fut:15:17-73
                    
                    int64_t tmp_19437;
                    
                    if (cond_19436) {
                        tmp_19437 = (int64_t) -1;
                    } else {
                        // stdlib/bigint.fut:15:45-73
                        
                        bool cond_19438 = ult32(zl_rhs_19435, zl_lhs_19431);
                        
                        // stdlib/bigint.fut:66:15-36
                        
                        int64_t tmp_f_res_19439 = btoi_bool_i64(cond_19438);
                        
                        tmp_19437 = tmp_f_res_19439;
                    }
                    // stdlib/bigint.fut:14:16-37
                    
                    bool cond_19440 = sle64((int64_t) 0, tmp_19426);
                    
                    // stdlib/bigint.fut:14:33-37
                    
                    bool loop_cond_t_res_19441 = tmp_19437 == (int64_t) 0;
                    
                    // stdlib/bigint.fut:66:15-36
                    
                    bool x_19442 = cond_19440 && loop_cond_t_res_19441;
                    bool loop_while_tmp_21029 = x_19442;
                    int64_t i_tmp_21030 = tmp_19426;
                    int64_t result_tmp_21031 = tmp_19437;
                    
                    loop_while_19423 = loop_while_tmp_21029;
                    i_19424 = i_tmp_21030;
                    result_19425 = result_tmp_21031;
                }
                compare_abs_res_f_res_19420 = loop_while_19423;
                compare_abs_res_f_res_19421 = i_19424;
                compare_abs_res_f_res_19422 = result_19425;
                compare_abs_res_19415 = compare_abs_res_f_res_19422;
            }
            // stdlib/bigint.fut:66:37-40
            
            bool loop_cond_19443 = slt64((int64_t) 0, compare_abs_res_19415);
            
            if (memblock_set(ctx, &mem_param_tmp_21021, &ext_mem_20907, "ext_mem_20907") != 0)
                return 1;
            
            int64_t loop_dz2083Uz2081U_tmp_21022 = sub_abs_res_19410;
            bool loop_while_tmp_21023 = loop_cond_19443;
            int32_t digit_tmp_21024 = tmp_19390;
            
            if (memblock_set(ctx, &mem_param_20896, &mem_param_tmp_21021, "mem_param_tmp_21021") != 0)
                return 1;
            loop_dz2083Uz2081U_19386 = loop_dz2083Uz2081U_tmp_21022;
            loop_while_19387 = loop_while_tmp_21023;
            digit_19388 = digit_tmp_21024;
        }
        if (memblock_set(ctx, &ext_mem_20910, &mem_param_20896, "mem_param_20896") != 0)
            return 1;
        loopres_19382 = loop_dz2083Uz2081U_19386;
        loopres_19383 = loop_while_19387;
        loopres_19384 = digit_19388;
        if (memblock_unref(ctx, &ext_mem_20893, "ext_mem_20893") != 0)
            return 1;
        if (memblock_alloc(ctx, &mem_20912, bytes_20911, "mem_20912")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:31:44-69
        for (int64_t nest_i_21032 = 0; nest_i_21032 < loop_dz2081Uz2080U_19311; nest_i_21032++) {
            ((int32_t *) mem_20912.mem)[nest_i_21032] = 0;
        }
        // stdlib/bigint.fut:31:23-35:39
        
        int64_t sub_abs_res_19446;
        int64_t borrow_19449 = (int64_t) 0;
        
        for (int64_t i_19447 = 0; i_19447 < loop_dz2081Uz2080U_19311; i_19447++) {
            // stdlib/bigint.fut:32:21-25
            
            int32_t u32_arg0_19450 = ((int32_t *) mem_param_20887.mem)[i_19447];
            
            // stdlib/bigint.fut:32:13-25
            
            int64_t u32_res_19451 = zext_i32_i64(u32_arg0_19450);
            
            // stdlib/bigint.fut:33:14-54
            
            bool cond_19452 = slt64(i_19447, loopres_19382);
            
            // stdlib/bigint.fut:33:14-54
            
            int64_t zp_lhs_19453;
            
            if (cond_19452) {
                // stdlib/bigint.fut:33:43-47
                
                bool index_certs_20605;
                
                if (!cond_19452) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19447, "] out of bounds for array of shape [", (long long) loopres_19382, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:67:15-32\n   #2  probes/stdlib-tests.fut:6:7-8:60\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:33:43-47
                
                int32_t u32_arg0_20606 = ((int32_t *) ext_mem_20910.mem)[i_19447];
                
                // stdlib/bigint.fut:33:35-47
                
                int64_t u32_res_20607 = zext_i32_i64(u32_arg0_20606);
                
                zp_lhs_19453 = u32_res_20607;
            } else {
                zp_lhs_19453 = (int64_t) 0;
            }
            // stdlib/bigint.fut:33:55-62
            
            int64_t y_19457 = add64(borrow_19449, zp_lhs_19453);
            
            // stdlib/bigint.fut:34:28-30
            
            int64_t u64_arg0_19458 = sub64(u32_res_19451, y_19457);
            
            // stdlib/bigint.fut:34:18-30
            
            int32_t u64_res_19459 = zext_i64_i32(u64_arg0_19458);
            
            // stdlib/bigint.fut:34:9-31
            ((int32_t *) mem_20912.mem)[i_19447] = u64_res_19459;
            // stdlib/bigint.fut:35:13-38
            
            bool cond_19461 = ult64(u32_res_19451, y_19457);
            
            // stdlib/bigint.fut:67:15-32
            
            int64_t tmp_19462 = btoi_bool_i64(cond_19461);
            int64_t borrow_tmp_21034 = tmp_19462;
            
            borrow_19449 = borrow_tmp_21034;
        }
        sub_abs_res_19446 = borrow_19449;
        if (memblock_unref(ctx, &ext_mem_20910, "ext_mem_20910") != 0)
            return 1;
        // stdlib/bigint.fut:36:7-26
        
        int64_t sub_abs_res_19463;
        bool sub_abs_res_19464;
        
        if (futrts_normalise_11258(ctx, &ext_mem_20921, &sub_abs_res_19463, &sub_abs_res_19464, mem_20912, loop_dz2081Uz2080U_19311, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_20912, "mem_20912") != 0)
            return 1;
        // stdlib/bigint.fut:68:11-27
        ((int32_t *) mem_20881.mem)[binop_x_19314] = loopres_19384;
        if (memblock_set(ctx, &mem_param_tmp_21012, &ext_mem_20921, "ext_mem_20921") != 0)
            return 1;
        
        int64_t loop_dz2081Uz2080U_tmp_21013 = sub_abs_res_19463;
        
        if (memblock_set(ctx, &mem_param_20887, &mem_param_tmp_21012, "mem_param_tmp_21012") != 0)
            return 1;
        loop_dz2081Uz2080U_19311 = loop_dz2081Uz2080U_tmp_21013;
    }
    if (memblock_set(ctx, &ext_mem_20926, &mem_param_20887, "mem_param_20887") != 0)
        return 1;
    quotient_res_19307 = loop_dz2081Uz2080U_19311;
    if (memblock_unref(ctx, &ext_mem_20878, "ext_mem_20878") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20879, "ext_mem_20879") != 0)
        return 1;
    // stdlib/bigint.fut:52:3-74:42
    
    bool assert_cond_19473 = !tmp_19265;
    
    // stdlib/bigint.fut:52:3-74:42
    
    bool assert_c_19474;
    
    if (!assert_cond_19473) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "Assertion is false: (!(length b.1 == 1 && b.1[0] == 0))", "-> #0  stdlib/bigint.fut:52:3-74:42\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:73:11-74:42
    
    bool cond_19476 = quotient_res_19307 == (int64_t) 1;
    
    // stdlib/bigint.fut:73:39-51
    
    bool y_19477 = slt64((int64_t) 0, quotient_res_19307);
    
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool loop_not_taken_19478 = !cond_19476;
    
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool protect_assert_disj_19479 = y_19477 || loop_not_taken_19478;
    
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool protect_assert_disj_19480 = cond_18979 || protect_assert_disj_19479;
    
    // stdlib/bigint.fut:73:39-51
    
    bool index_certs_19481;
    
    if (!protect_assert_disj_19480) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) quotient_res_19307, "].", "-> #0  stdlib/bigint.fut:73:39-51\n   #1  probes/stdlib-tests.fut:6:7-8:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:39:11-30
    
    int64_t zgze_lhs_18981;
    
    if (x_18980) {
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_20388 = arithmetic_res_18973 == arithmetic_res_18976;
        
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_20389 = !cond_20388;
        
        // stdlib/bigint.fut:12:3-16:17
        
        int64_t compare_abs_res_20390;
        
        if (cond_20389) {
            // stdlib/bigint.fut:12:32-69
            
            bool cond_20391 = slt64(arithmetic_res_18973, arithmetic_res_18976);
            
            // stdlib/bigint.fut:12:32-69
            
            int64_t compare_abs_res_t_res_20392;
            
            if (cond_20391) {
                compare_abs_res_t_res_20392 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_20392 = (int64_t) 1;
            }
            compare_abs_res_20390 = compare_abs_res_t_res_20392;
        } else {
            // stdlib/bigint.fut:13:53-55
            
            int64_t tmp_20393 = sub64(arithmetic_res_18973, (int64_t) 1);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool cond_20394 = sle64((int64_t) 0, tmp_20393);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool compare_abs_res_f_res_20395;
            int64_t compare_abs_res_f_res_20396;
            int64_t compare_abs_res_f_res_20397;
            bool loop_while_20398;
            int64_t i_20399;
            int64_t result_20400;
            
            loop_while_20398 = cond_20394;
            i_20399 = tmp_20393;
            result_20400 = (int64_t) 0;
            while (loop_while_20398) {
                // stdlib/bigint.fut:15:20-24
                
                bool x_20402 = sle64((int64_t) 0, i_20399);
                
                // stdlib/bigint.fut:15:20-24
                
                bool y_20403 = slt64(i_20399, arithmetic_res_18973);
                
                // stdlib/bigint.fut:15:20-24
                
                bool bounds_check_20404 = x_20402 && y_20403;
                
                // stdlib/bigint.fut:15:20-24
                
                bool index_certs_20405;
                
                if (!bounds_check_20404) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20399, "] out of bounds for array of shape [", (long long) arithmetic_res_18973, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  probes/stdlib-tests.fut:6:7-8:10\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:27-31
                
                bool y_20407 = slt64(i_20399, arithmetic_res_18976);
                
                // stdlib/bigint.fut:15:27-31
                
                bool bounds_check_20408 = x_20402 && y_20407;
                
                // stdlib/bigint.fut:15:27-31
                
                bool index_certs_20409;
                
                if (!bounds_check_20408) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20399, "] out of bounds for array of shape [", (long long) arithmetic_res_18976, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  probes/stdlib-tests.fut:6:7-8:10\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:14-16
                
                int64_t tmp_20401 = sub64(i_20399, (int64_t) 1);
                
                // stdlib/bigint.fut:15:20-24
                
                int32_t zl_lhs_20406 = ((int32_t *) ext_mem_20797.mem)[i_20399];
                
                // stdlib/bigint.fut:15:27-31
                
                int32_t zl_rhs_20410 = ((int32_t *) ext_mem_20798.mem)[i_20399];
                
                // stdlib/bigint.fut:15:17-73
                
                bool cond_20411 = ult32(zl_lhs_20406, zl_rhs_20410);
                
                // stdlib/bigint.fut:15:17-73
                
                int64_t tmp_20412;
                
                if (cond_20411) {
                    tmp_20412 = (int64_t) -1;
                } else {
                    // stdlib/bigint.fut:15:45-73
                    
                    bool cond_20413 = ult32(zl_rhs_20410, zl_lhs_20406);
                    
                    // probes/stdlib-tests.fut:6:7-8:10
                    
                    int64_t tmp_f_res_20414 = btoi_bool_i64(cond_20413);
                    
                    tmp_20412 = tmp_f_res_20414;
                }
                // stdlib/bigint.fut:14:16-37
                
                bool cond_20415 = sle64((int64_t) 0, tmp_20401);
                
                // stdlib/bigint.fut:14:33-37
                
                bool loop_cond_t_res_20416 = tmp_20412 == (int64_t) 0;
                
                // probes/stdlib-tests.fut:6:7-8:10
                
                bool x_20417 = cond_20415 && loop_cond_t_res_20416;
                bool loop_while_tmp_21035 = x_20417;
                int64_t i_tmp_21036 = tmp_20401;
                int64_t result_tmp_21037 = tmp_20412;
                
                loop_while_20398 = loop_while_tmp_21035;
                i_20399 = i_tmp_21036;
                result_20400 = result_tmp_21037;
            }
            compare_abs_res_f_res_20395 = loop_while_20398;
            compare_abs_res_f_res_20396 = i_20399;
            compare_abs_res_f_res_20397 = result_20400;
            compare_abs_res_20390 = compare_abs_res_f_res_20397;
        }
        zgze_lhs_18981 = compare_abs_res_20390;
    } else {
        zgze_lhs_18981 = (int64_t) 0;
    }
    // stdlib/bigint.fut:39:8-40:39
    
    bool cond_19023 = sle64((int64_t) 0, zgze_lhs_18981);
    int64_t bytes_20799 = (int64_t) 4 * arithmetic_res_18976;
    int64_t bytes_20811 = (int64_t) 4 * arithmetic_res_18973;
    int64_t bytes_20824 = (int64_t) 4 * dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19028;
    
    // stdlib/bigint.fut:38:3-40:39
    
    int64_t add_res_19024;
    bool add_res_19025;
    
    if (cond_18979) {
        if (memblock_alloc(ctx, &mem_20825, bytes_20824, "mem_20825")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:22:41-64
        for (int64_t nest_i_21038 = 0; nest_i_21038 < dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19028; nest_i_21038++) {
            ((int32_t *) mem_20825.mem)[nest_i_21038] = 0;
        }
        // stdlib/bigint.fut:22:21-27:23
        
        int64_t add_abs_res_20426;
        int64_t carry_20429 = (int64_t) 0;
        
        for (int64_t i_20427 = 0; i_20427 < max_res_19027; i_20427++) {
            // stdlib/bigint.fut:26:9-29
            
            bool y_20443 = slt64(i_20427, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19028);
            
            // stdlib/bigint.fut:26:9-29
            
            bool index_certs_20444;
            
            if (!y_20443) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20427, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19028, "].", "-> #0  stdlib/bigint.fut:26:9-29\n   #1  stdlib/bigint.fut:38:37-52\n   #2  probes/stdlib-tests.fut:6:7-8:10\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/bigint.fut:23:13-53
            
            bool cond_20430 = slt64(i_20427, arithmetic_res_18973);
            
            // stdlib/bigint.fut:23:13-53
            
            int64_t x_20431;
            
            if (cond_20430) {
                // stdlib/bigint.fut:23:42-46
                
                bool index_certs_20432;
                
                if (!cond_20430) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20427, "] out of bounds for array of shape [", (long long) arithmetic_res_18973, "].", "-> #0  stdlib/bigint.fut:23:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  probes/stdlib-tests.fut:6:7-8:10\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:23:42-46
                
                int32_t u32_arg0_20433 = ((int32_t *) ext_mem_20797.mem)[i_20427];
                
                // stdlib/bigint.fut:23:34-46
                
                int64_t u32_res_20434 = zext_i32_i64(u32_arg0_20433);
                
                x_20431 = u32_res_20434;
            } else {
                x_20431 = (int64_t) 0;
            }
            // stdlib/bigint.fut:24:13-53
            
            bool cond_20435 = slt64(i_20427, arithmetic_res_18976);
            
            // stdlib/bigint.fut:24:13-53
            
            int64_t y_20436;
            
            if (cond_20435) {
                // stdlib/bigint.fut:24:42-46
                
                bool index_certs_20437;
                
                if (!cond_20435) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20427, "] out of bounds for array of shape [", (long long) arithmetic_res_18976, "].", "-> #0  stdlib/bigint.fut:24:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  probes/stdlib-tests.fut:6:7-8:10\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:24:42-46
                
                int32_t u32_arg0_20438 = ((int32_t *) ext_mem_20798.mem)[i_20427];
                
                // stdlib/bigint.fut:24:34-46
                
                int64_t u32_res_20439 = zext_i32_i64(u32_arg0_20438);
                
                y_20436 = u32_res_20439;
            } else {
                y_20436 = (int64_t) 0;
            }
            // stdlib/bigint.fut:25:16-18
            
            int64_t zp_lhs_20440 = add64(x_20431, y_20436);
            
            // stdlib/bigint.fut:25:18-24
            
            int64_t sum_20441 = add64(carry_20429, zp_lhs_20440);
            
            // stdlib/bigint.fut:26:18-29
            
            int32_t u64_res_20442 = zext_i64_i32(sum_20441);
            
            // stdlib/bigint.fut:26:9-29
            ((int32_t *) mem_20825.mem)[i_20427] = u64_res_20442;
            // stdlib/bigint.fut:27:17-22
            
            int64_t tmp_20446 = lshr64(sum_20441, (int64_t) 32);
            int64_t carry_tmp_21040 = tmp_20446;
            
            carry_20429 = carry_tmp_21040;
        }
        add_abs_res_20426 = carry_20429;
        // stdlib/bigint.fut:28:19-32
        
        int32_t u64_res_20447 = zext_i64_i32(add_abs_res_20426);
        
        // stdlib/bigint.fut:28:7-32
        ((int32_t *) mem_20825.mem)[max_res_19027] = u64_res_20447;
        // stdlib/bigint.fut:29:7-26
        
        int64_t add_abs_res_20449;
        bool add_abs_res_20450;
        
        if (futrts_normalise_11258(ctx, &ext_mem_20834, &add_abs_res_20449, &add_abs_res_20450, mem_20825, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19028, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_20825, "mem_20825") != 0)
            return 1;
        // stdlib/bigint.fut:38:22-52
        
        int64_t add_res_t_res_20452;
        bool add_res_t_res_20453;
        
        if (futrts_normalise_11258(ctx, &ext_mem_20835, &add_res_t_res_20452, &add_res_t_res_20453, ext_mem_20834, add_abs_res_20449, arithmetic_res_18974) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_20834, "ext_mem_20834") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_20836, &ext_mem_20835, "ext_mem_20835") != 0)
            return 1;
        add_res_19024 = add_res_t_res_20452;
        add_res_19025 = add_res_t_res_20453;
    } else {
        // stdlib/bigint.fut:39:8-40:39
        
        int64_t add_res_f_res_19064;
        bool add_res_f_res_19065;
        
        if (cond_19023) {
            if (memblock_alloc(ctx, &mem_20812, bytes_20811, "mem_20812")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_21041 = 0; nest_i_21041 < arithmetic_res_18973; nest_i_21041++) {
                ((int32_t *) mem_20812.mem)[nest_i_21041] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_20460;
            int64_t borrow_20463 = (int64_t) 0;
            
            for (int64_t i_20461 = 0; i_20461 < arithmetic_res_18973; i_20461++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_20464 = ((int32_t *) ext_mem_20797.mem)[i_20461];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_20465 = zext_i32_i64(u32_arg0_20464);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_20466 = slt64(i_20461, arithmetic_res_18976);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_20467;
                
                if (cond_20466) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_20468;
                    
                    if (!cond_20466) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20461, "] out of bounds for array of shape [", (long long) arithmetic_res_18976, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:39:56-71\n   #2  probes/stdlib-tests.fut:6:7-8:10\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:33:43-47
                    
                    int32_t u32_arg0_20469 = ((int32_t *) ext_mem_20798.mem)[i_20461];
                    
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_20470 = zext_i32_i64(u32_arg0_20469);
                    
                    zp_lhs_20467 = u32_res_20470;
                } else {
                    zp_lhs_20467 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_20471 = add64(borrow_20463, zp_lhs_20467);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_20472 = sub64(u32_res_20465, y_20471);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_20473 = zext_i64_i32(u64_arg0_20472);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_20812.mem)[i_20461] = u64_res_20473;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_20475 = ult64(u32_res_20465, y_20471);
                
                // stdlib/bigint.fut:39:56-71
                
                int64_t tmp_20476 = btoi_bool_i64(cond_20475);
                int64_t borrow_tmp_21043 = tmp_20476;
                
                borrow_20463 = borrow_tmp_21043;
            }
            sub_abs_res_20460 = borrow_20463;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_20477;
            bool sub_abs_res_20478;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20821, &sub_abs_res_20477, &sub_abs_res_20478, mem_20812, arithmetic_res_18973, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_20812, "mem_20812") != 0)
                return 1;
            // stdlib/bigint.fut:39:41-71
            
            int64_t add_res_f_res_t_res_20480;
            bool add_res_f_res_t_res_20481;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20822, &add_res_f_res_t_res_20480, &add_res_f_res_t_res_20481, ext_mem_20821, sub_abs_res_20477, arithmetic_res_18974) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_20821, "ext_mem_20821") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20823, &ext_mem_20822, "ext_mem_20822") != 0)
                return 1;
            add_res_f_res_19064 = add_res_f_res_t_res_20480;
            add_res_f_res_19065 = add_res_f_res_t_res_20481;
        } else {
            if (memblock_alloc(ctx, &mem_20800, bytes_20799, "mem_20800")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_21044 = 0; nest_i_21044 < arithmetic_res_18976; nest_i_21044++) {
                ((int32_t *) mem_20800.mem)[nest_i_21044] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_19094;
            int64_t borrow_19097 = (int64_t) 0;
            
            for (int64_t i_19095 = 0; i_19095 < arithmetic_res_18976; i_19095++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_19098 = ((int32_t *) ext_mem_20798.mem)[i_19095];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_19099 = zext_i32_i64(u32_arg0_19098);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_19100 = slt64(i_19095, arithmetic_res_18973);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_19101;
                
                if (cond_19100) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_20483;
                    
                    if (!cond_19100) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19095, "] out of bounds for array of shape [", (long long) arithmetic_res_18973, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:40:23-38\n   #2  probes/stdlib-tests.fut:6:7-8:10\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:33:43-47
                    
                    int32_t u32_arg0_20484 = ((int32_t *) ext_mem_20797.mem)[i_19095];
                    
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_20485 = zext_i32_i64(u32_arg0_20484);
                    
                    zp_lhs_19101 = u32_res_20485;
                } else {
                    zp_lhs_19101 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_19105 = add64(borrow_19097, zp_lhs_19101);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_19106 = sub64(u32_res_19099, y_19105);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_19107 = zext_i64_i32(u64_arg0_19106);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_20800.mem)[i_19095] = u64_res_19107;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_19109 = ult64(u32_res_19099, y_19105);
                
                // stdlib/bigint.fut:40:23-38
                
                int64_t tmp_19110 = btoi_bool_i64(cond_19109);
                int64_t borrow_tmp_21046 = tmp_19110;
                
                borrow_19097 = borrow_tmp_21046;
            }
            sub_abs_res_19094 = borrow_19097;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_19111;
            bool sub_abs_res_19112;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20809, &sub_abs_res_19111, &sub_abs_res_19112, mem_20800, arithmetic_res_18976, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_20800, "mem_20800") != 0)
                return 1;
            // stdlib/bigint.fut:40:8-38
            
            int64_t add_res_f_res_f_res_19114;
            bool add_res_f_res_f_res_19115;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20810, &add_res_f_res_f_res_19114, &add_res_f_res_f_res_19115, ext_mem_20809, sub_abs_res_19111, arithmetic_res_18977) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_20809, "ext_mem_20809") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20823, &ext_mem_20810, "ext_mem_20810") != 0)
                return 1;
            add_res_f_res_19064 = add_res_f_res_f_res_19114;
            add_res_f_res_19065 = add_res_f_res_f_res_19115;
        }
        if (memblock_set(ctx, &ext_mem_20836, &ext_mem_20823, "ext_mem_20823") != 0)
            return 1;
        add_res_19024 = add_res_f_res_19064;
        add_res_19025 = add_res_f_res_19065;
    }
    // stdlib/bigint.fut:39:11-30
    
    int64_t zgze_lhs_19123;
    
    if (x_19122) {
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_20488 = arithmetic_res_18973 == negate_res_19118;
        
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_20489 = !cond_20488;
        
        // stdlib/bigint.fut:12:3-16:17
        
        int64_t compare_abs_res_20490;
        
        if (cond_20489) {
            // stdlib/bigint.fut:12:32-69
            
            bool cond_20491 = slt64(arithmetic_res_18973, negate_res_19118);
            
            // stdlib/bigint.fut:12:32-69
            
            int64_t compare_abs_res_t_res_20492;
            
            if (cond_20491) {
                compare_abs_res_t_res_20492 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_20492 = (int64_t) 1;
            }
            compare_abs_res_20490 = compare_abs_res_t_res_20492;
        } else {
            // stdlib/bigint.fut:13:53-55
            
            int64_t tmp_20493 = sub64(arithmetic_res_18973, (int64_t) 1);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool cond_20494 = sle64((int64_t) 0, tmp_20493);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool compare_abs_res_f_res_20495;
            int64_t compare_abs_res_f_res_20496;
            int64_t compare_abs_res_f_res_20497;
            bool loop_while_20498;
            int64_t i_20499;
            int64_t result_20500;
            
            loop_while_20498 = cond_20494;
            i_20499 = tmp_20493;
            result_20500 = (int64_t) 0;
            while (loop_while_20498) {
                // stdlib/bigint.fut:15:20-24
                
                bool x_20502 = sle64((int64_t) 0, i_20499);
                
                // stdlib/bigint.fut:15:20-24
                
                bool y_20503 = slt64(i_20499, arithmetic_res_18973);
                
                // stdlib/bigint.fut:15:20-24
                
                bool bounds_check_20504 = x_20502 && y_20503;
                
                // stdlib/bigint.fut:15:20-24
                
                bool index_certs_20505;
                
                if (!bounds_check_20504) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20499, "] out of bounds for array of shape [", (long long) arithmetic_res_18973, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:42:15-63\n   #2  probes/stdlib-tests.fut:6:7-8:23\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:27-31
                
                bool y_20507 = slt64(i_20499, negate_res_19118);
                
                // stdlib/bigint.fut:15:27-31
                
                bool bounds_check_20508 = x_20502 && y_20507;
                
                // stdlib/bigint.fut:15:27-31
                
                bool index_certs_20509;
                
                if (!bounds_check_20508) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20499, "] out of bounds for array of shape [", (long long) negate_res_19118, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:42:15-63\n   #2  probes/stdlib-tests.fut:6:7-8:23\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:14-16
                
                int64_t tmp_20501 = sub64(i_20499, (int64_t) 1);
                
                // stdlib/bigint.fut:15:20-24
                
                int32_t zl_lhs_20506 = ((int32_t *) ext_mem_20797.mem)[i_20499];
                
                // stdlib/bigint.fut:15:27-31
                
                int32_t zl_rhs_20510 = ((int32_t *) ext_mem_20837.mem)[i_20499];
                
                // stdlib/bigint.fut:15:17-73
                
                bool cond_20511 = ult32(zl_lhs_20506, zl_rhs_20510);
                
                // stdlib/bigint.fut:15:17-73
                
                int64_t tmp_20512;
                
                if (cond_20511) {
                    tmp_20512 = (int64_t) -1;
                } else {
                    // stdlib/bigint.fut:15:45-73
                    
                    bool cond_20513 = ult32(zl_rhs_20510, zl_lhs_20506);
                    
                    // stdlib/bigint.fut:42:15-63
                    
                    int64_t tmp_f_res_20514 = btoi_bool_i64(cond_20513);
                    
                    tmp_20512 = tmp_f_res_20514;
                }
                // stdlib/bigint.fut:14:16-37
                
                bool cond_20515 = sle64((int64_t) 0, tmp_20501);
                
                // stdlib/bigint.fut:14:33-37
                
                bool loop_cond_t_res_20516 = tmp_20512 == (int64_t) 0;
                
                // stdlib/bigint.fut:42:15-63
                
                bool x_20517 = cond_20515 && loop_cond_t_res_20516;
                bool loop_while_tmp_21047 = x_20517;
                int64_t i_tmp_21048 = tmp_20501;
                int64_t result_tmp_21049 = tmp_20512;
                
                loop_while_20498 = loop_while_tmp_21047;
                i_20499 = i_tmp_21048;
                result_20500 = result_tmp_21049;
            }
            compare_abs_res_f_res_20495 = loop_while_20498;
            compare_abs_res_f_res_20496 = i_20499;
            compare_abs_res_f_res_20497 = result_20500;
            compare_abs_res_20490 = compare_abs_res_f_res_20497;
        }
        zgze_lhs_19123 = compare_abs_res_20490;
    } else {
        zgze_lhs_19123 = (int64_t) 0;
    }
    // stdlib/bigint.fut:39:8-40:39
    
    bool cond_19165 = sle64((int64_t) 0, zgze_lhs_19123);
    int64_t bytes_20838 = (int64_t) 4 * negate_res_19118;
    int64_t bytes_20863 = (int64_t) 4 * dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19170;
    
    // stdlib/bigint.fut:38:3-40:39
    
    int64_t add_res_19166;
    bool add_res_19167;
    
    if (cond_19121) {
        if (memblock_alloc(ctx, &mem_20864, bytes_20863, "mem_20864")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:22:41-64
        for (int64_t nest_i_21050 = 0; nest_i_21050 < dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19170; nest_i_21050++) {
            ((int32_t *) mem_20864.mem)[nest_i_21050] = 0;
        }
        // stdlib/bigint.fut:22:21-27:23
        
        int64_t add_abs_res_20526;
        int64_t carry_20529 = (int64_t) 0;
        
        for (int64_t i_20527 = 0; i_20527 < max_res_19169; i_20527++) {
            // stdlib/bigint.fut:26:9-29
            
            bool y_20543 = slt64(i_20527, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19170);
            
            // stdlib/bigint.fut:26:9-29
            
            bool index_certs_20544;
            
            if (!y_20543) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20527, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19170, "].", "-> #0  stdlib/bigint.fut:26:9-29\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:6:7-8:23\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/bigint.fut:23:13-53
            
            bool cond_20530 = slt64(i_20527, arithmetic_res_18973);
            
            // stdlib/bigint.fut:23:13-53
            
            int64_t x_20531;
            
            if (cond_20530) {
                // stdlib/bigint.fut:23:42-46
                
                bool index_certs_20532;
                
                if (!cond_20530) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20527, "] out of bounds for array of shape [", (long long) arithmetic_res_18973, "].", "-> #0  stdlib/bigint.fut:23:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:6:7-8:23\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:23:42-46
                
                int32_t u32_arg0_20533 = ((int32_t *) ext_mem_20797.mem)[i_20527];
                
                // stdlib/bigint.fut:23:34-46
                
                int64_t u32_res_20534 = zext_i32_i64(u32_arg0_20533);
                
                x_20531 = u32_res_20534;
            } else {
                x_20531 = (int64_t) 0;
            }
            // stdlib/bigint.fut:24:13-53
            
            bool cond_20535 = slt64(i_20527, negate_res_19118);
            
            // stdlib/bigint.fut:24:13-53
            
            int64_t y_20536;
            
            if (cond_20535) {
                // stdlib/bigint.fut:24:42-46
                
                bool index_certs_20537;
                
                if (!cond_20535) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20527, "] out of bounds for array of shape [", (long long) negate_res_19118, "].", "-> #0  stdlib/bigint.fut:24:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:6:7-8:23\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:24:42-46
                
                int32_t u32_arg0_20538 = ((int32_t *) ext_mem_20837.mem)[i_20527];
                
                // stdlib/bigint.fut:24:34-46
                
                int64_t u32_res_20539 = zext_i32_i64(u32_arg0_20538);
                
                y_20536 = u32_res_20539;
            } else {
                y_20536 = (int64_t) 0;
            }
            // stdlib/bigint.fut:25:16-18
            
            int64_t zp_lhs_20540 = add64(x_20531, y_20536);
            
            // stdlib/bigint.fut:25:18-24
            
            int64_t sum_20541 = add64(carry_20529, zp_lhs_20540);
            
            // stdlib/bigint.fut:26:18-29
            
            int32_t u64_res_20542 = zext_i64_i32(sum_20541);
            
            // stdlib/bigint.fut:26:9-29
            ((int32_t *) mem_20864.mem)[i_20527] = u64_res_20542;
            // stdlib/bigint.fut:27:17-22
            
            int64_t tmp_20546 = lshr64(sum_20541, (int64_t) 32);
            int64_t carry_tmp_21052 = tmp_20546;
            
            carry_20529 = carry_tmp_21052;
        }
        add_abs_res_20526 = carry_20529;
        // stdlib/bigint.fut:28:19-32
        
        int32_t u64_res_20547 = zext_i64_i32(add_abs_res_20526);
        
        // stdlib/bigint.fut:28:7-32
        ((int32_t *) mem_20864.mem)[max_res_19169] = u64_res_20547;
        // stdlib/bigint.fut:29:7-26
        
        int64_t add_abs_res_20549;
        bool add_abs_res_20550;
        
        if (futrts_normalise_11258(ctx, &ext_mem_20873, &add_abs_res_20549, &add_abs_res_20550, mem_20864, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19170, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_20864, "mem_20864") != 0)
            return 1;
        // stdlib/bigint.fut:38:22-52
        
        int64_t add_res_t_res_20552;
        bool add_res_t_res_20553;
        
        if (futrts_normalise_11258(ctx, &ext_mem_20874, &add_res_t_res_20552, &add_res_t_res_20553, ext_mem_20873, add_abs_res_20549, arithmetic_res_18974) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_20873, "ext_mem_20873") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_20875, &ext_mem_20874, "ext_mem_20874") != 0)
            return 1;
        add_res_19166 = add_res_t_res_20552;
        add_res_19167 = add_res_t_res_20553;
    } else {
        // stdlib/bigint.fut:39:8-40:39
        
        int64_t add_res_f_res_19206;
        bool add_res_f_res_19207;
        
        if (cond_19165) {
            if (memblock_alloc(ctx, &mem_20851, bytes_20811, "mem_20851")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_21053 = 0; nest_i_21053 < arithmetic_res_18973; nest_i_21053++) {
                ((int32_t *) mem_20851.mem)[nest_i_21053] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_20560;
            int64_t borrow_20563 = (int64_t) 0;
            
            for (int64_t i_20561 = 0; i_20561 < arithmetic_res_18973; i_20561++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_20564 = ((int32_t *) ext_mem_20797.mem)[i_20561];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_20565 = zext_i32_i64(u32_arg0_20564);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_20566 = slt64(i_20561, negate_res_19118);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_20567;
                
                if (cond_20566) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_20568;
                    
                    if (!cond_20566) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20561, "] out of bounds for array of shape [", (long long) negate_res_19118, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:39:56-71\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:6:7-8:23\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:33:43-47
                    
                    int32_t u32_arg0_20569 = ((int32_t *) ext_mem_20837.mem)[i_20561];
                    
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_20570 = zext_i32_i64(u32_arg0_20569);
                    
                    zp_lhs_20567 = u32_res_20570;
                } else {
                    zp_lhs_20567 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_20571 = add64(borrow_20563, zp_lhs_20567);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_20572 = sub64(u32_res_20565, y_20571);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_20573 = zext_i64_i32(u64_arg0_20572);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_20851.mem)[i_20561] = u64_res_20573;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_20575 = ult64(u32_res_20565, y_20571);
                
                // stdlib/bigint.fut:39:56-71
                
                int64_t tmp_20576 = btoi_bool_i64(cond_20575);
                int64_t borrow_tmp_21055 = tmp_20576;
                
                borrow_20563 = borrow_tmp_21055;
            }
            sub_abs_res_20560 = borrow_20563;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_20577;
            bool sub_abs_res_20578;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20860, &sub_abs_res_20577, &sub_abs_res_20578, mem_20851, arithmetic_res_18973, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_20851, "mem_20851") != 0)
                return 1;
            // stdlib/bigint.fut:39:41-71
            
            int64_t add_res_f_res_t_res_20580;
            bool add_res_f_res_t_res_20581;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20861, &add_res_f_res_t_res_20580, &add_res_f_res_t_res_20581, ext_mem_20860, sub_abs_res_20577, arithmetic_res_18974) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_20860, "ext_mem_20860") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20862, &ext_mem_20861, "ext_mem_20861") != 0)
                return 1;
            add_res_f_res_19206 = add_res_f_res_t_res_20580;
            add_res_f_res_19207 = add_res_f_res_t_res_20581;
        } else {
            if (memblock_alloc(ctx, &mem_20839, bytes_20838, "mem_20839")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_21056 = 0; nest_i_21056 < negate_res_19118; nest_i_21056++) {
                ((int32_t *) mem_20839.mem)[nest_i_21056] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_19236;
            int64_t borrow_19239 = (int64_t) 0;
            
            for (int64_t i_19237 = 0; i_19237 < negate_res_19118; i_19237++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_19240 = ((int32_t *) ext_mem_20837.mem)[i_19237];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_19241 = zext_i32_i64(u32_arg0_19240);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_19242 = slt64(i_19237, arithmetic_res_18973);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_19243;
                
                if (cond_19242) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_20583;
                    
                    if (!cond_19242) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19237, "] out of bounds for array of shape [", (long long) arithmetic_res_18973, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:40:23-38\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:6:7-8:23\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:33:43-47
                    
                    int32_t u32_arg0_20584 = ((int32_t *) ext_mem_20797.mem)[i_19237];
                    
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_20585 = zext_i32_i64(u32_arg0_20584);
                    
                    zp_lhs_19243 = u32_res_20585;
                } else {
                    zp_lhs_19243 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_19247 = add64(borrow_19239, zp_lhs_19243);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_19248 = sub64(u32_res_19241, y_19247);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_19249 = zext_i64_i32(u64_arg0_19248);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_20839.mem)[i_19237] = u64_res_19249;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_19251 = ult64(u32_res_19241, y_19247);
                
                // stdlib/bigint.fut:40:23-38
                
                int64_t tmp_19252 = btoi_bool_i64(cond_19251);
                int64_t borrow_tmp_21058 = tmp_19252;
                
                borrow_19239 = borrow_tmp_21058;
            }
            sub_abs_res_19236 = borrow_19239;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_19253;
            bool sub_abs_res_19254;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20848, &sub_abs_res_19253, &sub_abs_res_19254, mem_20839, negate_res_19118, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_20839, "mem_20839") != 0)
                return 1;
            // stdlib/bigint.fut:40:8-38
            
            int64_t add_res_f_res_f_res_19256;
            bool add_res_f_res_f_res_19257;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20849, &add_res_f_res_f_res_19256, &add_res_f_res_f_res_19257, ext_mem_20848, sub_abs_res_19253, negate_res_19119) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_20848, "ext_mem_20848") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20862, &ext_mem_20849, "ext_mem_20849") != 0)
                return 1;
            add_res_f_res_19206 = add_res_f_res_f_res_19256;
            add_res_f_res_19207 = add_res_f_res_f_res_19257;
        }
        if (memblock_set(ctx, &ext_mem_20875, &ext_mem_20862, "ext_mem_20862") != 0)
            return 1;
        add_res_19166 = add_res_f_res_19206;
        add_res_19167 = add_res_f_res_19207;
    }
    if (memblock_unref(ctx, &ext_mem_20837, "ext_mem_20837") != 0)
        return 1;
    // stdlib/bigint.fut:50:65-78
    
    int64_t normalise_arg1_19259;
    
    if (futrts_mul_abs_11275(ctx, &ext_mem_20876, &normalise_arg1_19259, ext_mem_20797, arithmetic_res_18973, factor_15067) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:50:50-78
    
    int64_t multiply_small_res_19261;
    bool multiply_small_res_19262;
    
    if (futrts_normalise_11258(ctx, &ext_mem_20877, &multiply_small_res_19261, &multiply_small_res_19262, ext_mem_20876, normalise_arg1_19259, arithmetic_res_18974) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &ext_mem_20876, "ext_mem_20876") != 0)
        return 1;
    // stdlib/bigint.fut:70:16-33
    
    int64_t quotient_res_19469;
    bool quotient_res_19470;
    
    if (futrts_normalise_11258(ctx, &ext_mem_20930, &quotient_res_19469, &quotient_res_19470, mem_20881, max_res_19289, 0) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &mem_20881, "mem_20881") != 0)
        return 1;
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool protect_cond_conj_19482 = x_18980 && cond_19476;
    
    // stdlib/bigint.fut:73:39-51
    
    int32_t zeze_lhs_19483;
    
    if (protect_cond_conj_19482) {
        // probes/stdlib-tests.fut:6:7-8:60
        
        int32_t x_20608 = ((int32_t *) ext_mem_20926.mem)[(int64_t) 0];
        
        zeze_lhs_19483 = x_20608;
    } else {
        zeze_lhs_19483 = 0;
    }
    if (memblock_unref(ctx, &ext_mem_20926, "ext_mem_20926") != 0)
        return 1;
    // stdlib/bigint.fut:73:52-56
    
    bool cond_t_res_19485 = zeze_lhs_19483 == 0;
    
    // probes/stdlib-tests.fut:6:7-8:60
    
    bool x_19486 = cond_19476 && cond_t_res_19485;
    
    if (memblock_alloc(ctx, &mem_20931, (int64_t) 8, "mem_20931")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:10:6-69
    
    struct memblock arithmeticzistatic_array_21059 = (struct memblock) {NULL, (unsigned char *) arithmeticzistatic_array_realtype_21092, 0, "arithmetic.static_array_21059"};
    
    // stdlib/bigint.fut:10:6-69
    lmad_copy_4b(ctx, 1, (uint32_t *) mem_20931.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (uint32_t *) arithmeticzistatic_array_21059.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {(int64_t) 2});
    // stdlib/bigint.fut:41:34-54
    
    bool normalise_arg_19488 = !quotient_res_19470;
    int64_t bytes_20945 = (int64_t) 4 * quotient_res_19469;
    
    // stdlib/bigint.fut:52:3-74:42
    
    int64_t quotient_res_19489;
    bool quotient_res_19490;
    
    if (cond_18979) {
        if (memblock_set(ctx, &ext_mem_20974, &ext_mem_20930, "ext_mem_20930") != 0)
            return 1;
        quotient_res_19489 = quotient_res_19469;
        quotient_res_19490 = quotient_res_19470;
    } else {
        // stdlib/bigint.fut:73:11-74:42
        
        int64_t quotient_res_f_res_19492;
        bool quotient_res_f_res_19493;
        
        if (x_19486) {
            // stdlib/bigint.fut:41:34-54
            
            int64_t negate_res_20609;
            bool negate_res_20610;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20972, &negate_res_20609, &negate_res_20610, ext_mem_20930, quotient_res_19469, normalise_arg_19488) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_20973, &ext_mem_20972, "ext_mem_20972") != 0)
                return 1;
            quotient_res_f_res_19492 = negate_res_20609;
            quotient_res_f_res_19493 = negate_res_20610;
        } else {
            // stdlib/bigint.fut:10:6-69
            
            int64_t integer_res_19498;
            bool integer_res_19499;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20932, &integer_res_19498, &integer_res_19499, mem_20931, (int64_t) 2, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:38:3-40:39
            
            bool cond_19501 = quotient_res_19470 == integer_res_19499;
            
            // stdlib/bigint.fut:70:7-74:40
            
            bool x_19502 = !cond_19501;
            
            // stdlib/bigint.fut:21:14-42
            
            int64_t max_res_19549 = smax64(quotient_res_19469, integer_res_19498);
            
            // stdlib/bigint.fut:22:52-58
            
            int64_t dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19550 = add64((int64_t) 1, max_res_19549);
            
            // stdlib/bigint.fut:28:7-32
            
            bool x_19575 = sle64((int64_t) 0, max_res_19549);
            
            // stdlib/bigint.fut:28:7-32
            
            bool y_19576 = slt64(max_res_19549, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19550);
            
            // stdlib/bigint.fut:28:7-32
            
            bool bounds_check_19577 = x_19575 && y_19576;
            bool protect_assert_disj_19693 = x_19502 || bounds_check_19577;
            
            // stdlib/bigint.fut:28:7-32
            
            bool index_certs_19578;
            
            if (!protect_assert_disj_19693) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) max_res_19549, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19550, "].", "-> #0  stdlib/bigint.fut:28:7-32\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:6:7-8:60\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/bigint.fut:39:11-30
            
            int64_t zgze_lhs_19503;
            
            if (x_19502) {
                // stdlib/bigint.fut:12:3-16:17
                
                bool cond_20635 = quotient_res_19469 == integer_res_19498;
                
                // stdlib/bigint.fut:12:3-16:17
                
                bool cond_20636 = !cond_20635;
                
                // stdlib/bigint.fut:13:53-55
                
                int64_t tmp_20637 = sub64(quotient_res_19469, (int64_t) 1);
                
                // stdlib/bigint.fut:13:26-15:74
                
                bool cond_20638 = sle64((int64_t) 0, tmp_20637);
                int64_t compare_abs_res_f_res_20639;
                
                if (cond_20635) {
                    bool x_20640;
                    int64_t x_20641;
                    int64_t x_20642;
                    bool loop_while_20643;
                    int64_t i_20644;
                    int64_t result_20645;
                    
                    loop_while_20643 = cond_20638;
                    i_20644 = tmp_20637;
                    result_20645 = (int64_t) 0;
                    while (loop_while_20643) {
                        // stdlib/bigint.fut:15:20-24
                        
                        bool x_20647 = sle64((int64_t) 0, i_20644);
                        
                        // stdlib/bigint.fut:15:20-24
                        
                        bool y_20648 = slt64(i_20644, quotient_res_19469);
                        
                        // stdlib/bigint.fut:15:20-24
                        
                        bool bounds_check_20649 = x_20647 && y_20648;
                        
                        // stdlib/bigint.fut:15:20-24
                        
                        bool index_certs_20650;
                        
                        if (!bounds_check_20649) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20644, "] out of bounds for array of shape [", (long long) quotient_res_19469, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:70:7-74:40\n   #2  probes/stdlib-tests.fut:6:7-8:60\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/bigint.fut:15:27-31
                        
                        bool y_20652 = slt64(i_20644, integer_res_19498);
                        
                        // stdlib/bigint.fut:15:27-31
                        
                        bool bounds_check_20653 = x_20647 && y_20652;
                        
                        // stdlib/bigint.fut:15:27-31
                        
                        bool index_certs_20654;
                        
                        if (!bounds_check_20653) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20644, "] out of bounds for array of shape [", (long long) integer_res_19498, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:70:7-74:40\n   #2  probes/stdlib-tests.fut:6:7-8:60\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/bigint.fut:15:14-16
                        
                        int64_t tmp_20646 = sub64(i_20644, (int64_t) 1);
                        
                        // stdlib/bigint.fut:15:20-24
                        
                        int32_t zl_lhs_20651 = ((int32_t *) ext_mem_20930.mem)[i_20644];
                        
                        // stdlib/bigint.fut:15:27-31
                        
                        int32_t zl_rhs_20655 = ((int32_t *) ext_mem_20932.mem)[i_20644];
                        
                        // stdlib/bigint.fut:15:17-73
                        
                        bool cond_20656 = ult32(zl_lhs_20651, zl_rhs_20655);
                        
                        // stdlib/bigint.fut:15:17-73
                        
                        int64_t tmp_20657;
                        
                        if (cond_20656) {
                            tmp_20657 = (int64_t) -1;
                        } else {
                            // stdlib/bigint.fut:15:45-73
                            
                            bool cond_20658 = ult32(zl_rhs_20655, zl_lhs_20651);
                            
                            // stdlib/bigint.fut:70:7-74:40
                            
                            int64_t tmp_f_res_20659 = btoi_bool_i64(cond_20658);
                            
                            tmp_20657 = tmp_f_res_20659;
                        }
                        // stdlib/bigint.fut:14:16-37
                        
                        bool cond_20660 = sle64((int64_t) 0, tmp_20646);
                        
                        // stdlib/bigint.fut:14:33-37
                        
                        bool loop_cond_t_res_20661 = tmp_20657 == (int64_t) 0;
                        
                        // stdlib/bigint.fut:70:7-74:40
                        
                        bool x_20662 = cond_20660 && loop_cond_t_res_20661;
                        bool loop_while_tmp_21060 = x_20662;
                        int64_t i_tmp_21061 = tmp_20646;
                        int64_t result_tmp_21062 = tmp_20657;
                        
                        loop_while_20643 = loop_while_tmp_21060;
                        i_20644 = i_tmp_21061;
                        result_20645 = result_tmp_21062;
                    }
                    x_20640 = loop_while_20643;
                    x_20641 = i_20644;
                    x_20642 = result_20645;
                    compare_abs_res_f_res_20639 = x_20642;
                } else {
                    compare_abs_res_f_res_20639 = (int64_t) 0;
                }
                // stdlib/bigint.fut:12:32-69
                
                bool cond_20663 = slt64(quotient_res_19469, integer_res_19498);
                
                // stdlib/bigint.fut:12:32-69
                
                int64_t compare_abs_res_t_res_20664;
                
                if (cond_20663) {
                    compare_abs_res_t_res_20664 = (int64_t) -1;
                } else {
                    compare_abs_res_t_res_20664 = (int64_t) 1;
                }
                // stdlib/bigint.fut:12:3-16:17
                
                int64_t compare_abs_res_20665;
                
                if (cond_20636) {
                    compare_abs_res_20665 = compare_abs_res_t_res_20664;
                } else {
                    compare_abs_res_20665 = compare_abs_res_f_res_20639;
                }
                zgze_lhs_19503 = compare_abs_res_20665;
            } else {
                zgze_lhs_19503 = (int64_t) 0;
            }
            // stdlib/bigint.fut:39:8-40:39
            
            bool cond_19545 = sle64((int64_t) 0, zgze_lhs_19503);
            int64_t bytes_20933 = (int64_t) 4 * integer_res_19498;
            int64_t bytes_20958 = (int64_t) 4 * dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19550;
            
            // stdlib/bigint.fut:38:3-40:39
            
            int64_t add_res_19546;
            bool add_res_19547;
            
            if (cond_19501) {
                if (memblock_alloc(ctx, &mem_20959, bytes_20958, "mem_20959")) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/bigint.fut:22:41-64
                for (int64_t nest_i_21063 = 0; nest_i_21063 < dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19550; nest_i_21063++) {
                    ((int32_t *) mem_20959.mem)[nest_i_21063] = 0;
                }
                // stdlib/bigint.fut:22:21-27:23
                
                int64_t add_abs_res_20674;
                int64_t carry_20677 = (int64_t) 0;
                
                for (int64_t i_20675 = 0; i_20675 < max_res_19549; i_20675++) {
                    // stdlib/bigint.fut:26:9-29
                    
                    bool y_20691 = slt64(i_20675, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19550);
                    
                    // stdlib/bigint.fut:26:9-29
                    
                    bool index_certs_20692;
                    
                    if (!y_20691) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20675, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19550, "].", "-> #0  stdlib/bigint.fut:26:9-29\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:6:7-8:60\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:23:13-53
                    
                    bool cond_20678 = slt64(i_20675, quotient_res_19469);
                    
                    // stdlib/bigint.fut:23:13-53
                    
                    int64_t x_20679;
                    
                    if (cond_20678) {
                        // stdlib/bigint.fut:23:42-46
                        
                        bool index_certs_20680;
                        
                        if (!cond_20678) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20675, "] out of bounds for array of shape [", (long long) quotient_res_19469, "].", "-> #0  stdlib/bigint.fut:23:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:6:7-8:60\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/bigint.fut:23:42-46
                        
                        int32_t u32_arg0_20681 = ((int32_t *) ext_mem_20930.mem)[i_20675];
                        
                        // stdlib/bigint.fut:23:34-46
                        
                        int64_t u32_res_20682 = zext_i32_i64(u32_arg0_20681);
                        
                        x_20679 = u32_res_20682;
                    } else {
                        x_20679 = (int64_t) 0;
                    }
                    // stdlib/bigint.fut:24:13-53
                    
                    bool cond_20683 = slt64(i_20675, integer_res_19498);
                    
                    // stdlib/bigint.fut:24:13-53
                    
                    int64_t y_20684;
                    
                    if (cond_20683) {
                        // stdlib/bigint.fut:24:42-46
                        
                        bool index_certs_20685;
                        
                        if (!cond_20683) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20675, "] out of bounds for array of shape [", (long long) integer_res_19498, "].", "-> #0  stdlib/bigint.fut:24:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:6:7-8:60\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/bigint.fut:24:42-46
                        
                        int32_t u32_arg0_20686 = ((int32_t *) ext_mem_20932.mem)[i_20675];
                        
                        // stdlib/bigint.fut:24:34-46
                        
                        int64_t u32_res_20687 = zext_i32_i64(u32_arg0_20686);
                        
                        y_20684 = u32_res_20687;
                    } else {
                        y_20684 = (int64_t) 0;
                    }
                    // stdlib/bigint.fut:25:16-18
                    
                    int64_t zp_lhs_20688 = add64(x_20679, y_20684);
                    
                    // stdlib/bigint.fut:25:18-24
                    
                    int64_t sum_20689 = add64(carry_20677, zp_lhs_20688);
                    
                    // stdlib/bigint.fut:26:18-29
                    
                    int32_t u64_res_20690 = zext_i64_i32(sum_20689);
                    
                    // stdlib/bigint.fut:26:9-29
                    ((int32_t *) mem_20959.mem)[i_20675] = u64_res_20690;
                    // stdlib/bigint.fut:27:17-22
                    
                    int64_t tmp_20694 = lshr64(sum_20689, (int64_t) 32);
                    int64_t carry_tmp_21065 = tmp_20694;
                    
                    carry_20677 = carry_tmp_21065;
                }
                add_abs_res_20674 = carry_20677;
                // stdlib/bigint.fut:28:19-32
                
                int32_t u64_res_20695 = zext_i64_i32(add_abs_res_20674);
                
                // stdlib/bigint.fut:28:7-32
                ((int32_t *) mem_20959.mem)[max_res_19549] = u64_res_20695;
                // stdlib/bigint.fut:29:7-26
                
                int64_t add_abs_res_20697;
                bool add_abs_res_20698;
                
                if (futrts_normalise_11258(ctx, &ext_mem_20968, &add_abs_res_20697, &add_abs_res_20698, mem_20959, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_19550, 0) != 0) {
                    err = 1;
                    goto cleanup;
                }
                if (memblock_unref(ctx, &mem_20959, "mem_20959") != 0)
                    return 1;
                // stdlib/bigint.fut:38:22-52
                
                int64_t add_res_t_res_20700;
                bool add_res_t_res_20701;
                
                if (futrts_normalise_11258(ctx, &ext_mem_20969, &add_res_t_res_20700, &add_res_t_res_20701, ext_mem_20968, add_abs_res_20697, quotient_res_19470) != 0) {
                    err = 1;
                    goto cleanup;
                }
                if (memblock_unref(ctx, &ext_mem_20968, "ext_mem_20968") != 0)
                    return 1;
                if (memblock_set(ctx, &ext_mem_20970, &ext_mem_20969, "ext_mem_20969") != 0)
                    return 1;
                add_res_19546 = add_res_t_res_20700;
                add_res_19547 = add_res_t_res_20701;
            } else {
                // stdlib/bigint.fut:39:8-40:39
                
                int64_t add_res_f_res_19586;
                bool add_res_f_res_19587;
                
                if (cond_19545) {
                    if (memblock_alloc(ctx, &mem_20946, bytes_20945, "mem_20946")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:31:44-69
                    for (int64_t nest_i_21066 = 0; nest_i_21066 < quotient_res_19469; nest_i_21066++) {
                        ((int32_t *) mem_20946.mem)[nest_i_21066] = 0;
                    }
                    // stdlib/bigint.fut:31:23-35:39
                    
                    int64_t sub_abs_res_20708;
                    int64_t borrow_20711 = (int64_t) 0;
                    
                    for (int64_t i_20709 = 0; i_20709 < quotient_res_19469; i_20709++) {
                        // stdlib/bigint.fut:32:21-25
                        
                        int32_t u32_arg0_20712 = ((int32_t *) ext_mem_20930.mem)[i_20709];
                        
                        // stdlib/bigint.fut:32:13-25
                        
                        int64_t u32_res_20713 = zext_i32_i64(u32_arg0_20712);
                        
                        // stdlib/bigint.fut:33:14-54
                        
                        bool cond_20714 = slt64(i_20709, integer_res_19498);
                        
                        // stdlib/bigint.fut:33:14-54
                        
                        int64_t zp_lhs_20715;
                        
                        if (cond_20714) {
                            // stdlib/bigint.fut:33:43-47
                            
                            bool index_certs_20716;
                            
                            if (!cond_20714) {
                                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20709, "] out of bounds for array of shape [", (long long) integer_res_19498, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:39:56-71\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:6:7-8:60\n"));
                                err = FUTHARK_PROGRAM_ERROR;
                                goto cleanup;
                            }
                            // stdlib/bigint.fut:33:43-47
                            
                            int32_t u32_arg0_20717 = ((int32_t *) ext_mem_20932.mem)[i_20709];
                            
                            // stdlib/bigint.fut:33:35-47
                            
                            int64_t u32_res_20718 = zext_i32_i64(u32_arg0_20717);
                            
                            zp_lhs_20715 = u32_res_20718;
                        } else {
                            zp_lhs_20715 = (int64_t) 0;
                        }
                        // stdlib/bigint.fut:33:55-62
                        
                        int64_t y_20719 = add64(borrow_20711, zp_lhs_20715);
                        
                        // stdlib/bigint.fut:34:28-30
                        
                        int64_t u64_arg0_20720 = sub64(u32_res_20713, y_20719);
                        
                        // stdlib/bigint.fut:34:18-30
                        
                        int32_t u64_res_20721 = zext_i64_i32(u64_arg0_20720);
                        
                        // stdlib/bigint.fut:34:9-31
                        ((int32_t *) mem_20946.mem)[i_20709] = u64_res_20721;
                        // stdlib/bigint.fut:35:13-38
                        
                        bool cond_20723 = ult64(u32_res_20713, y_20719);
                        
                        // stdlib/bigint.fut:39:56-71
                        
                        int64_t tmp_20724 = btoi_bool_i64(cond_20723);
                        int64_t borrow_tmp_21068 = tmp_20724;
                        
                        borrow_20711 = borrow_tmp_21068;
                    }
                    sub_abs_res_20708 = borrow_20711;
                    // stdlib/bigint.fut:36:7-26
                    
                    int64_t sub_abs_res_20725;
                    bool sub_abs_res_20726;
                    
                    if (futrts_normalise_11258(ctx, &ext_mem_20955, &sub_abs_res_20725, &sub_abs_res_20726, mem_20946, quotient_res_19469, 0) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &mem_20946, "mem_20946") != 0)
                        return 1;
                    // stdlib/bigint.fut:39:41-71
                    
                    int64_t add_res_f_res_t_res_20728;
                    bool add_res_f_res_t_res_20729;
                    
                    if (futrts_normalise_11258(ctx, &ext_mem_20956, &add_res_f_res_t_res_20728, &add_res_f_res_t_res_20729, ext_mem_20955, sub_abs_res_20725, quotient_res_19470) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &ext_mem_20955, "ext_mem_20955") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_20957, &ext_mem_20956, "ext_mem_20956") != 0)
                        return 1;
                    add_res_f_res_19586 = add_res_f_res_t_res_20728;
                    add_res_f_res_19587 = add_res_f_res_t_res_20729;
                } else {
                    if (memblock_alloc(ctx, &mem_20934, bytes_20933, "mem_20934")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:31:44-69
                    for (int64_t nest_i_21069 = 0; nest_i_21069 < integer_res_19498; nest_i_21069++) {
                        ((int32_t *) mem_20934.mem)[nest_i_21069] = 0;
                    }
                    // stdlib/bigint.fut:31:23-35:39
                    
                    int64_t sub_abs_res_19616;
                    int64_t borrow_19619 = (int64_t) 0;
                    
                    for (int64_t i_19617 = 0; i_19617 < integer_res_19498; i_19617++) {
                        // stdlib/bigint.fut:32:21-25
                        
                        int32_t u32_arg0_19620 = ((int32_t *) ext_mem_20932.mem)[i_19617];
                        
                        // stdlib/bigint.fut:32:13-25
                        
                        int64_t u32_res_19621 = zext_i32_i64(u32_arg0_19620);
                        
                        // stdlib/bigint.fut:33:14-54
                        
                        bool cond_19622 = slt64(i_19617, quotient_res_19469);
                        
                        // stdlib/bigint.fut:33:14-54
                        
                        int64_t zp_lhs_19623;
                        
                        if (cond_19622) {
                            // stdlib/bigint.fut:33:43-47
                            
                            bool index_certs_20731;
                            
                            if (!cond_19622) {
                                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19617, "] out of bounds for array of shape [", (long long) quotient_res_19469, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:40:23-38\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:6:7-8:60\n"));
                                err = FUTHARK_PROGRAM_ERROR;
                                goto cleanup;
                            }
                            // stdlib/bigint.fut:33:43-47
                            
                            int32_t u32_arg0_20732 = ((int32_t *) ext_mem_20930.mem)[i_19617];
                            
                            // stdlib/bigint.fut:33:35-47
                            
                            int64_t u32_res_20733 = zext_i32_i64(u32_arg0_20732);
                            
                            zp_lhs_19623 = u32_res_20733;
                        } else {
                            zp_lhs_19623 = (int64_t) 0;
                        }
                        // stdlib/bigint.fut:33:55-62
                        
                        int64_t y_19627 = add64(borrow_19619, zp_lhs_19623);
                        
                        // stdlib/bigint.fut:34:28-30
                        
                        int64_t u64_arg0_19628 = sub64(u32_res_19621, y_19627);
                        
                        // stdlib/bigint.fut:34:18-30
                        
                        int32_t u64_res_19629 = zext_i64_i32(u64_arg0_19628);
                        
                        // stdlib/bigint.fut:34:9-31
                        ((int32_t *) mem_20934.mem)[i_19617] = u64_res_19629;
                        // stdlib/bigint.fut:35:13-38
                        
                        bool cond_19631 = ult64(u32_res_19621, y_19627);
                        
                        // stdlib/bigint.fut:40:23-38
                        
                        int64_t tmp_19632 = btoi_bool_i64(cond_19631);
                        int64_t borrow_tmp_21071 = tmp_19632;
                        
                        borrow_19619 = borrow_tmp_21071;
                    }
                    sub_abs_res_19616 = borrow_19619;
                    // stdlib/bigint.fut:36:7-26
                    
                    int64_t sub_abs_res_19633;
                    bool sub_abs_res_19634;
                    
                    if (futrts_normalise_11258(ctx, &ext_mem_20943, &sub_abs_res_19633, &sub_abs_res_19634, mem_20934, integer_res_19498, 0) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &mem_20934, "mem_20934") != 0)
                        return 1;
                    // stdlib/bigint.fut:40:8-38
                    
                    int64_t add_res_f_res_f_res_19636;
                    bool add_res_f_res_f_res_19637;
                    
                    if (futrts_normalise_11258(ctx, &ext_mem_20944, &add_res_f_res_f_res_19636, &add_res_f_res_f_res_19637, ext_mem_20943, sub_abs_res_19633, integer_res_19499) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &ext_mem_20943, "ext_mem_20943") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_20957, &ext_mem_20944, "ext_mem_20944") != 0)
                        return 1;
                    add_res_f_res_19586 = add_res_f_res_f_res_19636;
                    add_res_f_res_19587 = add_res_f_res_f_res_19637;
                }
                if (memblock_set(ctx, &ext_mem_20970, &ext_mem_20957, "ext_mem_20957") != 0)
                    return 1;
                add_res_19546 = add_res_f_res_19586;
                add_res_19547 = add_res_f_res_19587;
            }
            if (memblock_unref(ctx, &ext_mem_20932, "ext_mem_20932") != 0)
                return 1;
            // stdlib/bigint.fut:41:34-54
            
            bool normalise_arg_19639 = !add_res_19547;
            
            // stdlib/bigint.fut:41:34-54
            
            int64_t negate_res_19640;
            bool negate_res_19641;
            
            if (futrts_normalise_11258(ctx, &ext_mem_20971, &negate_res_19640, &negate_res_19641, ext_mem_20970, add_res_19546, normalise_arg_19639) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_20970, "ext_mem_20970") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20973, &ext_mem_20971, "ext_mem_20971") != 0)
                return 1;
            quotient_res_f_res_19492 = negate_res_19640;
            quotient_res_f_res_19493 = negate_res_19641;
        }
        if (memblock_set(ctx, &ext_mem_20974, &ext_mem_20973, "ext_mem_20973") != 0)
            return 1;
        quotient_res_19489 = quotient_res_f_res_19492;
        quotient_res_19490 = quotient_res_f_res_19493;
    }
    if (memblock_unref(ctx, &ext_mem_20930, "ext_mem_20930") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_20931, "mem_20931") != 0)
        return 1;
    // stdlib/bigint.fut:18:3-19:51
    
    int64_t compare_res_19645;
    
    if (x_18980) {
        // stdlib/bigint.fut:18:22-43
        
        int64_t compare_res_t_res_20734;
        
        if (arithmetic_res_18974) {
            compare_res_t_res_20734 = (int64_t) -1;
        } else {
            compare_res_t_res_20734 = (int64_t) 1;
        }
        compare_res_19645 = compare_res_t_res_20734;
    } else {
        // stdlib/bigint.fut:19:9-30
        
        int64_t zt_lhs_19647;
        
        if (arithmetic_res_18974) {
            zt_lhs_19647 = (int64_t) -1;
        } else {
            zt_lhs_19647 = (int64_t) 1;
        }
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_19648 = arithmetic_res_18973 == arithmetic_res_18976;
        
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_19649 = !cond_19648;
        
        // stdlib/bigint.fut:12:3-16:17
        
        int64_t compare_abs_res_19650;
        
        if (cond_19649) {
            // stdlib/bigint.fut:12:32-69
            
            bool cond_20735 = slt64(arithmetic_res_18973, arithmetic_res_18976);
            
            // stdlib/bigint.fut:12:32-69
            
            int64_t compare_abs_res_t_res_20736;
            
            if (cond_20735) {
                compare_abs_res_t_res_20736 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_20736 = (int64_t) 1;
            }
            compare_abs_res_19650 = compare_abs_res_t_res_20736;
        } else {
            // stdlib/bigint.fut:13:53-55
            
            int64_t tmp_19653 = sub64(arithmetic_res_18973, (int64_t) 1);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool cond_19654 = sle64((int64_t) 0, tmp_19653);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool compare_abs_res_f_res_19655;
            int64_t compare_abs_res_f_res_19656;
            int64_t compare_abs_res_f_res_19657;
            bool loop_while_19658;
            int64_t i_19659;
            int64_t result_19660;
            
            loop_while_19658 = cond_19654;
            i_19659 = tmp_19653;
            result_19660 = (int64_t) 0;
            while (loop_while_19658) {
                // stdlib/bigint.fut:15:20-24
                
                bool x_19662 = sle64((int64_t) 0, i_19659);
                
                // stdlib/bigint.fut:15:20-24
                
                bool y_19663 = slt64(i_19659, arithmetic_res_18973);
                
                // stdlib/bigint.fut:15:20-24
                
                bool bounds_check_19664 = x_19662 && y_19663;
                
                // stdlib/bigint.fut:15:20-24
                
                bool index_certs_19665;
                
                if (!bounds_check_19664) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19659, "] out of bounds for array of shape [", (long long) arithmetic_res_18973, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:19:32-51\n   #2  probes/stdlib-tests.fut:6:7-8:72\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:27-31
                
                bool y_19668 = slt64(i_19659, arithmetic_res_18976);
                
                // stdlib/bigint.fut:15:27-31
                
                bool bounds_check_19669 = x_19662 && y_19668;
                
                // stdlib/bigint.fut:15:27-31
                
                bool index_certs_19670;
                
                if (!bounds_check_19669) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19659, "] out of bounds for array of shape [", (long long) arithmetic_res_18976, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:19:32-51\n   #2  probes/stdlib-tests.fut:6:7-8:72\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:14-16
                
                int64_t tmp_19661 = sub64(i_19659, (int64_t) 1);
                
                // stdlib/bigint.fut:15:20-24
                
                int32_t zl_lhs_19666 = ((int32_t *) ext_mem_20797.mem)[i_19659];
                
                // stdlib/bigint.fut:15:27-31
                
                int32_t zl_rhs_19671 = ((int32_t *) ext_mem_20798.mem)[i_19659];
                
                // stdlib/bigint.fut:15:17-73
                
                bool cond_19672 = ult32(zl_lhs_19666, zl_rhs_19671);
                
                // stdlib/bigint.fut:15:17-73
                
                int64_t tmp_19673;
                
                if (cond_19672) {
                    tmp_19673 = (int64_t) -1;
                } else {
                    // stdlib/bigint.fut:15:45-73
                    
                    bool cond_19684 = ult32(zl_rhs_19671, zl_lhs_19666);
                    
                    // stdlib/bigint.fut:19:32-51
                    
                    int64_t tmp_f_res_19685 = btoi_bool_i64(cond_19684);
                    
                    tmp_19673 = tmp_f_res_19685;
                }
                // stdlib/bigint.fut:14:16-37
                
                bool cond_19686 = sle64((int64_t) 0, tmp_19661);
                
                // stdlib/bigint.fut:14:33-37
                
                bool loop_cond_t_res_19687 = tmp_19673 == (int64_t) 0;
                
                // stdlib/bigint.fut:19:32-51
                
                bool x_19688 = cond_19686 && loop_cond_t_res_19687;
                bool loop_while_tmp_21072 = x_19688;
                int64_t i_tmp_21073 = tmp_19661;
                int64_t result_tmp_21074 = tmp_19673;
                
                loop_while_19658 = loop_while_tmp_21072;
                i_19659 = i_tmp_21073;
                result_19660 = result_tmp_21074;
            }
            compare_abs_res_f_res_19655 = loop_while_19658;
            compare_abs_res_f_res_19656 = i_19659;
            compare_abs_res_f_res_19657 = result_19660;
            compare_abs_res_19650 = compare_abs_res_f_res_19657;
        }
        // stdlib/bigint.fut:19:31-51
        
        int64_t compare_res_f_res_19689 = mul64(zt_lhs_19647, compare_abs_res_19650);
        
        compare_res_19645 = compare_res_f_res_19689;
    }
    if (memblock_unref(ctx, &ext_mem_20797, "ext_mem_20797") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20998, &ext_mem_20836, "ext_mem_20836") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20999, &ext_mem_20875, "ext_mem_20875") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_21000, &ext_mem_20877, "ext_mem_20877") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_21001, &ext_mem_20974, "ext_mem_20974") != 0)
        return 1;
    prim_out_21002 = add_res_19024;
    prim_out_21003 = add_res_19166;
    prim_out_21004 = multiply_small_res_19261;
    prim_out_21005 = quotient_res_19489;
    prim_out_21006 = add_res_19025;
    prim_out_21007 = add_res_19167;
    prim_out_21008 = multiply_small_res_19262;
    prim_out_21009 = quotient_res_19490;
    prim_out_21010 = compare_res_19645;
    if (memblock_set(ctx, &*mem_out_p_21079, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21080, &mem_out_20999, "mem_out_20999") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21081, &mem_out_21000, "mem_out_21000") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21082, &mem_out_21001, "mem_out_21001") != 0)
        return 1;
    *out_prim_out_21083 = prim_out_21002;
    *out_prim_out_21084 = prim_out_21003;
    *out_prim_out_21085 = prim_out_21004;
    *out_prim_out_21086 = prim_out_21005;
    *out_prim_out_21087 = prim_out_21006;
    *out_prim_out_21088 = prim_out_21007;
    *out_prim_out_21089 = prim_out_21008;
    *out_prim_out_21090 = prim_out_21009;
    *out_prim_out_21091 = prim_out_21010;
    
  cleanup:
    {
        if (memblock_unref(ctx, &ext_mem_20971, "ext_mem_20971") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20944, "ext_mem_20944") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20943, "ext_mem_20943") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20934, "mem_20934") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20956, "ext_mem_20956") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20955, "ext_mem_20955") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20946, "mem_20946") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20957, "ext_mem_20957") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20969, "ext_mem_20969") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20968, "ext_mem_20968") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20959, "mem_20959") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20970, "ext_mem_20970") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20932, "ext_mem_20932") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20972, "ext_mem_20972") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20973, "ext_mem_20973") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20974, "ext_mem_20974") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20931, "mem_20931") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20930, "ext_mem_20930") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20877, "ext_mem_20877") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20876, "ext_mem_20876") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20849, "ext_mem_20849") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20848, "ext_mem_20848") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20839, "mem_20839") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20861, "ext_mem_20861") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20860, "ext_mem_20860") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20851, "mem_20851") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20862, "ext_mem_20862") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20874, "ext_mem_20874") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20873, "ext_mem_20873") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20864, "mem_20864") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20875, "ext_mem_20875") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20810, "ext_mem_20810") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20809, "ext_mem_20809") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20800, "mem_20800") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20822, "ext_mem_20822") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20821, "ext_mem_20821") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20812, "mem_20812") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20823, "ext_mem_20823") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20835, "ext_mem_20835") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20834, "ext_mem_20834") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20825, "mem_20825") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20836, "ext_mem_20836") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21012, "mem_param_tmp_21012") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20921, "ext_mem_20921") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20912, "mem_20912") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21021, "mem_param_tmp_21021") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20907, "ext_mem_20907") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20898, "mem_20898") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20896, "mem_param_20896") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20910, "ext_mem_20910") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20893, "ext_mem_20893") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20892, "mem_20892") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20890, "ext_mem_20890") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20887, "mem_param_20887") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20926, "ext_mem_20926") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20881, "mem_20881") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20879, "ext_mem_20879") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20878, "ext_mem_20878") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20837, "ext_mem_20837") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20797, "ext_mem_20797") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_21001, "mem_out_21001") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_21000, "mem_out_21000") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20999, "mem_out_20999") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_hashmap_increment(struct futhark_context *ctx, struct memblock *mem_out_p_21093, struct memblock keys_mem_20795, struct memblock deltas_mem_20796, int64_t dz2080U_17344, int64_t dz2081U_17345)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_20837;
    
    mem_20837.references = NULL;
    
    struct memblock mem_param_tmp_21001;
    
    mem_param_tmp_21001.references = NULL;
    
    struct memblock mem_param_tmp_21000;
    
    mem_param_tmp_21000.references = NULL;
    
    struct memblock mem_param_tmp_20999;
    
    mem_param_tmp_20999.references = NULL;
    
    struct memblock ext_mem_20815;
    
    ext_mem_20815.references = NULL;
    
    struct memblock ext_mem_20816;
    
    ext_mem_20816.references = NULL;
    
    struct memblock ext_mem_20817;
    
    ext_mem_20817.references = NULL;
    
    struct memblock ext_mem_20820;
    
    ext_mem_20820.references = NULL;
    
    struct memblock ext_mem_20823;
    
    ext_mem_20823.references = NULL;
    
    struct memblock ext_mem_20826;
    
    ext_mem_20826.references = NULL;
    
    struct memblock mem_param_20808;
    
    mem_param_20808.references = NULL;
    
    struct memblock mem_param_20805;
    
    mem_param_20805.references = NULL;
    
    struct memblock mem_param_20802;
    
    mem_param_20802.references = NULL;
    
    struct memblock ext_mem_20833;
    
    ext_mem_20833.references = NULL;
    
    struct memblock ext_mem_20834;
    
    ext_mem_20834.references = NULL;
    
    struct memblock ext_mem_20835;
    
    ext_mem_20835.references = NULL;
    
    struct memblock ext_mem_20797;
    
    ext_mem_20797.references = NULL;
    
    struct memblock ext_mem_20798;
    
    ext_mem_20798.references = NULL;
    
    struct memblock ext_mem_20799;
    
    ext_mem_20799.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t bytes_20836 = (int64_t) 8 * dz2080U_17344;
    
    // probes/stdlib-tests.fut:18:28-35
    
    int64_t hashmap_increment_res_18060;
    
    if (futrts_empty_7765(ctx, &ext_mem_20799, &ext_mem_20798, &ext_mem_20797, &hashmap_increment_res_18060, (int64_t) 2) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:18:15-92
    
    int64_t hashmap_increment_res_18061;
    int64_t hashmap_increment_res_18065;
    int64_t loop_dz2082U_18067;
    int64_t table_18071;
    
    if (memblock_set(ctx, &mem_param_20802, &ext_mem_20799, "ext_mem_20799") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_20805, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_20808, &ext_mem_20797, "ext_mem_20797") != 0)
        return 1;
    loop_dz2082U_18067 = (int64_t) 2;
    table_18071 = hashmap_increment_res_18060;
    for (int64_t i_18066 = 0; i_18066 < dz2080U_17344; i_18066++) {
        // probes/stdlib-tests.fut:18:83-92
        
        bool y_18072 = slt64(i_18066, dz2081U_17345);
        
        // probes/stdlib-tests.fut:18:83-92
        
        bool index_certs_18073;
        
        if (!y_18072) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18066, "] out of bounds for array of shape [", (long long) dz2081U_17345, "].", "-> #0  probes/stdlib-tests.fut:18:83-92\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // probes/stdlib-tests.fut:18:75-82
        
        int64_t increment_arg1_18077 = ((int64_t *) keys_mem_20795.mem)[i_18066];
        
        // stdlib/hashmap.fut:34:18-20
        
        int64_t zt_lhs_18078 = add64((int64_t) 1, table_18071);
        
        // stdlib/hashmap.fut:34:21-24
        
        int64_t zg_lhs_18079 = mul64((int64_t) 10, zt_lhs_18078);
        
        // stdlib/hashmap.fut:34:37-39
        
        int64_t zg_rhs_18080 = mul64((int64_t) 7, loop_dz2082U_18067);
        
        // stdlib/hashmap.fut:34:11-70
        
        bool cond_18081 = slt64(zg_rhs_18080, zg_lhs_18079);
        int64_t increment_res_18082;
        int64_t increment_res_18086;
        
        if (cond_18081) {
            // stdlib/hashmap.fut:33:16-34:61
            
            int64_t increment_res_t_res_20386;
            int64_t increment_res_t_res_20390;
            
            if (futrts_grow_11344(ctx, &ext_mem_20817, &ext_mem_20816, &ext_mem_20815, &increment_res_t_res_20386, &increment_res_t_res_20390, mem_param_20802, mem_param_20805, mem_param_20808, loop_dz2082U_18067, table_18071) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_20826, &ext_mem_20817, "ext_mem_20817") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20823, &ext_mem_20816, "ext_mem_20816") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20820, &ext_mem_20815, "ext_mem_20815") != 0)
                return 1;
            increment_res_18082 = increment_res_t_res_20386;
            increment_res_18086 = increment_res_t_res_20390;
        } else {
            if (memblock_set(ctx, &ext_mem_20826, &mem_param_20802, "mem_param_20802") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20823, &mem_param_20805, "mem_param_20805") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20820, &mem_param_20808, "mem_param_20808") != 0)
                return 1;
            increment_res_18082 = loop_dz2082U_18067;
            increment_res_18086 = table_18071;
        }
        // stdlib/hashmap.fut:11:26-28
        
        int64_t mask_20329 = sub64(increment_res_18082, (int64_t) 1);
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_20330 = lshr64(increment_arg1_18077, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_20331 = increment_arg1_18077 ^ zc_rhs_20330;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_20332 = mul64((int64_t) -4658895280553007687, zt_lhs_20331);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_20333 = lshr64(x_20332, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_20334 = x_20332 ^ zc_rhs_20333;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_20335 = mul64((int64_t) -7723592293110705685, zt_lhs_20334);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_20336 = lshr64(x_20335, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_20337 = x_20335 ^ zc_rhs_20336;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_20338 = mask_20329 & hash_res_20337;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20339 = ((bool *) ext_mem_20820.mem)[u64_arg0_20338];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20340;
        
        if (loop_cond_20339) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_20391 = ((int64_t *) ext_mem_20826.mem)[u64_arg0_20338];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20392 = loop_cond_t_res_20391 == increment_arg1_18077;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20393 = !loop_cond_t_res_20392;
            
            loop_cond_20340 = loop_cond_t_res_20393;
        } else {
            loop_cond_20340 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_20344;
        int64_t slot_res_20345;
        bool loop_while_20346;
        int64_t i_20347;
        
        loop_while_20346 = loop_cond_20340;
        i_20347 = u64_arg0_20338;
        while (loop_while_20346) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_20348 = add64((int64_t) 1, i_20347);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_20349 = mask_20329 & za_lhs_20348;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_20350 = ((bool *) ext_mem_20820.mem)[loopres_20349];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_20351;
            
            if (loop_cond_20350) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_20394 = ((int64_t *) ext_mem_20826.mem)[loopres_20349];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20395 = loop_cond_t_res_20394 == increment_arg1_18077;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20396 = !loop_cond_t_res_20395;
                
                loop_cond_20351 = loop_cond_t_res_20396;
            } else {
                loop_cond_20351 = 0;
            }
            
            bool loop_while_tmp_21007 = loop_cond_20351;
            int64_t i_tmp_21008 = loopres_20349;
            
            loop_while_20346 = loop_while_tmp_21007;
            i_20347 = i_tmp_21008;
        }
        slot_res_20344 = loop_while_20346;
        slot_res_20345 = i_20347;
        // stdlib/hashmap.fut:38:7-62
        
        bool x_18097 = sle64((int64_t) 0, slot_res_20345);
        
        // stdlib/hashmap.fut:38:7-62
        
        bool y_18098 = slt64(slot_res_20345, increment_res_18082);
        
        // stdlib/hashmap.fut:38:7-62
        
        bool bounds_check_18099 = x_18097 && y_18098;
        
        // stdlib/hashmap.fut:38:7-62
        
        bool index_certs_18100;
        
        if (!bounds_check_18099) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_20345, "] out of bounds for array of shape [", (long long) increment_res_18082, "].", "-> #0  stdlib/hashmap.fut:38:7-62\n   #1  probes/stdlib-tests.fut:18:59-92\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // probes/stdlib-tests.fut:18:83-92
        
        int64_t increment_arg2_18074 = ((int64_t *) deltas_mem_20796.mem)[i_18066];
        
        // stdlib/hashmap.fut:37:40-48
        
        int64_t increment_res_18093 = ((int64_t *) ext_mem_20823.mem)[slot_res_20345];
        
        // stdlib/hashmap.fut:37:40-48
        
        bool increment_res_18094 = ((bool *) ext_mem_20820.mem)[slot_res_20345];
        
        // stdlib/hashmap.fut:38:24-49
        
        int64_t zp_lhs_18095;
        
        if (increment_res_18094) {
            zp_lhs_18095 = increment_res_18093;
        } else {
            zp_lhs_18095 = (int64_t) 0;
        }
        // stdlib/hashmap.fut:38:50-56
        
        int64_t tmp_18096 = add64(increment_arg2_18074, zp_lhs_18095);
        
        // stdlib/hashmap.fut:38:7-62
        ((int64_t *) ext_mem_20826.mem)[slot_res_20345] = increment_arg1_18077;
        // stdlib/hashmap.fut:38:7-62
        ((int64_t *) ext_mem_20823.mem)[slot_res_20345] = tmp_18096;
        // stdlib/hashmap.fut:38:7-62
        ((bool *) ext_mem_20820.mem)[slot_res_20345] = 1;
        // probes/stdlib-tests.fut:18:59-92
        
        bool cond_neg_18104 = !increment_res_18094;
        
        // probes/stdlib-tests.fut:18:59-92
        
        int64_t zp_rhs_18105 = btoi_bool_i64(cond_neg_18104);
        
        // stdlib/hashmap.fut:39:17-40
        
        int64_t tmp_18106 = add64(increment_res_18086, zp_rhs_18105);
        
        if (memblock_set(ctx, &mem_param_tmp_20999, &ext_mem_20826, "ext_mem_20826") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_21000, &ext_mem_20823, "ext_mem_20823") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_21001, &ext_mem_20820, "ext_mem_20820") != 0)
            return 1;
        
        int64_t loop_dz2082U_tmp_21002 = increment_res_18082;
        int64_t table_tmp_21006 = tmp_18106;
        
        if (memblock_set(ctx, &mem_param_20802, &mem_param_tmp_20999, "mem_param_tmp_20999") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20805, &mem_param_tmp_21000, "mem_param_tmp_21000") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20808, &mem_param_tmp_21001, "mem_param_tmp_21001") != 0)
            return 1;
        loop_dz2082U_18067 = loop_dz2082U_tmp_21002;
        table_18071 = table_tmp_21006;
    }
    if (memblock_set(ctx, &ext_mem_20835, &mem_param_20802, "mem_param_20802") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_20834, &mem_param_20805, "mem_param_20805") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_20833, &mem_param_20808, "mem_param_20808") != 0)
        return 1;
    hashmap_increment_res_18061 = loop_dz2082U_18067;
    hashmap_increment_res_18065 = table_18071;
    if (memblock_unref(ctx, &ext_mem_20797, "ext_mem_20797") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20799, "ext_mem_20799") != 0)
        return 1;
    // stdlib/hashmap.fut:11:26-28
    
    int64_t mask_20360 = sub64(hashmap_increment_res_18061, (int64_t) 1);
    
    if (memblock_alloc(ctx, &mem_20837, bytes_20836, "mem_20837")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:19:6-26
    for (int64_t i_20781 = 0; i_20781 < dz2080U_17344; i_20781++) {
        int64_t eta_p_18108 = ((int64_t *) keys_mem_20795.mem)[i_20781];
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_20361 = lshr64(eta_p_18108, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_20362 = eta_p_18108 ^ zc_rhs_20361;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_20363 = mul64((int64_t) -4658895280553007687, zt_lhs_20362);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_20364 = lshr64(x_20363, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_20365 = x_20363 ^ zc_rhs_20364;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_20366 = mul64((int64_t) -7723592293110705685, zt_lhs_20365);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_20367 = lshr64(x_20366, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_20368 = x_20366 ^ zc_rhs_20367;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_20369 = mask_20360 & hash_res_20368;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20370 = ((bool *) ext_mem_20833.mem)[u64_arg0_20369];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20371;
        
        if (loop_cond_20370) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_20397 = ((int64_t *) ext_mem_20835.mem)[u64_arg0_20369];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20398 = loop_cond_t_res_20397 == eta_p_18108;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20399 = !loop_cond_t_res_20398;
            
            loop_cond_20371 = loop_cond_t_res_20399;
        } else {
            loop_cond_20371 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_20375;
        int64_t slot_res_20376;
        bool loop_while_20377;
        int64_t i_20378;
        
        loop_while_20377 = loop_cond_20371;
        i_20378 = u64_arg0_20369;
        while (loop_while_20377) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_20379 = add64((int64_t) 1, i_20378);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_20380 = mask_20360 & za_lhs_20379;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_20381 = ((bool *) ext_mem_20833.mem)[loopres_20380];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_20382;
            
            if (loop_cond_20381) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_20400 = ((int64_t *) ext_mem_20835.mem)[loopres_20380];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20401 = loop_cond_t_res_20400 == eta_p_18108;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20402 = !loop_cond_t_res_20401;
                
                loop_cond_20382 = loop_cond_t_res_20402;
            } else {
                loop_cond_20382 = 0;
            }
            
            bool loop_while_tmp_21010 = loop_cond_20382;
            int64_t i_tmp_21011 = loopres_20380;
            
            loop_while_20377 = loop_while_tmp_21010;
            i_20378 = i_tmp_21011;
        }
        slot_res_20375 = loop_while_20377;
        slot_res_20376 = i_20378;
        // stdlib/hashmap.fut:17:9-17
        
        bool x_18110 = sle64((int64_t) 0, slot_res_20376);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool y_18111 = slt64(slot_res_20376, hashmap_increment_res_18061);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool bounds_check_18112 = x_18110 && y_18111;
        
        // stdlib/hashmap.fut:17:9-17
        
        bool index_certs_18113;
        
        if (!bounds_check_18112) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_20376, "] out of bounds for array of shape [", (long long) hashmap_increment_res_18061, "].", "-> #0  stdlib/hashmap.fut:17:9-17\n   #1  probes/stdlib-tests.fut:19:11-14\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:17:9-17
        
        bool get_res_18114 = ((bool *) ext_mem_20833.mem)[slot_res_20376];
        
        // stdlib/hashmap.fut:17:6-42
        
        int64_t get_res_18115;
        
        if (get_res_18114) {
            // stdlib/hashmap.fut:17:25-33
            
            int64_t get_res_t_res_20403 = ((int64_t *) ext_mem_20834.mem)[slot_res_20376];
            
            get_res_18115 = get_res_t_res_20403;
        } else {
            get_res_18115 = (int64_t) 0;
        }
        ((int64_t *) mem_20837.mem)[i_20781] = get_res_18115;
    }
    if (memblock_unref(ctx, &ext_mem_20833, "ext_mem_20833") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20834, "ext_mem_20834") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20835, "ext_mem_20835") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20998, &mem_20837, "mem_20837") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21093, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_20837, "mem_20837") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21001, "mem_param_tmp_21001") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21000, "mem_param_tmp_21000") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_20999, "mem_param_tmp_20999") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20815, "ext_mem_20815") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20816, "ext_mem_20816") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20817, "ext_mem_20817") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20820, "ext_mem_20820") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20823, "ext_mem_20823") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20826, "ext_mem_20826") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20808, "mem_param_20808") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20805, "mem_param_20805") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20802, "mem_param_20802") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20833, "ext_mem_20833") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20834, "ext_mem_20834") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20835, "ext_mem_20835") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20797, "ext_mem_20797") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20799, "ext_mem_20799") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_hashmap_reserved(struct futhark_context *ctx, struct memblock *mem_out_p_21094, struct memblock keys_mem_20795, struct memblock deltas_mem_20796, int64_t dz2080U_17606, int64_t dz2081U_17607, int64_t batch_17610)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_20864;
    
    mem_20864.references = NULL;
    
    struct memblock mem_param_tmp_21001;
    
    mem_param_tmp_21001.references = NULL;
    
    struct memblock mem_param_tmp_21000;
    
    mem_param_tmp_21000.references = NULL;
    
    struct memblock mem_param_tmp_20999;
    
    mem_param_tmp_20999.references = NULL;
    
    struct memblock mem_param_tmp_21011;
    
    mem_param_tmp_21011.references = NULL;
    
    struct memblock mem_param_tmp_21010;
    
    mem_param_tmp_21010.references = NULL;
    
    struct memblock mem_param_tmp_21009;
    
    mem_param_tmp_21009.references = NULL;
    
    struct memblock ext_mem_20824;
    
    ext_mem_20824.references = NULL;
    
    struct memblock ext_mem_20825;
    
    ext_mem_20825.references = NULL;
    
    struct memblock ext_mem_20826;
    
    ext_mem_20826.references = NULL;
    
    struct memblock mem_param_20817;
    
    mem_param_20817.references = NULL;
    
    struct memblock mem_param_20814;
    
    mem_param_20814.references = NULL;
    
    struct memblock mem_param_20811;
    
    mem_param_20811.references = NULL;
    
    struct memblock ext_mem_20833;
    
    ext_mem_20833.references = NULL;
    
    struct memblock ext_mem_20834;
    
    ext_mem_20834.references = NULL;
    
    struct memblock ext_mem_20835;
    
    ext_mem_20835.references = NULL;
    
    struct memblock mem_param_20808;
    
    mem_param_20808.references = NULL;
    
    struct memblock mem_param_20805;
    
    mem_param_20805.references = NULL;
    
    struct memblock mem_param_20802;
    
    mem_param_20802.references = NULL;
    
    struct memblock ext_mem_20860;
    
    ext_mem_20860.references = NULL;
    
    struct memblock ext_mem_20861;
    
    ext_mem_20861.references = NULL;
    
    struct memblock ext_mem_20862;
    
    ext_mem_20862.references = NULL;
    
    struct memblock ext_mem_20797;
    
    ext_mem_20797.references = NULL;
    
    struct memblock ext_mem_20798;
    
    ext_mem_20798.references = NULL;
    
    struct memblock ext_mem_20799;
    
    ext_mem_20799.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t bytes_20863 = (int64_t) 8 * dz2080U_17606;
    
    // probes/stdlib-tests.fut:21:36-43
    
    int64_t hashmap_reserved_res_18061;
    
    if (futrts_empty_7765(ctx, &ext_mem_20799, &ext_mem_20798, &ext_mem_20797, &hashmap_reserved_res_18061, (int64_t) 2) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:21:15-27:27
    
    bool loop_cond_18062 = slt64((int64_t) 0, dz2080U_17606);
    
    // probes/stdlib-tests.fut:21:15-27:27
    
    int64_t hashmap_reserved_res_18065;
    bool hashmap_reserved_res_18066;
    int64_t hashmap_reserved_res_18070;
    int64_t hashmap_reserved_res_18071;
    int64_t loop_dz2082U_18072;
    bool loop_while_18073;
    int64_t table_18077;
    int64_t next_18078;
    
    if (memblock_set(ctx, &mem_param_20802, &ext_mem_20799, "ext_mem_20799") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_20805, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_20808, &ext_mem_20797, "ext_mem_20797") != 0)
        return 1;
    loop_dz2082U_18072 = (int64_t) 2;
    loop_while_18073 = loop_cond_18062;
    table_18077 = hashmap_reserved_res_18061;
    next_18078 = (int64_t) 0;
    while (loop_while_18073) {
        // probes/stdlib-tests.fut:22:43-49
        
        int64_t min_arg1_18079 = add64(batch_17610, next_18078);
        
        // probes/stdlib-tests.fut:22:16-49
        
        int64_t min_res_18080 = smin64(dz2080U_17606, min_arg1_18079);
        
        // probes/stdlib-tests.fut:24:24-26:28
        
        bool bounds_invalid_upwards_18107 = slt64(min_res_18080, next_18078);
        
        // probes/stdlib-tests.fut:24:24-26:28
        
        bool valid_18109 = !bounds_invalid_upwards_18107;
        
        // probes/stdlib-tests.fut:24:24-26:28
        
        bool range_valid_c_18110;
        
        if (!valid_18109) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) next_18078, "..<", (long long) min_res_18080, " is invalid.", "-> #0  probes/stdlib-tests.fut:24:24-26:28\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // probes/stdlib-tests.fut:24:66-77
        
        int64_t dzlz7bUZLzmZRz20Ustopz20Unextz7dUzg_18081 = sub64(min_res_18080, next_18078);
        
        // stdlib/hashmap.fut:43:3-76
        
        int64_t zt_lhs_18082 = add64(table_18077, dzlz7bUZLzmZRz20Ustopz20Unextz7dUzg_18081);
        
        // stdlib/hashmap.fut:43:3-76
        
        int64_t zg_lhs_18083 = mul64((int64_t) 10, zt_lhs_18082);
        
        // stdlib/hashmap.fut:43:3-76
        
        int64_t zg_rhs_18084 = mul64((int64_t) 7, loop_dz2082U_18072);
        
        // stdlib/hashmap.fut:43:3-76
        
        bool loop_cond_18085 = slt64(zg_rhs_18084, zg_lhs_18083);
        
        // stdlib/hashmap.fut:43:3-76
        
        int64_t reserve_res_18086;
        bool reserve_res_18087;
        int64_t reserve_res_18091;
        int64_t loop_dz2080U_18092;
        bool loop_while_18093;
        int64_t t_18097;
        
        if (memblock_set(ctx, &mem_param_20811, &mem_param_20802, "mem_param_20802") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20814, &mem_param_20805, "mem_param_20805") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20817, &mem_param_20808, "mem_param_20808") != 0)
            return 1;
        loop_dz2080U_18092 = loop_dz2082U_18072;
        loop_while_18093 = loop_cond_18085;
        t_18097 = table_18077;
        while (loop_while_18093) {
            // stdlib/hashmap.fut:43:70-76
            
            int64_t loopres_18098;
            int64_t loopres_18102;
            
            if (futrts_grow_11344(ctx, &ext_mem_20826, &ext_mem_20825, &ext_mem_20824, &loopres_18098, &loopres_18102, mem_param_20811, mem_param_20814, mem_param_20817, loop_dz2080U_18092, t_18097) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/hashmap.fut:43:24-35
            
            int64_t zt_lhs_18103 = add64(dzlz7bUZLzmZRz20Ustopz20Unextz7dUzg_18081, loopres_18102);
            
            // stdlib/hashmap.fut:43:36-39
            
            int64_t zg_lhs_18104 = mul64((int64_t) 10, zt_lhs_18103);
            
            // stdlib/hashmap.fut:43:52-54
            
            int64_t zg_rhs_18105 = mul64((int64_t) 7, loopres_18098);
            
            // stdlib/hashmap.fut:43:40-54
            
            bool loop_cond_18106 = slt64(zg_rhs_18105, zg_lhs_18104);
            
            if (memblock_set(ctx, &mem_param_tmp_21009, &ext_mem_20826, "ext_mem_20826") != 0)
                return 1;
            if (memblock_set(ctx, &mem_param_tmp_21010, &ext_mem_20825, "ext_mem_20825") != 0)
                return 1;
            if (memblock_set(ctx, &mem_param_tmp_21011, &ext_mem_20824, "ext_mem_20824") != 0)
                return 1;
            
            int64_t loop_dz2080U_tmp_21012 = loopres_18098;
            bool loop_while_tmp_21013 = loop_cond_18106;
            int64_t t_tmp_21017 = loopres_18102;
            
            if (memblock_set(ctx, &mem_param_20811, &mem_param_tmp_21009, "mem_param_tmp_21009") != 0)
                return 1;
            if (memblock_set(ctx, &mem_param_20814, &mem_param_tmp_21010, "mem_param_tmp_21010") != 0)
                return 1;
            if (memblock_set(ctx, &mem_param_20817, &mem_param_tmp_21011, "mem_param_tmp_21011") != 0)
                return 1;
            loop_dz2080U_18092 = loop_dz2080U_tmp_21012;
            loop_while_18093 = loop_while_tmp_21013;
            t_18097 = t_tmp_21017;
        }
        if (memblock_set(ctx, &ext_mem_20835, &mem_param_20811, "mem_param_20811") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_20834, &mem_param_20814, "mem_param_20814") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_20833, &mem_param_20817, "mem_param_20817") != 0)
            return 1;
        reserve_res_18086 = loop_dz2080U_18092;
        reserve_res_18087 = loop_while_18093;
        reserve_res_18091 = t_18097;
        // stdlib/hashmap.fut:11:26-28
        
        int64_t mask_20329 = sub64(reserve_res_18086, (int64_t) 1);
        
        // probes/stdlib-tests.fut:24:24-26:28
        
        int64_t loopres_18114;
        int64_t sizze_18119 = reserve_res_18091;
        
        for (int64_t i_18115 = 0; i_18115 < dzlz7bUZLzmZRz20Ustopz20Unextz7dUzg_18081; i_18115++) {
            int64_t index_primexp_18120 = add64(next_18078, i_18115);
            
            // probes/stdlib-tests.fut:25:57-66
            
            bool x_18121 = sle64((int64_t) 0, index_primexp_18120);
            
            // probes/stdlib-tests.fut:25:57-66
            
            bool y_18122 = slt64(index_primexp_18120, dz2081U_17607);
            
            // probes/stdlib-tests.fut:25:57-66
            
            bool bounds_check_18123 = x_18121 && y_18122;
            
            // probes/stdlib-tests.fut:25:57-66
            
            bool index_certs_18124;
            
            if (!bounds_check_18123) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) index_primexp_18120, "] out of bounds for array of shape [", (long long) dz2081U_17607, "].", "-> #0  probes/stdlib-tests.fut:25:57-66\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // probes/stdlib-tests.fut:25:49-56
            
            bool y_18127 = slt64(index_primexp_18120, dz2080U_17606);
            
            // probes/stdlib-tests.fut:25:49-56
            
            bool bounds_check_18128 = x_18121 && y_18127;
            
            // probes/stdlib-tests.fut:25:49-56
            
            bool index_certs_18129;
            
            if (!bounds_check_18128) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) index_primexp_18120, "] out of bounds for array of shape [", (long long) dz2080U_17606, "].", "-> #0  probes/stdlib-tests.fut:25:49-56\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // probes/stdlib-tests.fut:25:57-66
            
            int64_t increment_slots_arg2_18125 = ((int64_t *) deltas_mem_20796.mem)[index_primexp_18120];
            
            // probes/stdlib-tests.fut:25:49-56
            
            int64_t increment_slots_arg1_18130 = ((int64_t *) keys_mem_20795.mem)[index_primexp_18120];
            
            // stdlib/hashmap.fut:7:19-24
            
            int64_t zc_rhs_20330 = lshr64(increment_slots_arg1_18130, (int64_t) 30);
            
            // stdlib/hashmap.fut:7:14-24
            
            int64_t zt_lhs_20331 = increment_slots_arg1_18130 ^ zc_rhs_20330;
            
            // stdlib/hashmap.fut:7:26-48
            
            int64_t x_20332 = mul64((int64_t) -4658895280553007687, zt_lhs_20331);
            
            // stdlib/hashmap.fut:8:19-24
            
            int64_t zc_rhs_20333 = lshr64(x_20332, (int64_t) 27);
            
            // stdlib/hashmap.fut:8:14-24
            
            int64_t zt_lhs_20334 = x_20332 ^ zc_rhs_20333;
            
            // stdlib/hashmap.fut:8:26-48
            
            int64_t x_20335 = mul64((int64_t) -7723592293110705685, zt_lhs_20334);
            
            // stdlib/hashmap.fut:9:13-18
            
            int64_t zc_rhs_20336 = lshr64(x_20335, (int64_t) 31);
            
            // stdlib/hashmap.fut:9:8-18
            
            int64_t hash_res_20337 = x_20335 ^ zc_rhs_20336;
            
            // stdlib/hashmap.fut:12:43-57
            
            int64_t u64_arg0_20338 = mask_20329 & hash_res_20337;
            
            // stdlib/hashmap.fut:12:16-13:57
            
            bool loop_cond_20339 = ((bool *) ext_mem_20833.mem)[u64_arg0_20338];
            
            // stdlib/hashmap.fut:12:16-13:57
            
            bool loop_cond_20340;
            
            if (loop_cond_20339) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_20386 = ((int64_t *) ext_mem_20835.mem)[u64_arg0_20338];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20387 = loop_cond_t_res_20386 == increment_slots_arg1_18130;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20388 = !loop_cond_t_res_20387;
                
                loop_cond_20340 = loop_cond_t_res_20388;
            } else {
                loop_cond_20340 = 0;
            }
            // stdlib/hashmap.fut:12:16-13:57
            
            bool slot_res_20344;
            int64_t slot_res_20345;
            bool loop_while_20346;
            int64_t i_20347;
            
            loop_while_20346 = loop_cond_20340;
            i_20347 = u64_arg0_20338;
            while (loop_while_20346) {
                // stdlib/hashmap.fut:13:49-51
                
                int64_t za_lhs_20348 = add64((int64_t) 1, i_20347);
                
                // stdlib/hashmap.fut:13:52-57
                
                int64_t loopres_20349 = mask_20329 & za_lhs_20348;
                
                // stdlib/hashmap.fut:13:12-20
                
                bool loop_cond_20350 = ((bool *) ext_mem_20833.mem)[loopres_20349];
                
                // stdlib/hashmap.fut:13:12-43
                
                bool loop_cond_20351;
                
                if (loop_cond_20350) {
                    // stdlib/hashmap.fut:13:26-34
                    
                    int64_t loop_cond_t_res_20389 = ((int64_t *) ext_mem_20835.mem)[loopres_20349];
                    
                    // stdlib/hashmap.fut:13:37-43
                    
                    bool loop_cond_t_res_20390 = loop_cond_t_res_20389 == increment_slots_arg1_18130;
                    
                    // stdlib/hashmap.fut:13:37-43
                    
                    bool loop_cond_t_res_20391 = !loop_cond_t_res_20390;
                    
                    loop_cond_20351 = loop_cond_t_res_20391;
                } else {
                    loop_cond_20351 = 0;
                }
                
                bool loop_while_tmp_21022 = loop_cond_20351;
                int64_t i_tmp_21023 = loopres_20349;
                
                loop_while_20346 = loop_while_tmp_21022;
                i_20347 = i_tmp_21023;
            }
            slot_res_20344 = loop_while_20346;
            slot_res_20345 = i_20347;
            // stdlib/hashmap.fut:48:37-45
            
            int64_t increment_slots_res_18132 = ((int64_t *) ext_mem_20834.mem)[slot_res_20345];
            
            // stdlib/hashmap.fut:48:37-45
            
            bool increment_slots_res_18133 = ((bool *) ext_mem_20833.mem)[slot_res_20345];
            
            // stdlib/hashmap.fut:49:49-74
            
            int64_t zp_lhs_18134;
            
            if (increment_slots_res_18133) {
                zp_lhs_18134 = increment_slots_res_18132;
            } else {
                zp_lhs_18134 = (int64_t) 0;
            }
            // stdlib/hashmap.fut:49:75-81
            
            int64_t slots_18135 = add64(increment_slots_arg2_18125, zp_lhs_18134);
            
            // stdlib/hashmap.fut:49:26-87
            ((int64_t *) ext_mem_20835.mem)[slot_res_20345] = increment_slots_arg1_18130;
            // stdlib/hashmap.fut:49:26-87
            ((int64_t *) ext_mem_20834.mem)[slot_res_20345] = slots_18135;
            // stdlib/hashmap.fut:49:26-87
            ((bool *) ext_mem_20833.mem)[slot_res_20345] = 1;
            // probes/stdlib-tests.fut:25:27-66
            
            bool cond_neg_18139 = !increment_slots_res_18133;
            
            // probes/stdlib-tests.fut:25:27-66
            
            int64_t tmp_18140 = btoi_bool_i64(cond_neg_18139);
            
            // probes/stdlib-tests.fut:26:21-27
            
            int64_t tmp_18141 = add64(sizze_18119, tmp_18140);
            int64_t sizze_tmp_21021 = tmp_18141;
            
            sizze_18119 = sizze_tmp_21021;
        }
        loopres_18114 = sizze_18119;
        // probes/stdlib-tests.fut:21:61-74
        
        bool loop_cond_18142 = slt64(min_res_18080, dz2080U_17606);
        
        if (memblock_set(ctx, &mem_param_tmp_20999, &ext_mem_20835, "ext_mem_20835") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_21000, &ext_mem_20834, "ext_mem_20834") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_21001, &ext_mem_20833, "ext_mem_20833") != 0)
            return 1;
        
        int64_t loop_dz2082U_tmp_21002 = reserve_res_18086;
        bool loop_while_tmp_21003 = loop_cond_18142;
        int64_t table_tmp_21007 = loopres_18114;
        int64_t next_tmp_21008 = min_res_18080;
        
        if (memblock_set(ctx, &mem_param_20802, &mem_param_tmp_20999, "mem_param_tmp_20999") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20805, &mem_param_tmp_21000, "mem_param_tmp_21000") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20808, &mem_param_tmp_21001, "mem_param_tmp_21001") != 0)
            return 1;
        loop_dz2082U_18072 = loop_dz2082U_tmp_21002;
        loop_while_18073 = loop_while_tmp_21003;
        table_18077 = table_tmp_21007;
        next_18078 = next_tmp_21008;
    }
    if (memblock_set(ctx, &ext_mem_20862, &mem_param_20802, "mem_param_20802") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_20861, &mem_param_20805, "mem_param_20805") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_20860, &mem_param_20808, "mem_param_20808") != 0)
        return 1;
    hashmap_reserved_res_18065 = loop_dz2082U_18072;
    hashmap_reserved_res_18066 = loop_while_18073;
    hashmap_reserved_res_18070 = table_18077;
    hashmap_reserved_res_18071 = next_18078;
    if (memblock_unref(ctx, &ext_mem_20797, "ext_mem_20797") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20799, "ext_mem_20799") != 0)
        return 1;
    // stdlib/hashmap.fut:11:26-28
    
    int64_t mask_20360 = sub64(hashmap_reserved_res_18065, (int64_t) 1);
    
    if (memblock_alloc(ctx, &mem_20864, bytes_20863, "mem_20864")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:28:6-28
    for (int64_t i_20781 = 0; i_20781 < dz2080U_17606; i_20781++) {
        int64_t eta_p_18144 = ((int64_t *) keys_mem_20795.mem)[i_20781];
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_20361 = lshr64(eta_p_18144, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_20362 = eta_p_18144 ^ zc_rhs_20361;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_20363 = mul64((int64_t) -4658895280553007687, zt_lhs_20362);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_20364 = lshr64(x_20363, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_20365 = x_20363 ^ zc_rhs_20364;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_20366 = mul64((int64_t) -7723592293110705685, zt_lhs_20365);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_20367 = lshr64(x_20366, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_20368 = x_20366 ^ zc_rhs_20367;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_20369 = mask_20360 & hash_res_20368;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20370 = ((bool *) ext_mem_20860.mem)[u64_arg0_20369];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20371;
        
        if (loop_cond_20370) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_20392 = ((int64_t *) ext_mem_20862.mem)[u64_arg0_20369];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20393 = loop_cond_t_res_20392 == eta_p_18144;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20394 = !loop_cond_t_res_20393;
            
            loop_cond_20371 = loop_cond_t_res_20394;
        } else {
            loop_cond_20371 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_20375;
        int64_t slot_res_20376;
        bool loop_while_20377;
        int64_t i_20378;
        
        loop_while_20377 = loop_cond_20371;
        i_20378 = u64_arg0_20369;
        while (loop_while_20377) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_20379 = add64((int64_t) 1, i_20378);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_20380 = mask_20360 & za_lhs_20379;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_20381 = ((bool *) ext_mem_20860.mem)[loopres_20380];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_20382;
            
            if (loop_cond_20381) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_20395 = ((int64_t *) ext_mem_20862.mem)[loopres_20380];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20396 = loop_cond_t_res_20395 == eta_p_18144;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20397 = !loop_cond_t_res_20396;
                
                loop_cond_20382 = loop_cond_t_res_20397;
            } else {
                loop_cond_20382 = 0;
            }
            
            bool loop_while_tmp_21025 = loop_cond_20382;
            int64_t i_tmp_21026 = loopres_20380;
            
            loop_while_20377 = loop_while_tmp_21025;
            i_20378 = i_tmp_21026;
        }
        slot_res_20375 = loop_while_20377;
        slot_res_20376 = i_20378;
        // stdlib/hashmap.fut:17:9-17
        
        bool x_18146 = sle64((int64_t) 0, slot_res_20376);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool y_18147 = slt64(slot_res_20376, hashmap_reserved_res_18065);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool bounds_check_18148 = x_18146 && y_18147;
        
        // stdlib/hashmap.fut:17:9-17
        
        bool index_certs_18149;
        
        if (!bounds_check_18148) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_20376, "] out of bounds for array of shape [", (long long) hashmap_reserved_res_18065, "].", "-> #0  stdlib/hashmap.fut:17:9-17\n   #1  probes/stdlib-tests.fut:28:11-14\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:17:9-17
        
        bool get_res_18150 = ((bool *) ext_mem_20860.mem)[slot_res_20376];
        
        // stdlib/hashmap.fut:17:6-42
        
        int64_t get_res_18151;
        
        if (get_res_18150) {
            // stdlib/hashmap.fut:17:25-33
            
            int64_t get_res_t_res_20398 = ((int64_t *) ext_mem_20861.mem)[slot_res_20376];
            
            get_res_18151 = get_res_t_res_20398;
        } else {
            get_res_18151 = (int64_t) 0;
        }
        ((int64_t *) mem_20864.mem)[i_20781] = get_res_18151;
    }
    if (memblock_unref(ctx, &ext_mem_20860, "ext_mem_20860") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20861, "ext_mem_20861") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20862, "ext_mem_20862") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20998, &mem_20864, "mem_20864") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21094, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_20864, "mem_20864") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21001, "mem_param_tmp_21001") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21000, "mem_param_tmp_21000") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_20999, "mem_param_tmp_20999") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21011, "mem_param_tmp_21011") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21010, "mem_param_tmp_21010") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21009, "mem_param_tmp_21009") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20824, "ext_mem_20824") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20825, "ext_mem_20825") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20826, "ext_mem_20826") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20817, "mem_param_20817") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20814, "mem_param_20814") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20811, "mem_param_20811") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20833, "ext_mem_20833") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20834, "ext_mem_20834") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20835, "ext_mem_20835") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20808, "mem_param_20808") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20805, "mem_param_20805") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20802, "mem_param_20802") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20860, "ext_mem_20860") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20861, "ext_mem_20861") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20862, "ext_mem_20862") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20797, "ext_mem_20797") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20799, "ext_mem_20799") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_hashmap_test(struct futhark_context *ctx, struct memblock *mem_out_p_21095, struct memblock keys_mem_20795, int64_t dz2080U_17166)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_20845;
    
    mem_20845.references = NULL;
    
    struct memblock mem_param_tmp_21001;
    
    mem_param_tmp_21001.references = NULL;
    
    struct memblock mem_param_tmp_21000;
    
    mem_param_tmp_21000.references = NULL;
    
    struct memblock mem_param_tmp_20999;
    
    mem_param_tmp_20999.references = NULL;
    
    struct memblock ext_mem_20832;
    
    ext_mem_20832.references = NULL;
    
    struct memblock ext_mem_20833;
    
    ext_mem_20833.references = NULL;
    
    struct memblock ext_mem_20834;
    
    ext_mem_20834.references = NULL;
    
    struct memblock ext_mem_20814;
    
    ext_mem_20814.references = NULL;
    
    struct memblock ext_mem_20815;
    
    ext_mem_20815.references = NULL;
    
    struct memblock ext_mem_20816;
    
    ext_mem_20816.references = NULL;
    
    struct memblock ext_mem_20819;
    
    ext_mem_20819.references = NULL;
    
    struct memblock ext_mem_20822;
    
    ext_mem_20822.references = NULL;
    
    struct memblock ext_mem_20825;
    
    ext_mem_20825.references = NULL;
    
    struct memblock mem_param_20807;
    
    mem_param_20807.references = NULL;
    
    struct memblock mem_param_20804;
    
    mem_param_20804.references = NULL;
    
    struct memblock mem_param_20801;
    
    mem_param_20801.references = NULL;
    
    struct memblock ext_mem_20841;
    
    ext_mem_20841.references = NULL;
    
    struct memblock ext_mem_20842;
    
    ext_mem_20842.references = NULL;
    
    struct memblock ext_mem_20843;
    
    ext_mem_20843.references = NULL;
    
    struct memblock ext_mem_20796;
    
    ext_mem_20796.references = NULL;
    
    struct memblock ext_mem_20797;
    
    ext_mem_20797.references = NULL;
    
    struct memblock ext_mem_20798;
    
    ext_mem_20798.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t bytes_20844 = (int64_t) 8 * dz2080U_17166;
    
    // probes/stdlib-tests.fut:13:28-35
    
    int64_t hashmap_test_res_18058;
    
    if (futrts_empty_7765(ctx, &ext_mem_20798, &ext_mem_20797, &ext_mem_20796, &hashmap_test_res_18058, (int64_t) 2) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:13:15-15:31
    
    int64_t hashmap_test_res_18059;
    int64_t hashmap_test_res_18063;
    int64_t loop_dz2081U_18065;
    int64_t table_18069;
    
    if (memblock_set(ctx, &mem_param_20801, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_20804, &ext_mem_20797, "ext_mem_20797") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_20807, &ext_mem_20796, "ext_mem_20796") != 0)
        return 1;
    loop_dz2081U_18065 = (int64_t) 2;
    table_18069 = hashmap_test_res_18058;
    for (int64_t i_18064 = 0; i_18064 < dz2080U_17166; i_18064++) {
        int64_t key_18070 = ((int64_t *) keys_mem_20795.mem)[i_18064];
        
        // stdlib/hashmap.fut:11:26-28
        
        int64_t mask_20329 = sub64(loop_dz2081U_18065, (int64_t) 1);
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_20330 = lshr64(key_18070, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_20331 = key_18070 ^ zc_rhs_20330;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_20332 = mul64((int64_t) -4658895280553007687, zt_lhs_20331);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_20333 = lshr64(x_20332, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_20334 = x_20332 ^ zc_rhs_20333;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_20335 = mul64((int64_t) -7723592293110705685, zt_lhs_20334);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_20336 = lshr64(x_20335, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_20337 = x_20335 ^ zc_rhs_20336;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_20338 = mask_20329 & hash_res_20337;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20339 = ((bool *) mem_param_20807.mem)[u64_arg0_20338];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20340;
        
        if (loop_cond_20339) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_20386 = ((int64_t *) mem_param_20801.mem)[u64_arg0_20338];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20387 = loop_cond_t_res_20386 == key_18070;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20388 = !loop_cond_t_res_20387;
            
            loop_cond_20340 = loop_cond_t_res_20388;
        } else {
            loop_cond_20340 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_20344;
        int64_t slot_res_20345;
        bool loop_while_20346;
        int64_t i_20347;
        
        loop_while_20346 = loop_cond_20340;
        i_20347 = u64_arg0_20338;
        while (loop_while_20346) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_20348 = add64((int64_t) 1, i_20347);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_20349 = mask_20329 & za_lhs_20348;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_20350 = ((bool *) mem_param_20807.mem)[loopres_20349];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_20351;
            
            if (loop_cond_20350) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_20389 = ((int64_t *) mem_param_20801.mem)[loopres_20349];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20390 = loop_cond_t_res_20389 == key_18070;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20391 = !loop_cond_t_res_20390;
                
                loop_cond_20351 = loop_cond_t_res_20391;
            } else {
                loop_cond_20351 = 0;
            }
            
            bool loop_while_tmp_21007 = loop_cond_20351;
            int64_t i_tmp_21008 = loopres_20349;
            
            loop_while_20346 = loop_while_tmp_21007;
            i_20347 = i_tmp_21008;
        }
        slot_res_20344 = loop_while_20346;
        slot_res_20345 = i_20347;
        // stdlib/hashmap.fut:17:9-17
        
        bool x_18072 = sle64((int64_t) 0, slot_res_20345);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool y_18073 = slt64(slot_res_20345, loop_dz2081U_18065);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool bounds_check_18074 = x_18072 && y_18073;
        
        // stdlib/hashmap.fut:17:9-17
        
        bool index_certs_18075;
        
        if (!bounds_check_18074) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_20345, "] out of bounds for array of shape [", (long long) loop_dz2081U_18065, "].", "-> #0  stdlib/hashmap.fut:17:9-17\n   #1  probes/stdlib-tests.fut:14:17-30\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:17:9-17
        
        bool get_res_18076 = ((bool *) mem_param_20807.mem)[slot_res_20345];
        
        // stdlib/hashmap.fut:17:6-42
        
        int64_t get_res_18077;
        
        if (get_res_18076) {
            // stdlib/hashmap.fut:17:25-33
            
            int64_t get_res_t_res_20392 = ((int64_t *) mem_param_20804.mem)[slot_res_20345];
            
            get_res_18077 = get_res_t_res_20392;
        } else {
            get_res_18077 = (int64_t) 0;
        }
        // probes/stdlib-tests.fut:15:28-30
        
        int64_t put_arg2_18083 = add64((int64_t) 1, get_res_18077);
        
        // stdlib/hashmap.fut:29:18-20
        
        int64_t zt_lhs_18084 = add64((int64_t) 1, table_18069);
        
        // stdlib/hashmap.fut:29:21-24
        
        int64_t zg_lhs_18085 = mul64((int64_t) 10, zt_lhs_18084);
        
        // stdlib/hashmap.fut:29:37-39
        
        int64_t zg_rhs_18086 = mul64((int64_t) 7, loop_dz2081U_18065);
        
        // stdlib/hashmap.fut:29:11-70
        
        bool cond_18087 = slt64(zg_rhs_18086, zg_lhs_18085);
        int64_t put_res_18088;
        int64_t put_res_18092;
        
        if (cond_18087) {
            // stdlib/hashmap.fut:28:10-29:61
            
            int64_t put_res_t_res_20393;
            int64_t put_res_t_res_20397;
            
            if (futrts_grow_11344(ctx, &ext_mem_20816, &ext_mem_20815, &ext_mem_20814, &put_res_t_res_20393, &put_res_t_res_20397, mem_param_20801, mem_param_20804, mem_param_20807, loop_dz2081U_18065, table_18069) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_20825, &ext_mem_20816, "ext_mem_20816") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20822, &ext_mem_20815, "ext_mem_20815") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20819, &ext_mem_20814, "ext_mem_20814") != 0)
                return 1;
            put_res_18088 = put_res_t_res_20393;
            put_res_18092 = put_res_t_res_20397;
        } else {
            if (memblock_set(ctx, &ext_mem_20825, &mem_param_20801, "mem_param_20801") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20822, &mem_param_20804, "mem_param_20804") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20819, &mem_param_20807, "mem_param_20807") != 0)
                return 1;
            put_res_18088 = loop_dz2081U_18065;
            put_res_18092 = table_18069;
        }
        // stdlib/hashmap.fut:29:7-30:25
        
        int64_t put_res_18101;
        
        if (futrts_put_raw_11346(ctx, &ext_mem_20834, &ext_mem_20833, &ext_mem_20832, &put_res_18101, ext_mem_20825, ext_mem_20822, ext_mem_20819, put_res_18088, put_res_18092, key_18070, put_arg2_18083) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_20819, "ext_mem_20819") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20822, "ext_mem_20822") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20825, "ext_mem_20825") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_20999, &ext_mem_20834, "ext_mem_20834") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_21000, &ext_mem_20833, "ext_mem_20833") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_21001, &ext_mem_20832, "ext_mem_20832") != 0)
            return 1;
        
        int64_t loop_dz2081U_tmp_21002 = put_res_18088;
        int64_t table_tmp_21006 = put_res_18101;
        
        if (memblock_set(ctx, &mem_param_20801, &mem_param_tmp_20999, "mem_param_tmp_20999") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20804, &mem_param_tmp_21000, "mem_param_tmp_21000") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20807, &mem_param_tmp_21001, "mem_param_tmp_21001") != 0)
            return 1;
        loop_dz2081U_18065 = loop_dz2081U_tmp_21002;
        table_18069 = table_tmp_21006;
    }
    if (memblock_set(ctx, &ext_mem_20843, &mem_param_20801, "mem_param_20801") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_20842, &mem_param_20804, "mem_param_20804") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_20841, &mem_param_20807, "mem_param_20807") != 0)
        return 1;
    hashmap_test_res_18059 = loop_dz2081U_18065;
    hashmap_test_res_18063 = table_18069;
    if (memblock_unref(ctx, &ext_mem_20796, "ext_mem_20796") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20797, "ext_mem_20797") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    // stdlib/hashmap.fut:11:26-28
    
    int64_t mask_20360 = sub64(hashmap_test_res_18059, (int64_t) 1);
    
    if (memblock_alloc(ctx, &mem_20845, bytes_20844, "mem_20845")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:16:6-26
    for (int64_t i_20781 = 0; i_20781 < dz2080U_17166; i_20781++) {
        int64_t eta_p_18103 = ((int64_t *) keys_mem_20795.mem)[i_20781];
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_20361 = lshr64(eta_p_18103, (int64_t) 30);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_20362 = eta_p_18103 ^ zc_rhs_20361;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_20363 = mul64((int64_t) -4658895280553007687, zt_lhs_20362);
        
        // stdlib/hashmap.fut:8:19-24
        
        int64_t zc_rhs_20364 = lshr64(x_20363, (int64_t) 27);
        
        // stdlib/hashmap.fut:8:14-24
        
        int64_t zt_lhs_20365 = x_20363 ^ zc_rhs_20364;
        
        // stdlib/hashmap.fut:8:26-48
        
        int64_t x_20366 = mul64((int64_t) -7723592293110705685, zt_lhs_20365);
        
        // stdlib/hashmap.fut:9:13-18
        
        int64_t zc_rhs_20367 = lshr64(x_20366, (int64_t) 31);
        
        // stdlib/hashmap.fut:9:8-18
        
        int64_t hash_res_20368 = x_20366 ^ zc_rhs_20367;
        
        // stdlib/hashmap.fut:12:43-57
        
        int64_t u64_arg0_20369 = mask_20360 & hash_res_20368;
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20370 = ((bool *) ext_mem_20841.mem)[u64_arg0_20369];
        
        // stdlib/hashmap.fut:12:16-13:57
        
        bool loop_cond_20371;
        
        if (loop_cond_20370) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_20398 = ((int64_t *) ext_mem_20843.mem)[u64_arg0_20369];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20399 = loop_cond_t_res_20398 == eta_p_18103;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20400 = !loop_cond_t_res_20399;
            
            loop_cond_20371 = loop_cond_t_res_20400;
        } else {
            loop_cond_20371 = 0;
        }
        // stdlib/hashmap.fut:12:16-13:57
        
        bool slot_res_20375;
        int64_t slot_res_20376;
        bool loop_while_20377;
        int64_t i_20378;
        
        loop_while_20377 = loop_cond_20371;
        i_20378 = u64_arg0_20369;
        while (loop_while_20377) {
            // stdlib/hashmap.fut:13:49-51
            
            int64_t za_lhs_20379 = add64((int64_t) 1, i_20378);
            
            // stdlib/hashmap.fut:13:52-57
            
            int64_t loopres_20380 = mask_20360 & za_lhs_20379;
            
            // stdlib/hashmap.fut:13:12-20
            
            bool loop_cond_20381 = ((bool *) ext_mem_20841.mem)[loopres_20380];
            
            // stdlib/hashmap.fut:13:12-43
            
            bool loop_cond_20382;
            
            if (loop_cond_20381) {
                // stdlib/hashmap.fut:13:26-34
                
                int64_t loop_cond_t_res_20401 = ((int64_t *) ext_mem_20843.mem)[loopres_20380];
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20402 = loop_cond_t_res_20401 == eta_p_18103;
                
                // stdlib/hashmap.fut:13:37-43
                
                bool loop_cond_t_res_20403 = !loop_cond_t_res_20402;
                
                loop_cond_20382 = loop_cond_t_res_20403;
            } else {
                loop_cond_20382 = 0;
            }
            
            bool loop_while_tmp_21010 = loop_cond_20382;
            int64_t i_tmp_21011 = loopres_20380;
            
            loop_while_20377 = loop_while_tmp_21010;
            i_20378 = i_tmp_21011;
        }
        slot_res_20375 = loop_while_20377;
        slot_res_20376 = i_20378;
        // stdlib/hashmap.fut:17:9-17
        
        bool x_18105 = sle64((int64_t) 0, slot_res_20376);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool y_18106 = slt64(slot_res_20376, hashmap_test_res_18059);
        
        // stdlib/hashmap.fut:17:9-17
        
        bool bounds_check_18107 = x_18105 && y_18106;
        
        // stdlib/hashmap.fut:17:9-17
        
        bool index_certs_18108;
        
        if (!bounds_check_18107) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_20376, "] out of bounds for array of shape [", (long long) hashmap_test_res_18059, "].", "-> #0  stdlib/hashmap.fut:17:9-17\n   #1  probes/stdlib-tests.fut:16:11-14\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:17:9-17
        
        bool get_res_18109 = ((bool *) ext_mem_20841.mem)[slot_res_20376];
        
        // stdlib/hashmap.fut:17:6-42
        
        int64_t get_res_18110;
        
        if (get_res_18109) {
            // stdlib/hashmap.fut:17:25-33
            
            int64_t get_res_t_res_20404 = ((int64_t *) ext_mem_20842.mem)[slot_res_20376];
            
            get_res_18110 = get_res_t_res_20404;
        } else {
            get_res_18110 = (int64_t) 0;
        }
        ((int64_t *) mem_20845.mem)[i_20781] = get_res_18110;
    }
    if (memblock_unref(ctx, &ext_mem_20841, "ext_mem_20841") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20842, "ext_mem_20842") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20843, "ext_mem_20843") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20998, &mem_20845, "mem_20845") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21095, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_20845, "mem_20845") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21001, "mem_param_tmp_21001") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21000, "mem_param_tmp_21000") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_20999, "mem_param_tmp_20999") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20832, "ext_mem_20832") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20833, "ext_mem_20833") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20834, "ext_mem_20834") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20814, "ext_mem_20814") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20815, "ext_mem_20815") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20816, "ext_mem_20816") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20819, "ext_mem_20819") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20822, "ext_mem_20822") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20825, "ext_mem_20825") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20807, "mem_param_20807") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20804, "mem_param_20804") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20801, "mem_param_20801") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20841, "ext_mem_20841") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20842, "ext_mem_20842") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20843, "ext_mem_20843") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20796, "ext_mem_20796") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20797, "ext_mem_20797") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_regex_test(struct futhark_context *ctx, struct memblock *mem_out_p_21096, int64_t *out_prim_out_21097, int64_t *out_prim_out_21098, struct memblock pattern_mem_20795, struct memblock text_mem_20796, struct memblock replacement_mem_20797, int64_t dz2080U_16711, int64_t dz2081U_16712, int64_t dz2082U_16713)
{
    (void) ctx;
    
    int err = 0;
    int64_t mem_20866_cached_sizze_21099 = 0;
    unsigned char *mem_20866 = NULL;
    struct memblock mem_20881;
    
    mem_20881.references = NULL;
    
    struct memblock ext_mem_20821;
    
    ext_mem_20821.references = NULL;
    
    struct memblock mem_20820;
    
    mem_20820.references = NULL;
    
    struct memblock mem_20822;
    
    mem_20822.references = NULL;
    
    struct memblock ext_mem_20823;
    
    ext_mem_20823.references = NULL;
    
    struct memblock mem_20840;
    
    mem_20840.references = NULL;
    
    struct memblock ext_mem_20849;
    
    ext_mem_20849.references = NULL;
    
    struct memblock mem_param_tmp_21015;
    
    mem_param_tmp_21015.references = NULL;
    
    struct memblock mem_param_tmp_21019;
    
    mem_param_tmp_21019.references = NULL;
    
    struct memblock ext_mem_20833;
    
    ext_mem_20833.references = NULL;
    
    struct memblock mem_param_20830;
    
    mem_param_20830.references = NULL;
    
    struct memblock ext_mem_20836;
    
    ext_mem_20836.references = NULL;
    
    struct memblock mem_param_20827;
    
    mem_param_20827.references = NULL;
    
    struct memblock ext_mem_20839;
    
    ext_mem_20839.references = NULL;
    
    struct memblock mem_20824;
    
    mem_20824.references = NULL;
    
    struct memblock ext_mem_20852;
    
    ext_mem_20852.references = NULL;
    
    struct memblock mem_20806;
    
    mem_20806.references = NULL;
    
    struct memblock mem_20804;
    
    mem_20804.references = NULL;
    
    struct memblock mem_20802;
    
    mem_20802.references = NULL;
    
    struct memblock mem_20800;
    
    mem_20800.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t prim_out_20999;
    int64_t prim_out_21000;
    
    // stdlib/regex.fut:26:28-30
    
    int64_t sizze_19941 = add64((int64_t) 1, dz2080U_16711);
    int64_t binop_x_20798 = (int64_t) 8 * sizze_19941;
    int64_t bytes_20799 = (int64_t) 32 * sizze_19941;
    
    if (memblock_alloc(ctx, &mem_20800, bytes_20799, "mem_20800")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:10:12-27
    for (int64_t nest_i_21001 = 0; nest_i_21001 < sizze_19941; nest_i_21001++) {
        for (int64_t nest_i_21002 = 0; nest_i_21002 < (int64_t) 4; nest_i_21002++) {
            ((int64_t *) mem_20800.mem)[nest_i_21001 * (int64_t) 4 + nest_i_21002] = (int64_t) 0;
        }
    }
    if (memblock_alloc(ctx, &mem_20802, binop_x_20798, "mem_20802")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:47-66
    for (int64_t nest_i_21003 = 0; nest_i_21003 < sizze_19941; nest_i_21003++) {
        ((int64_t *) mem_20802.mem)[nest_i_21003] = (int64_t) 1;
    }
    if (memblock_alloc(ctx, &mem_20804, binop_x_20798, "mem_20804")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:67-86
    for (int64_t nest_i_21004 = 0; nest_i_21004 < sizze_19941; nest_i_21004++) {
        ((int64_t *) mem_20804.mem)[nest_i_21004] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_20806, binop_x_20798, "mem_20806")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:87-106
    for (int64_t nest_i_21005 = 0; nest_i_21005 < sizze_19941; nest_i_21005++) {
        ((int64_t *) mem_20806.mem)[nest_i_21005] = (int64_t) 0;
    }
    // stdlib/regex.fut:28:5-49:95
    
    bool loop_cond_19946 = slt64((int64_t) 0, dz2080U_16711);
    
    // stdlib/regex.fut:28:5-49:95
    
    bool compile_res_19947;
    int64_t compile_res_19948;
    int64_t compile_res_19953;
    int64_t compile_res_19954;
    bool loop_while_19955;
    int64_t p_19956;
    int64_t used_19961;
    int64_t branch_19962;
    
    loop_while_19955 = loop_cond_19946;
    p_19956 = (int64_t) 0;
    used_19961 = (int64_t) 0;
    branch_19962 = (int64_t) 0;
    while (loop_while_19955) {
        // stdlib/regex.fut:31:12-22
        
        bool x_19963 = sle64((int64_t) 0, p_19956);
        
        // stdlib/regex.fut:31:12-22
        
        bool y_19964 = slt64(p_19956, dz2080U_16711);
        
        // stdlib/regex.fut:31:12-22
        
        bool bounds_check_19965 = x_19963 && y_19964;
        
        // stdlib/regex.fut:31:12-22
        
        bool index_certs_19966;
        
        if (!bounds_check_19965) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) p_19956, "] out of bounds for array of shape [", (long long) dz2080U_16711, "].", "-> #0  stdlib/regex.fut:31:12-22\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:31:12-22
        
        int8_t zeze_lhs_19967 = ((int8_t *) pattern_mem_20795.mem)[p_19956];
        
        // stdlib/regex.fut:31:9-49:95
        
        bool cond_19968 = zeze_lhs_19967 == (int8_t) 124;
        
        // stdlib/regex.fut:36:29-42:57
        
        bool cond_19988 = zeze_lhs_19967 == (int8_t) 91;
        
        // stdlib/regex.fut:31:9-49:95
        
        int64_t loopres_19969;
        int64_t loopres_19974;
        int64_t loopres_19975;
        
        if (cond_19968) {
            // stdlib/regex.fut:32:15-34
            
            bool x_20597 = sle64((int64_t) 0, branch_19962);
            
            // stdlib/regex.fut:32:15-34
            
            bool y_20598 = slt64(branch_19962, sizze_19941);
            
            // stdlib/regex.fut:32:15-34
            
            bool bounds_check_20599 = x_20597 && y_20598;
            
            // stdlib/regex.fut:32:15-34
            
            bool index_certs_20600;
            
            if (!bounds_check_20599) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) branch_19962, "] out of bounds for array of shape [", (long long) sizze_19941, "].", "-> #0  stdlib/regex.fut:32:15-34\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:33:28-30
            
            int64_t tmp_20602 = add64((int64_t) 1, branch_19962);
            
            // stdlib/regex.fut:33:15-38
            
            bool x_20603 = sle64((int64_t) 0, tmp_20602);
            
            // stdlib/regex.fut:33:15-38
            
            bool y_20604 = slt64(tmp_20602, sizze_19941);
            
            // stdlib/regex.fut:33:15-38
            
            bool bounds_check_20605 = x_20603 && y_20604;
            
            // stdlib/regex.fut:33:15-38
            
            bool index_certs_20606;
            
            if (!bounds_check_20605) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_20602, "] out of bounds for array of shape [", (long long) sizze_19941, "].", "-> #0  stdlib/regex.fut:33:15-38\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:32:15-34
            ((int64_t *) mem_20806.mem)[branch_19962] = used_19961;
            // stdlib/regex.fut:33:15-38
            ((int64_t *) mem_20804.mem)[tmp_20602] = used_19961;
            // stdlib/regex.fut:34:16-18
            
            int64_t tmp_20608 = add64((int64_t) 1, p_19956);
            
            loopres_19969 = tmp_20608;
            loopres_19974 = used_19961;
            loopres_19975 = tmp_20602;
        } else {
            // stdlib/regex.fut:15:23-25
            
            int64_t zl_lhs_20644 = add64((int64_t) 1, p_19956);
            
            // stdlib/regex.fut:15:18-68
            
            bool cond_20645 = slt64(zl_lhs_20644, dz2080U_16711);
            
            // stdlib/regex.fut:15:46-62
            
            bool x_20647 = sle64((int64_t) 0, zl_lhs_20644);
            
            // stdlib/regex.fut:15:46-62
            
            bool bounds_check_20648 = cond_20645 && x_20647;
            bool loop_not_taken_20977 = !cond_20645;
            bool protect_assert_disj_20978 = bounds_check_20648 || loop_not_taken_20977;
            bool loop_not_taken_20983 = !cond_19988;
            bool protect_assert_disj_20984 = protect_assert_disj_20978 || loop_not_taken_20983;
            
            // stdlib/regex.fut:15:46-62
            
            bool index_certs_20649;
            
            if (!protect_assert_disj_20984) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_20644, "] out of bounds for array of shape [", (long long) dz2080U_16711, "].", "-> #0  stdlib/regex.fut:15:46-62\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:10:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:47:15-33
            
            bool x_20084 = sle64((int64_t) 0, used_19961);
            
            // stdlib/regex.fut:47:15-33
            
            bool y_20085 = slt64(used_19961, sizze_19941);
            
            // stdlib/regex.fut:47:15-33
            
            bool bounds_check_20086 = x_20084 && y_20085;
            
            // stdlib/regex.fut:47:15-33
            
            bool index_certs_20087;
            
            if (!bounds_check_20086) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) used_19961, "] out of bounds for array of shape [", (long long) sizze_19941, "].", "-> #0  stdlib/regex.fut:47:15-33\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:37:18-42:57
            
            bool cond_20061 = zeze_lhs_19967 == (int8_t) 46;
            bool x_20986 = !cond_20061;
            bool protect_cond_conj_20994 = loop_not_taken_20983 && x_20986;
            
            // stdlib/regex.fut:41:33-50
            
            int8_t loopres_f_res_f_res_f_res_20067;
            int64_t loopres_f_res_f_res_f_res_20068;
            
            if (protect_cond_conj_20994) {
                int8_t x_20988;
                int64_t x_20989;
                
                if (futrts_literal_11305(ctx, &x_20988, &x_20989, pattern_mem_20795, dz2080U_16711, p_19956) != 0) {
                    err = 1;
                    goto cleanup;
                }
                loopres_f_res_f_res_f_res_20067 = x_20988;
                loopres_f_res_f_res_f_res_20068 = x_20989;
            } else {
                loopres_f_res_f_res_f_res_20067 = (int8_t) 0;
                loopres_f_res_f_res_f_res_20068 = (int64_t) 0;
            }
            
            bool protect_cond_conj_20985 = cond_19988 && cond_20645;
            
            // stdlib/regex.fut:15:46-62
            
            int8_t zeze_lhs_20650;
            
            if (protect_cond_conj_20985) {
                int8_t x_20979 = ((int8_t *) pattern_mem_20795.mem)[zl_lhs_20644];
                
                zeze_lhs_20650 = x_20979;
            } else {
                zeze_lhs_20650 = (int8_t) 0;
            }
            // stdlib/regex.fut:15:63-68
            
            bool negative_t_res_20651 = zeze_lhs_20650 == (int8_t) 94;
            bool x_20995 = cond_20645 && negative_t_res_20651;
            
            // stdlib/regex.fut:36:54-74
            
            int64_t zp_rhs_20652 = btoi_bool_i64(x_20995);
            
            // stdlib/regex.fut:16:22-49
            
            int64_t start_20653 = add64(zl_lhs_20644, zp_rhs_20652);
            
            // stdlib/regex.fut:17:18-23:21
            
            bool cond_20655 = slt64(start_20653, dz2080U_16711);
            int64_t loopres_f_res_19990;
            
            if (cond_19988) {
                if (memblock_alloc(ctx, &mem_20824, (int64_t) 32, "mem_20824")) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/regex.fut:17:35-51
                for (int64_t nest_i_21014 = 0; nest_i_21014 < (int64_t) 4; nest_i_21014++) {
                    ((int64_t *) mem_20824.mem)[nest_i_21014] = (int64_t) 0;
                }
                // stdlib/regex.fut:17:18-23:21
                
                bool class_mask_res_20656;
                int64_t class_mask_res_20658;
                bool loop_while_20659;
                int64_t p_20661;
                
                if (memblock_set(ctx, &mem_param_20827, &mem_20824, "mem_20824") != 0)
                    return 1;
                loop_while_20659 = cond_20655;
                p_20661 = start_20653;
                while (loop_while_20659) {
                    // stdlib/regex.fut:19:23-40
                    
                    int8_t loopres_20662;
                    int64_t loopres_20663;
                    
                    if (futrts_literal_11305(ctx, &loopres_20662, &loopres_20663, pattern_mem_20795, dz2080U_16711, p_20661) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:20:23-25
                    
                    int64_t zl_lhs_20664 = add64((int64_t) 1, loopres_20663);
                    
                    // stdlib/regex.fut:20:19-90
                    
                    bool cond_20665 = slt64(zl_lhs_20664, dz2080U_16711);
                    
                    // stdlib/regex.fut:20:19-90
                    
                    bool cond_20666;
                    
                    if (cond_20665) {
                        // stdlib/regex.fut:20:46-59
                        
                        bool x_20667 = sle64((int64_t) 0, loopres_20663);
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool y_20668 = slt64(loopres_20663, dz2080U_16711);
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool bounds_check_20669 = x_20667 && y_20668;
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool index_certs_20670;
                        
                        if (!bounds_check_20669) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_20663, "] out of bounds for array of shape [", (long long) dz2080U_16711, "].", "-> #0  stdlib/regex.fut:20:46-59\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:10:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:20:46-59
                        
                        int8_t zeze_lhs_20671 = ((int8_t *) pattern_mem_20795.mem)[loopres_20663];
                        
                        // stdlib/regex.fut:20:60-65
                        
                        bool cond_t_res_20672 = zeze_lhs_20671 == (int8_t) 45;
                        
                        cond_20666 = cond_t_res_20672;
                    } else {
                        cond_20666 = 0;
                    }
                    // stdlib/regex.fut:20:19-90
                    
                    bool range_20673;
                    
                    if (cond_20666) {
                        // stdlib/regex.fut:20:69-84
                        
                        bool x_20674 = sle64((int64_t) 0, zl_lhs_20664);
                        
                        // stdlib/regex.fut:20:69-84
                        
                        bool bounds_check_20675 = cond_20665 && x_20674;
                        
                        // stdlib/regex.fut:20:69-84
                        
                        bool index_certs_20676;
                        
                        if (!bounds_check_20675) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_20664, "] out of bounds for array of shape [", (long long) dz2080U_16711, "].", "-> #0  stdlib/regex.fut:20:69-84\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:10:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:20:69-84
                        
                        int8_t znze_lhs_20677 = ((int8_t *) pattern_mem_20795.mem)[zl_lhs_20664];
                        
                        // stdlib/regex.fut:20:85-90
                        
                        bool range_t_res_20678 = znze_lhs_20677 == (int8_t) 93;
                        
                        // stdlib/regex.fut:20:85-90
                        
                        bool range_t_res_20679 = !range_t_res_20678;
                        
                        range_20673 = range_t_res_20679;
                    } else {
                        range_20673 = 0;
                    }
                    // stdlib/regex.fut:21:23-76
                    
                    int8_t loopres_20680;
                    int64_t loopres_20681;
                    
                    if (range_20673) {
                        // stdlib/regex.fut:21:37-60
                        
                        int8_t loopres_t_res_20682;
                        int64_t loopres_t_res_20683;
                        
                        if (futrts_literal_11305(ctx, &loopres_t_res_20682, &loopres_t_res_20683, pattern_mem_20795, dz2080U_16711, zl_lhs_20664) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        loopres_20680 = loopres_t_res_20682;
                        loopres_20681 = loopres_t_res_20683;
                    } else {
                        loopres_20680 = loopres_20662;
                        loopres_20681 = loopres_20663;
                    }
                    // stdlib/regex.fut:22:48-57
                    
                    int64_t u8_res_20684 = zext_i8_i64(loopres_20662);
                    
                    // stdlib/regex.fut:22:60-69
                    
                    int64_t u8_res_20685 = zext_i8_i64(loopres_20680);
                    
                    // stdlib/regex.fut:22:69-71
                    
                    int64_t mask_20686 = add64((int64_t) 1, u8_res_20685);
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool bounds_invalid_upwards_20687 = slt64(mask_20686, u8_res_20684);
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool valid_20689 = !bounds_invalid_upwards_20687;
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool range_valid_c_20690;
                    
                    if (!valid_20689) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) u8_res_20684, "..<", (long long) mask_20686, " is invalid.", "-> #0  stdlib/regex.fut:22:18-103\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:10:12-27\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:22:18-103
                    
                    int64_t distance_20688 = sub64(mask_20686, u8_res_20684);
                    
                    // stdlib/regex.fut:22:18-103
                    if (memblock_set(ctx, &mem_param_20830, &mem_param_20827, "mem_param_20827") != 0)
                        return 1;
                    for (int64_t i_20692 = 0; i_20692 < distance_20688; i_20692++) {
                        // stdlib/regex.fut:36:54-74
                        
                        int64_t index_primexp_20694 = add64(u8_res_20684, i_20692);
                        
                        // stdlib/regex.fut:22:90-102
                        
                        int8_t unsign_arg0_20695 = zext_i64_i8(index_primexp_20694);
                        
                        // stdlib/regex.fut:22:75-102
                        if (futrts_mask_set_8212(ctx, &ext_mem_20833, mem_param_20830, unsign_arg0_20695) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        if (memblock_set(ctx, &mem_param_tmp_21019, &ext_mem_20833, "ext_mem_20833") != 0)
                            return 1;
                        if (memblock_set(ctx, &mem_param_20830, &mem_param_tmp_21019, "mem_param_tmp_21019") != 0)
                            return 1;
                    }
                    if (memblock_set(ctx, &ext_mem_20836, &mem_param_20830, "mem_param_20830") != 0)
                        return 1;
                    // stdlib/regex.fut:18:11-65
                    
                    bool cond_20697 = slt64(loopres_20681, dz2080U_16711);
                    
                    // stdlib/regex.fut:18:11-65
                    
                    bool loop_cond_20698;
                    
                    if (cond_20697) {
                        // stdlib/regex.fut:18:34-44
                        
                        bool x_20699 = sle64((int64_t) 0, loopres_20681);
                        
                        // stdlib/regex.fut:18:34-44
                        
                        bool bounds_check_20700 = cond_20697 && x_20699;
                        
                        // stdlib/regex.fut:18:34-44
                        
                        bool index_certs_20701;
                        
                        if (!bounds_check_20700) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_20681, "] out of bounds for array of shape [", (long long) dz2080U_16711, "].", "-> #0  stdlib/regex.fut:18:34-44\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:10:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:18:34-44
                        
                        int8_t znze_lhs_20702 = ((int8_t *) pattern_mem_20795.mem)[loopres_20681];
                        
                        // stdlib/regex.fut:18:34-64
                        
                        bool cond_20703 = znze_lhs_20702 == (int8_t) 93;
                        
                        // stdlib/regex.fut:18:34-64
                        
                        bool cond_20704 = !cond_20703;
                        
                        // stdlib/regex.fut:18:56-64
                        
                        bool loop_cond_t_res_f_res_20705 = loopres_20681 == start_20653;
                        
                        // stdlib/regex.fut:36:54-74
                        
                        bool y_20706 = cond_20703 && loop_cond_t_res_f_res_20705;
                        
                        // stdlib/regex.fut:36:54-74
                        
                        bool loop_cond_t_res_20707 = cond_20704 || y_20706;
                        
                        loop_cond_20698 = loop_cond_t_res_20707;
                    } else {
                        loop_cond_20698 = 0;
                    }
                    if (memblock_set(ctx, &mem_param_tmp_21015, &ext_mem_20836, "ext_mem_20836") != 0)
                        return 1;
                    
                    bool loop_while_tmp_21016 = loop_cond_20698;
                    int64_t p_tmp_21018 = loopres_20681;
                    
                    if (memblock_set(ctx, &mem_param_20827, &mem_param_tmp_21015, "mem_param_tmp_21015") != 0)
                        return 1;
                    loop_while_20659 = loop_while_tmp_21016;
                    p_20661 = p_tmp_21018;
                }
                if (memblock_set(ctx, &ext_mem_20839, &mem_param_20827, "mem_param_20827") != 0)
                    return 1;
                class_mask_res_20656 = loop_while_20659;
                class_mask_res_20658 = p_20661;
                if (memblock_unref(ctx, &mem_20824, "mem_20824") != 0)
                    return 1;
                if (x_20995) {
                    if (memblock_alloc(ctx, &mem_20840, (int64_t) 32, "mem_20840")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:24:24-56
                    for (int64_t i_20781 = 0; i_20781 < (int64_t) 4; i_20781++) {
                        int64_t eta_p_20710 = ((int64_t *) ext_mem_20839.mem)[i_20781];
                        
                        // stdlib/regex.fut:24:37-50
                        
                        int64_t lifted_lambda_res_20711 = (int64_t) -1 ^ eta_p_20710;
                        
                        ((int64_t *) mem_20840.mem)[i_20781] = lifted_lambda_res_20711;
                    }
                    if (memblock_set(ctx, &ext_mem_20849, &mem_20840, "mem_20840") != 0)
                        return 1;
                } else if (memblock_set(ctx, &ext_mem_20849, &ext_mem_20839, "ext_mem_20839") != 0)
                    return 1;
                if (memblock_unref(ctx, &ext_mem_20839, "ext_mem_20839") != 0)
                    return 1;
                // stdlib/regex.fut:24:68-70
                
                int64_t tmp_20715 = add64((int64_t) 1, class_mask_res_20658);
                
                if (memblock_set(ctx, &ext_mem_20852, &ext_mem_20849, "ext_mem_20849") != 0)
                    return 1;
                loopres_f_res_19990 = tmp_20715;
            } else {
                int64_t loopres_f_res_f_res_20063;
                
                if (cond_20061) {
                    loopres_f_res_f_res_20063 = zl_lhs_20644;
                } else {
                    loopres_f_res_f_res_20063 = loopres_f_res_f_res_f_res_20068;
                }
                if (cond_20061) {
                    if (memblock_alloc(ctx, &mem_20822, (int64_t) 32, "mem_20822")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:38:26-49
                    for (int64_t nest_i_21022 = 0; nest_i_21022 < (int64_t) 4; nest_i_21022++) {
                        ((int64_t *) mem_20822.mem)[nest_i_21022] = (int64_t) -1;
                    }
                    // stdlib/regex.fut:39:19-51
                    ((int64_t *) mem_20822.mem)[(int64_t) 0] = (int64_t) -1025;
                    if (memblock_set(ctx, &ext_mem_20823, &mem_20822, "mem_20822") != 0)
                        return 1;
                } else {
                    if (memblock_alloc(ctx, &mem_20820, (int64_t) 32, "mem_20820")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:42:32-48
                    for (int64_t nest_i_21023 = 0; nest_i_21023 < (int64_t) 4; nest_i_21023++) {
                        ((int64_t *) mem_20820.mem)[nest_i_21023] = (int64_t) 0;
                    }
                    // stdlib/regex.fut:42:22-51
                    if (futrts_mask_set_8212(ctx, &ext_mem_20821, mem_20820, loopres_f_res_f_res_f_res_20067) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &mem_20820, "mem_20820") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_20823, &ext_mem_20821, "ext_mem_20821") != 0)
                        return 1;
                }
                if (memblock_set(ctx, &ext_mem_20852, &ext_mem_20823, "ext_mem_20823") != 0)
                    return 1;
                loopres_f_res_19990 = loopres_f_res_f_res_20063;
            }
            // stdlib/regex.fut:43:28-46:54
            
            bool cond_20071 = sle64(dz2080U_16711, loopres_f_res_19990);
            
            // stdlib/regex.fut:43:28-46:54
            
            int64_t quantifier_20072;
            
            if (cond_20071) {
                quantifier_20072 = (int64_t) 1;
            } else {
                // stdlib/regex.fut:44:21-34
                
                bool x_20073 = sle64((int64_t) 0, loopres_f_res_19990);
                
                // stdlib/regex.fut:44:21-34
                
                bool y_20074 = slt64(loopres_f_res_19990, dz2080U_16711);
                
                // stdlib/regex.fut:44:21-34
                
                bool bounds_check_20075 = x_20073 && y_20074;
                
                // stdlib/regex.fut:44:21-34
                
                bool index_certs_20076;
                
                if (!bounds_check_20075) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_f_res_19990, "] out of bounds for array of shape [", (long long) dz2080U_16711, "].", "-> #0  stdlib/regex.fut:44:21-34\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:44:21-34
                
                int8_t zeze_lhs_20077 = ((int8_t *) pattern_mem_20795.mem)[loopres_f_res_19990];
                
                // stdlib/regex.fut:44:18-46:54
                
                bool cond_20078 = zeze_lhs_20077 == (int8_t) 42;
                
                // stdlib/regex.fut:44:18-46:54
                
                int64_t quantifier_f_res_20079;
                
                if (cond_20078) {
                    quantifier_f_res_20079 = (int64_t) 0;
                } else {
                    // stdlib/regex.fut:45:18-46:54
                    
                    bool cond_20080 = zeze_lhs_20077 == (int8_t) 43;
                    
                    // stdlib/regex.fut:45:18-46:54
                    
                    int64_t quantifier_f_res_f_res_20081;
                    
                    if (cond_20080) {
                        quantifier_f_res_f_res_20081 = (int64_t) 2;
                    } else {
                        // stdlib/regex.fut:46:18-54
                        
                        bool cond_20082 = zeze_lhs_20077 == (int8_t) 63;
                        
                        // stdlib/regex.fut:46:18-54
                        
                        int64_t quantifier_f_res_f_res_f_res_20083;
                        
                        if (cond_20082) {
                            quantifier_f_res_f_res_f_res_20083 = (int64_t) 3;
                        } else {
                            quantifier_f_res_f_res_f_res_20083 = (int64_t) 1;
                        }
                        quantifier_f_res_f_res_20081 = quantifier_f_res_f_res_f_res_20083;
                    }
                    quantifier_f_res_20079 = quantifier_f_res_f_res_20081;
                }
                quantifier_20072 = quantifier_f_res_20079;
            }
            // stdlib/regex.fut:47:15-33
            // stdlib/regex.fut:47:15-33
            lmad_copy_8b(ctx, 1, (uint64_t *) mem_20800.mem, used_19961 * (int64_t) 4, (int64_t []) {(int64_t) 1}, (uint64_t *) ext_mem_20852.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {(int64_t) 4});
            if (memblock_unref(ctx, &ext_mem_20852, "ext_mem_20852") != 0)
                return 1;
            // stdlib/regex.fut:48:15-41
            ((int64_t *) mem_20802.mem)[used_19961] = quantifier_20072;
            // stdlib/regex.fut:49:21-53
            
            bool cond_20090 = quantifier_20072 == (int64_t) 1;
            
            // probes/stdlib-tests.fut:10:12-27
            
            bool cond_neg_20091 = !cond_20090;
            
            // probes/stdlib-tests.fut:10:12-27
            
            int64_t zp_rhs_20092 = btoi_bool_i64(cond_neg_20091);
            
            // stdlib/regex.fut:49:19-53
            
            int64_t tmp_20093 = add64(loopres_f_res_19990, zp_rhs_20092);
            
            // stdlib/regex.fut:49:85-87
            
            int64_t tmp_20094 = add64((int64_t) 1, used_19961);
            
            loopres_19969 = tmp_20093;
            loopres_19974 = tmp_20094;
            loopres_19975 = branch_19962;
        }
        // stdlib/regex.fut:30:15-31
        
        bool loop_cond_20095 = slt64(loopres_19969, dz2080U_16711);
        bool loop_while_tmp_21006 = loop_cond_20095;
        int64_t p_tmp_21007 = loopres_19969;
        int64_t used_tmp_21012 = loopres_19974;
        int64_t branch_tmp_21013 = loopres_19975;
        
        loop_while_19955 = loop_while_tmp_21006;
        p_19956 = p_tmp_21007;
        used_19961 = used_tmp_21012;
        branch_19962 = branch_tmp_21013;
    }
    compile_res_19947 = loop_while_19955;
    compile_res_19948 = p_19956;
    compile_res_19953 = used_19961;
    compile_res_19954 = branch_19962;
    // stdlib/regex.fut:51:47-57
    
    int64_t dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096 = add64((int64_t) 1, compile_res_19954);
    
    // stdlib/regex.fut:50:7-28
    
    bool x_20097 = sle64((int64_t) 0, compile_res_19954);
    
    // stdlib/regex.fut:50:7-28
    
    bool y_20098 = slt64(compile_res_19954, sizze_19941);
    
    // stdlib/regex.fut:50:7-28
    
    bool bounds_check_20099 = x_20097 && y_20098;
    
    // stdlib/regex.fut:50:7-28
    
    bool index_certs_20100;
    
    if (!bounds_check_20099) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) compile_res_19954, "] out of bounds for array of shape [", (long long) sizze_19941, "].", "-> #0  stdlib/regex.fut:50:7-28\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:50:7-28
    ((int64_t *) mem_20806.mem)[compile_res_19954] = compile_res_19953;
    // stdlib/regex.fut:51:7-22
    
    bool empty_slice_20102 = compile_res_19953 == (int64_t) 0;
    
    // stdlib/regex.fut:51:7-22
    
    int64_t m_20103 = sub64(compile_res_19953, (int64_t) 1);
    
    // stdlib/regex.fut:51:7-22
    
    bool zzero_leq_i_p_m_t_s_20104 = sle64((int64_t) 0, m_20103);
    
    // stdlib/regex.fut:51:7-22
    
    bool i_p_m_t_s_leq_w_20105 = slt64(m_20103, sizze_19941);
    
    // stdlib/regex.fut:51:7-22
    
    bool i_lte_j_20106 = sle64((int64_t) 0, compile_res_19953);
    
    // stdlib/regex.fut:51:7-22
    
    bool y_20107 = zzero_leq_i_p_m_t_s_20104 && i_p_m_t_s_leq_w_20105;
    
    // stdlib/regex.fut:51:7-22
    
    bool forwards_ok_20108 = i_lte_j_20106 && y_20107;
    
    // stdlib/regex.fut:51:7-22
    
    bool ok_or_empty_20109 = empty_slice_20102 || forwards_ok_20108;
    
    // stdlib/regex.fut:51:7-22
    
    bool index_certs_20110;
    
    if (!ok_or_empty_20109) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) compile_res_19953, "] out of bounds for array of shape [", (long long) sizze_19941, "].", "-> #0  stdlib/regex.fut:51:7-22\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:23-40
    
    bool ok_or_empty_20112 = empty_slice_20102 || y_20107;
    
    // stdlib/regex.fut:51:23-40
    
    bool index_certs_20113;
    
    if (!ok_or_empty_20112) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) compile_res_19953, "] out of bounds for array of shape [", (long long) sizze_19941, "].", "-> #0  stdlib/regex.fut:51:23-40\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:41-65
    
    bool empty_slice_20115 = dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096 == (int64_t) 0;
    
    // stdlib/regex.fut:51:41-65
    
    bool i_lte_j_20116 = sle64((int64_t) 0, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096);
    
    // stdlib/regex.fut:51:41-65
    
    bool forwards_ok_20117 = bounds_check_20099 && i_lte_j_20116;
    
    // stdlib/regex.fut:51:41-65
    
    bool ok_or_empty_20118 = empty_slice_20115 || forwards_ok_20117;
    
    // stdlib/regex.fut:51:41-65
    
    bool index_certs_20119;
    
    if (!ok_or_empty_20118) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096, "] out of bounds for array of shape [", (long long) sizze_19941, "].", "-> #0  stdlib/regex.fut:51:41-65\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:66-88
    
    bool ok_or_empty_20121 = bounds_check_20099 || empty_slice_20115;
    
    // stdlib/regex.fut:51:66-88
    
    bool index_certs_20122;
    
    if (!ok_or_empty_20121) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096, "] out of bounds for array of shape [", (long long) sizze_19941, "].", "-> #0  stdlib/regex.fut:51:66-88\n   #1  probes/stdlib-tests.fut:10:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:94:15-22
    
    bool y_20124 = slt64((int64_t) 0, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096);
    
    // stdlib/regex.fut:94:15-22
    
    bool index_certs_20125;
    
    if (!y_20124) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096, "].", "-> #0  stdlib/regex.fut:94:15-22\n   #1  probes/stdlib-tests.fut:10:7-11:20\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:102:42-44
    
    int64_t zt_lhs_20239 = add64((int64_t) 1, dz2081U_16712);
    
    // stdlib/regex.fut:102:45-64
    
    int64_t zp_rhs_20240 = mul64(dz2082U_16713, zt_lhs_20239);
    
    // stdlib/regex.fut:102:29-64
    
    int64_t capacity_20241 = add64(dz2081U_16712, zp_rhs_20240);
    
    if (memblock_alloc(ctx, &mem_20881, capacity_20241, "mem_20881")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:103:63-85
    for (int64_t nest_i_21024 = 0; nest_i_21024 < capacity_20241; nest_i_21024++) {
        ((int8_t *) mem_20881.mem)[nest_i_21024] = (int8_t) 0;
    }
    // stdlib/regex.fut:103:30-119:72
    
    bool replace_res_20243;
    int64_t replace_res_20244;
    int64_t replace_res_20246;
    bool loop_while_20247;
    int64_t position_20248;
    int64_t used_20250;
    
    loop_while_20247 = 1;
    position_20248 = (int64_t) 0;
    used_20250 = (int64_t) 0;
    while (loop_while_20247) {
        // stdlib/regex.fut:101:14-105:47
        
        int64_t loopres_20251;
        int64_t loopres_20252;
        
        if (futrts_find_11321(ctx, &loopres_20251, &loopres_20252, mem_20800, mem_20802, mem_20804, mem_20806, text_mem_20796, compile_res_19953, compile_res_19953, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096, dz2081U_16712, position_20248) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/regex.fut:106:20-59
        
        bool cond_20253 = slt64(loopres_20252, (int64_t) 0);
        
        // stdlib/regex.fut:106:20-59
        
        int64_t finish_20254;
        
        if (cond_20253) {
            finish_20254 = dz2081U_16712;
        } else {
            finish_20254 = loopres_20251;
        }
        // stdlib/regex.fut:107:17-109:15
        
        bool bounds_invalid_upwards_20255 = slt64(finish_20254, position_20248);
        
        // stdlib/regex.fut:107:17-109:15
        
        bool valid_20257 = !bounds_invalid_upwards_20255;
        
        // stdlib/regex.fut:107:17-109:15
        
        bool range_valid_c_20258;
        
        if (!valid_20257) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) position_20248, "..<", (long long) finish_20254, " is invalid.", "-> #0  stdlib/regex.fut:107:17-109:15\n   #1  probes/stdlib-tests.fut:10:7-11:48\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:107:17-109:15
        
        int64_t distance_20256 = sub64(finish_20254, position_20248);
        
        // stdlib/regex.fut:107:17-109:15
        for (int64_t i_20260 = 0; i_20260 < distance_20256; i_20260++) {
            // probes/stdlib-tests.fut:10:7-11:48
            
            int64_t index_primexp_20262 = add64(position_20248, i_20260);
            
            // stdlib/regex.fut:108:21-23
            
            int64_t zm_lhs_20263 = add64(used_20250, index_primexp_20262);
            
            // stdlib/regex.fut:108:23-32
            
            int64_t tmp_20264 = sub64(zm_lhs_20263, position_20248);
            
            // stdlib/regex.fut:108:36-43
            
            bool x_20265 = sle64((int64_t) 0, index_primexp_20262);
            
            // stdlib/regex.fut:108:36-43
            
            bool y_20266 = slt64(index_primexp_20262, dz2081U_16712);
            
            // stdlib/regex.fut:108:36-43
            
            bool bounds_check_20267 = x_20265 && y_20266;
            
            // stdlib/regex.fut:108:36-43
            
            bool index_certs_20268;
            
            if (!bounds_check_20267) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) index_primexp_20262, "] out of bounds for array of shape [", (long long) dz2081U_16712, "].", "-> #0  stdlib/regex.fut:108:36-43\n   #1  probes/stdlib-tests.fut:10:7-11:48\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:108:13-43
            
            bool x_20270 = sle64((int64_t) 0, tmp_20264);
            
            // stdlib/regex.fut:108:13-43
            
            bool y_20271 = slt64(tmp_20264, capacity_20241);
            
            // stdlib/regex.fut:108:13-43
            
            bool bounds_check_20272 = x_20270 && y_20271;
            
            // stdlib/regex.fut:108:13-43
            
            bool index_certs_20273;
            
            if (!bounds_check_20272) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_20264, "] out of bounds for array of shape [", (long long) capacity_20241, "].", "-> #0  stdlib/regex.fut:108:13-43\n   #1  probes/stdlib-tests.fut:10:7-11:48\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:108:36-43
            
            int8_t tmp_20269 = ((int8_t *) text_mem_20796.mem)[index_primexp_20262];
            
            // stdlib/regex.fut:108:13-43
            ((int8_t *) mem_20881.mem)[tmp_20264] = tmp_20269;
        }
        // stdlib/regex.fut:110:22-29
        
        int64_t zm_lhs_20275 = add64(used_20250, finish_20254);
        
        // stdlib/regex.fut:110:29-38
        
        int64_t used_20276 = sub64(zm_lhs_20275, position_20248);
        int64_t loopres_20277;
        int64_t loopres_20279;
        
        if (cond_20253) {
            loopres_20277 = zt_lhs_20239;
            loopres_20279 = used_20276;
        } else {
            // stdlib/regex.fut:112:25-114:23
            for (int64_t i_20281 = 0; i_20281 < dz2082U_16713; i_20281++) {
                // stdlib/regex.fut:113:29-31
                
                int64_t tmp_20283 = add64(used_20276, i_20281);
                
                // stdlib/regex.fut:113:21-49
                
                bool x_20285 = sle64((int64_t) 0, tmp_20283);
                
                // stdlib/regex.fut:113:21-49
                
                bool y_20286 = slt64(tmp_20283, capacity_20241);
                
                // stdlib/regex.fut:113:21-49
                
                bool bounds_check_20287 = x_20285 && y_20286;
                
                // stdlib/regex.fut:113:21-49
                
                bool index_certs_20288;
                
                if (!bounds_check_20287) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_20283, "] out of bounds for array of shape [", (long long) capacity_20241, "].", "-> #0  stdlib/regex.fut:113:21-49\n   #1  probes/stdlib-tests.fut:10:7-11:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:113:35-49
                
                int8_t tmp_20284 = ((int8_t *) replacement_mem_20797.mem)[i_20281];
                
                // stdlib/regex.fut:113:21-49
                ((int8_t *) mem_20881.mem)[tmp_20283] = tmp_20284;
            }
            // stdlib/regex.fut:115:30-49
            
            int64_t used_20290 = add64(dz2082U_16713, used_20276);
            
            // stdlib/regex.fut:116:18-119:72
            
            bool cond_20291 = loopres_20251 == loopres_20252;
            
            // stdlib/regex.fut:116:43-56
            
            bool cond_t_res_20292 = slt64(loopres_20252, dz2081U_16712);
            
            // probes/stdlib-tests.fut:10:7-11:48
            
            bool x_20293 = cond_20291 && cond_t_res_20292;
            
            // stdlib/regex.fut:116:18-119:72
            
            int64_t loopres_f_res_20294;
            int64_t loopres_f_res_20296;
            
            if (x_20293) {
                // stdlib/regex.fut:117:36-46
                
                bool x_20767 = sle64((int64_t) 0, loopres_20252);
                
                // stdlib/regex.fut:117:36-46
                
                bool bounds_check_20768 = cond_t_res_20292 && x_20767;
                
                // stdlib/regex.fut:117:36-46
                
                bool index_certs_20769;
                
                if (!bounds_check_20768) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_20252, "] out of bounds for array of shape [", (long long) dz2081U_16712, "].", "-> #0  stdlib/regex.fut:117:36-46\n   #1  probes/stdlib-tests.fut:10:7-11:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:117:24-46
                
                bool x_20771 = sle64((int64_t) 0, used_20290);
                
                // stdlib/regex.fut:117:24-46
                
                bool y_20772 = slt64(used_20290, capacity_20241);
                
                // stdlib/regex.fut:117:24-46
                
                bool bounds_check_20773 = x_20771 && y_20772;
                
                // stdlib/regex.fut:117:24-46
                
                bool index_certs_20774;
                
                if (!bounds_check_20773) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) used_20290, "] out of bounds for array of shape [", (long long) capacity_20241, "].", "-> #0  stdlib/regex.fut:117:24-46\n   #1  probes/stdlib-tests.fut:10:7-11:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:117:36-46
                
                int8_t tmp_20770 = ((int8_t *) text_mem_20796.mem)[loopres_20252];
                
                // stdlib/regex.fut:117:24-46
                ((int8_t *) mem_20881.mem)[used_20290] = tmp_20770;
                // stdlib/regex.fut:118:28-30
                
                int64_t tmp_20776 = add64((int64_t) 1, loopres_20252);
                
                // stdlib/regex.fut:118:39-41
                
                int64_t tmp_20777 = add64((int64_t) 1, used_20290);
                
                loopres_f_res_20294 = tmp_20776;
                loopres_f_res_20296 = tmp_20777;
            } else {
                // stdlib/regex.fut:119:24-62
                
                int64_t tmp_20308;
                
                if (cond_20291) {
                    // stdlib/regex.fut:119:50-52
                    
                    int64_t tmp_t_res_20778 = add64((int64_t) 1, loopres_20252);
                    
                    tmp_20308 = tmp_t_res_20778;
                } else {
                    tmp_20308 = loopres_20252;
                }
                loopres_f_res_20294 = tmp_20308;
                loopres_f_res_20296 = used_20290;
            }
            loopres_20277 = loopres_f_res_20294;
            loopres_20279 = loopres_f_res_20296;
        }
        // stdlib/regex.fut:104:20-34
        
        bool loop_cond_20310 = sle64(loopres_20277, dz2081U_16712);
        bool loop_while_tmp_21025 = loop_cond_20310;
        int64_t position_tmp_21026 = loopres_20277;
        int64_t used_tmp_21028 = loopres_20279;
        
        loop_while_20247 = loop_while_tmp_21025;
        position_20248 = position_tmp_21026;
        used_20250 = used_tmp_21028;
    }
    replace_res_20243 = loop_while_20247;
    replace_res_20244 = position_20248;
    replace_res_20246 = used_20250;
    // stdlib/regex.fut:120:6-19
    
    bool empty_slice_20311 = replace_res_20246 == (int64_t) 0;
    
    // stdlib/regex.fut:120:6-19
    
    int64_t m_20312 = sub64(replace_res_20246, (int64_t) 1);
    
    // stdlib/regex.fut:120:6-19
    
    bool zzero_leq_i_p_m_t_s_20313 = sle64((int64_t) 0, m_20312);
    
    // stdlib/regex.fut:120:6-19
    
    bool i_p_m_t_s_leq_w_20314 = slt64(m_20312, capacity_20241);
    
    // stdlib/regex.fut:120:6-19
    
    bool i_lte_j_20315 = sle64((int64_t) 0, replace_res_20246);
    
    // stdlib/regex.fut:120:6-19
    
    bool y_20316 = zzero_leq_i_p_m_t_s_20313 && i_p_m_t_s_leq_w_20314;
    
    // stdlib/regex.fut:120:6-19
    
    bool forwards_ok_20317 = i_lte_j_20315 && y_20316;
    
    // stdlib/regex.fut:120:6-19
    
    bool ok_or_empty_20318 = empty_slice_20311 || forwards_ok_20317;
    
    // stdlib/regex.fut:120:6-19
    
    bool index_certs_20319;
    
    if (!ok_or_empty_20318) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) replace_res_20246, "] out of bounds for array of shape [", (long long) capacity_20241, "].", "-> #0  stdlib/regex.fut:120:6-19\n   #1  probes/stdlib-tests.fut:10:7-11:48\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    
    int64_t zm_lhs_20126 = ((int64_t *) mem_20806.mem)[(int64_t) 0];
    int64_t zm_rhs_20129 = ((int64_t *) mem_20804.mem)[(int64_t) 0];
    
    // stdlib/regex.fut:94:22-30
    
    int64_t width_20130 = sub64(zm_lhs_20126, zm_rhs_20129);
    
    // stdlib/regex.fut:95:15-103
    
    bool cond_20131 = slt64((int64_t) 0, width_20130);
    
    // stdlib/regex.fut:95:15-103
    
    bool cond_20132;
    
    if (cond_20131) {
        // stdlib/regex.fut:95:28-42
        
        bool defunc_0_reduce_res_20720;
        bool redout_20783 = 1;
        
        for (int64_t i_20784 = 0; i_20784 < compile_res_19953; i_20784++) {
            int64_t eta_p_20721 = ((int64_t *) mem_20802.mem)[i_20784];
            
            // stdlib/regex.fut:95:32-37
            
            bool lifted_lambda_res_20722 = eta_p_20721 == (int64_t) 1;
            
            // stdlib/regex.fut:95:28-42
            
            bool x_20725 = lifted_lambda_res_20722 && redout_20783;
            bool redout_tmp_21031 = x_20725;
            
            redout_20783 = redout_tmp_21031;
        }
        defunc_0_reduce_res_20720 = redout_20783;
        cond_20132 = defunc_0_reduce_res_20720;
    } else {
        cond_20132 = 0;
    }
    // stdlib/regex.fut:95:15-103
    
    bool fixed_20142;
    
    if (cond_20132) {
        // stdlib/regex.fut:95:46-103
        
        bool defunc_0_reduce_res_20730;
        bool redout_20785 = 1;
        
        for (int64_t i_20786 = 0; i_20786 < dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096; i_20786++) {
            int64_t zm_lhs_20736 = ((int64_t *) mem_20806.mem)[i_20786];
            int64_t zm_rhs_20737 = ((int64_t *) mem_20804.mem)[i_20786];
            
            // stdlib/regex.fut:95:64-72
            
            int64_t zeze_lhs_20738 = sub64(zm_lhs_20736, zm_rhs_20737);
            
            // stdlib/regex.fut:95:73-81
            
            bool lifted_lambda_res_20739 = zeze_lhs_20738 == width_20130;
            
            // stdlib/regex.fut:95:46-103
            
            bool x_20742 = lifted_lambda_res_20739 && redout_20785;
            bool redout_tmp_21032 = x_20742;
            
            redout_20785 = redout_tmp_21032;
        }
        defunc_0_reduce_res_20730 = redout_20785;
        fixed_20142 = defunc_0_reduce_res_20730;
    } else {
        fixed_20142 = 0;
    }
    // stdlib/regex.fut:96:6-100:13
    
    bool cond_20163 = !fixed_20142;
    
    // stdlib/regex.fut:97:88-94
    
    int64_t zp_lhs_20164 = sub64(dz2081U_16712, width_20130);
    
    // stdlib/regex.fut:97:94-96
    
    int64_t max_arg1_20165 = add64((int64_t) 1, zp_lhs_20164);
    
    // probes/stdlib-tests.fut:10:7-11:20
    
    int64_t max_res_20166 = smax64((int64_t) 0, max_arg1_20165);
    
    // stdlib/regex.fut:96:6-100:13
    
    int64_t count_res_20167;
    
    if (cond_20163) {
        // stdlib/regex.fut:81:27-84:61
        
        bool count_general_res_20747;
        int64_t count_general_res_20748;
        int64_t count_general_res_20749;
        bool loop_while_20750;
        int64_t position_20751;
        int64_t count_20752;
        
        loop_while_20750 = 1;
        position_20751 = (int64_t) 0;
        count_20752 = (int64_t) 0;
        while (loop_while_20750) {
            // stdlib/regex.fut:80:20-82:45
            
            int64_t loopres_20753;
            int64_t loopres_20754;
            
            if (futrts_find_11321(ctx, &loopres_20753, &loopres_20754, mem_20800, mem_20802, mem_20804, mem_20806, text_mem_20796, compile_res_19953, compile_res_19953, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096, dz2081U_16712, position_20751) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/regex.fut:83:8-84:61
            
            bool cond_20755 = slt64(loopres_20754, (int64_t) 0);
            
            // stdlib/regex.fut:83:8-84:61
            
            int64_t loopres_20756;
            int64_t loopres_20757;
            
            if (cond_20755) {
                loopres_20756 = zt_lhs_20239;
                loopres_20757 = count_20752;
            } else {
                // stdlib/regex.fut:84:14-52
                
                bool cond_20759 = loopres_20754 == loopres_20753;
                
                // stdlib/regex.fut:84:14-52
                
                int64_t tmp_20760;
                
                if (cond_20759) {
                    // stdlib/regex.fut:84:40-42
                    
                    int64_t tmp_t_res_20761 = add64((int64_t) 1, loopres_20754);
                    
                    tmp_20760 = tmp_t_res_20761;
                } else {
                    tmp_20760 = loopres_20754;
                }
                // stdlib/regex.fut:84:58-60
                
                int64_t tmp_20762 = add64((int64_t) 1, count_20752);
                
                loopres_20756 = tmp_20760;
                loopres_20757 = tmp_20762;
            }
            // stdlib/regex.fut:81:78-92
            
            bool loop_cond_20763 = sle64(loopres_20756, dz2081U_16712);
            bool loop_while_tmp_21033 = loop_cond_20763;
            int64_t position_tmp_21034 = loopres_20756;
            int64_t count_tmp_21035 = loopres_20757;
            
            loop_while_20750 = loop_while_tmp_21033;
            position_20751 = position_tmp_21034;
            count_20752 = count_tmp_21035;
        }
        count_general_res_20747 = loop_while_20750;
        count_general_res_20748 = position_20751;
        count_general_res_20749 = count_20752;
        count_res_20167 = count_general_res_20749;
    } else {
        // stdlib/regex.fut:97:60-97
        
        bool bounds_invalid_upwards_20187 = slt64(max_res_20166, (int64_t) 0);
        
        // stdlib/regex.fut:97:60-97
        
        bool valid_20188 = !bounds_invalid_upwards_20187;
        
        // stdlib/regex.fut:97:60-97
        
        bool range_valid_c_20189;
        
        if (!valid_20188) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) (int64_t) 0, "..", (long long) (int64_t) 1, "..<", (long long) max_res_20166, " is invalid.", "-> #0  stdlib/regex.fut:97:60-97\n   #1  probes/stdlib-tests.fut:10:7-11:20\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        if (mem_20866_cached_sizze_21099 < max_res_20166) {
            err = lexical_realloc(ctx, &mem_20866, &mem_20866_cached_sizze_21099, max_res_20166);
            if (err != FUTHARK_SUCCESS)
                goto cleanup;
        }
        // stdlib/regex.fut:97:19-99
        for (int64_t i_20789 = 0; i_20789 < max_res_20166; i_20789++) {
            // stdlib/regex.fut:87:3-92:12
            
            bool fixed_match_res_20193;
            bool found_20195 = 0;
            
            for (int64_t branch_20194 = 0; branch_20194 < dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_20096; branch_20194++) {
                // stdlib/regex.fut:88:5-92:12
                
                bool loopres_20196;
                
                if (found_20195) {
                    loopres_20196 = 1;
                } else {
                    int64_t first_20197 = ((int64_t *) mem_20804.mem)[branch_20194];
                    
                    // stdlib/regex.fut:90:21-91:80
                    
                    bool loopres_f_res_20199;
                    int64_t loopres_f_res_20200;
                    bool loopres_f_res_20201;
                    bool loop_while_20202;
                    int64_t i_20203;
                    bool ok_20204;
                    
                    loop_while_20202 = cond_20131;
                    i_20203 = (int64_t) 0;
                    ok_20204 = 1;
                    while (loop_while_20202) {
                        // stdlib/regex.fut:91:11-13
                        
                        int64_t tmp_20205 = add64((int64_t) 1, i_20203);
                        
                        // stdlib/regex.fut:91:75-77
                        
                        int64_t token_accepts_arg2_20206 = add64(i_20203, i_20789);
                        
                        // stdlib/regex.fut:91:62-78
                        
                        int8_t token_accepts_arg2_20207 = ((int8_t *) text_mem_20796.mem)[token_accepts_arg2_20206];
                        
                        // stdlib/regex.fut:91:47-49
                        
                        int64_t token_accepts_arg1_20208 = add64(first_20197, i_20203);
                        
                        // stdlib/regex.fut:55:26-34
                        
                        int64_t u8_res_20209 = zext_i8_i64(token_accepts_arg2_20207);
                        
                        // stdlib/regex.fut:55:34-37
                        
                        int64_t zgzg_lhs_20210 = sdiv64(u8_res_20209, (int64_t) 64);
                        int64_t zgzg_lhs_20211 = ((int64_t *) mem_20800.mem)[token_accepts_arg1_20208 * (int64_t) 4 + zgzg_lhs_20210];
                        
                        // stdlib/regex.fut:55:51-54
                        
                        int8_t u8_arg0_20212 = umod8(token_accepts_arg2_20207, (int8_t) 64);
                        
                        // stdlib/regex.fut:55:42-54
                        
                        int64_t u8_res_20213 = zext_i8_i64(u8_arg0_20212);
                        
                        // stdlib/regex.fut:55:39-55
                        
                        int64_t za_lhs_20214 = lshr64(zgzg_lhs_20211, u8_res_20213);
                        
                        // stdlib/regex.fut:55:56-58
                        
                        int64_t znze_lhs_20215 = (int64_t) 1 & za_lhs_20214;
                        
                        // stdlib/regex.fut:55:60-64
                        
                        bool token_accepts_res_20216 = znze_lhs_20215 == (int64_t) 0;
                        
                        // stdlib/regex.fut:55:60-64
                        
                        bool token_accepts_res_20217 = !token_accepts_res_20216;
                        
                        // stdlib/regex.fut:90:53-68
                        
                        bool cond_20218 = slt64(tmp_20205, width_20130);
                        
                        // stdlib/regex.fut:93:12-97:57
                        
                        bool x_20219 = token_accepts_res_20217 && cond_20218;
                        bool loop_while_tmp_21038 = x_20219;
                        int64_t i_tmp_21039 = tmp_20205;
                        bool ok_tmp_21040 = token_accepts_res_20217;
                        
                        loop_while_20202 = loop_while_tmp_21038;
                        i_20203 = i_tmp_21039;
                        ok_20204 = ok_tmp_21040;
                    }
                    loopres_f_res_20199 = loop_while_20202;
                    loopres_f_res_20200 = i_20203;
                    loopres_f_res_20201 = ok_20204;
                    loopres_20196 = loopres_f_res_20201;
                }
                
                bool found_tmp_21037 = loopres_20196;
                
                found_20195 = found_tmp_21037;
            }
            fixed_match_res_20193 = found_20195;
            ((bool *) mem_20866)[i_20789] = fixed_match_res_20193;
        }
        // stdlib/regex.fut:98:22-99:60
        
        bool loop_cond_20221 = slt64((int64_t) 0, max_res_20166);
        
        // stdlib/regex.fut:98:22-99:60
        
        bool count_res_f_res_20222;
        int64_t count_res_f_res_20223;
        int64_t count_res_f_res_20224;
        bool loop_while_20225;
        int64_t i_20226;
        int64_t total_20227;
        
        loop_while_20225 = loop_cond_20221;
        i_20226 = (int64_t) 0;
        total_20227 = (int64_t) 0;
        while (loop_while_20225) {
            // stdlib/regex.fut:99:7-60
            
            bool x_20228 = sle64((int64_t) 0, i_20226);
            
            // stdlib/regex.fut:99:7-60
            
            bool y_20229 = slt64(i_20226, max_res_20166);
            
            // stdlib/regex.fut:99:7-60
            
            bool bounds_check_20230 = x_20228 && y_20229;
            
            // stdlib/regex.fut:99:7-60
            
            bool index_certs_20231;
            
            if (!bounds_check_20230) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_20226, "] out of bounds for array of shape [", (long long) max_res_20166, "].", "-> #0  stdlib/regex.fut:99:7-60\n   #1  probes/stdlib-tests.fut:10:7-11:20\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:99:7-60
            
            bool cond_20232 = ((bool *) mem_20866)[i_20226];
            
            // stdlib/regex.fut:99:7-60
            
            int64_t loopres_20233;
            int64_t loopres_20234;
            
            if (cond_20232) {
                // stdlib/regex.fut:99:28-34
                
                int64_t tmp_20764 = add64(width_20130, i_20226);
                
                // stdlib/regex.fut:99:40-42
                
                int64_t tmp_20765 = add64((int64_t) 1, total_20227);
                
                loopres_20233 = tmp_20764;
                loopres_20234 = tmp_20765;
            } else {
                // stdlib/regex.fut:99:51-53
                
                int64_t tmp_20237 = add64((int64_t) 1, i_20226);
                
                loopres_20233 = tmp_20237;
                loopres_20234 = total_20227;
            }
            // stdlib/regex.fut:98:59-75
            
            bool loop_cond_20238 = slt64(loopres_20233, max_res_20166);
            bool loop_while_tmp_21041 = loop_cond_20238;
            int64_t i_tmp_21042 = loopres_20233;
            int64_t total_tmp_21043 = loopres_20234;
            
            loop_while_20225 = loop_while_tmp_21041;
            i_20226 = i_tmp_21042;
            total_20227 = total_tmp_21043;
        }
        count_res_f_res_20222 = loop_while_20225;
        count_res_f_res_20223 = i_20226;
        count_res_f_res_20224 = total_20227;
        count_res_20167 = count_res_f_res_20224;
    }
    if (memblock_unref(ctx, &mem_20800, "mem_20800") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_20802, "mem_20802") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_20804, "mem_20804") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_20806, "mem_20806") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20998, &mem_20881, "mem_20881") != 0)
        return 1;
    prim_out_20999 = replace_res_20246;
    prim_out_21000 = count_res_20167;
    if (memblock_set(ctx, &*mem_out_p_21096, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    *out_prim_out_21097 = prim_out_20999;
    *out_prim_out_21098 = prim_out_21000;
    
  cleanup:
    {
        free(mem_20866);
        if (memblock_unref(ctx, &mem_20881, "mem_20881") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20821, "ext_mem_20821") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20820, "mem_20820") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20822, "mem_20822") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20823, "ext_mem_20823") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20840, "mem_20840") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20849, "ext_mem_20849") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21015, "mem_param_tmp_21015") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21019, "mem_param_tmp_21019") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20833, "ext_mem_20833") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20830, "mem_param_20830") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20836, "ext_mem_20836") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20827, "mem_param_20827") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20839, "ext_mem_20839") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20824, "mem_20824") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20852, "ext_mem_20852") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20806, "mem_20806") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20804, "mem_20804") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20802, "mem_20802") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20800, "mem_20800") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_wrap_test(struct futhark_context *ctx, struct memblock *mem_out_p_21100, int64_t *out_prim_out_21101, struct memblock text_mem_20795, int64_t dz2080U_17737, int64_t width_17738)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_20796;
    
    mem_20796.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t prim_out_20999;
    
    // stdlib/common.fut:22:17-20
    
    bool tmp_18251 = slt64((int64_t) 0, width_17738);
    
    // stdlib/common.fut:23:38-44
    
    int64_t zm_lhs_18252 = add64(dz2080U_17737, width_17738);
    
    // stdlib/common.fut:23:44-46
    
    int64_t zs_lhs_18253 = sub64(zm_lhs_18252, (int64_t) 1);
    
    // stdlib/common.fut:23:47-53
    
    bool zzero_18254 = width_17738 == (int64_t) 0;
    
    // stdlib/common.fut:23:47-53
    
    bool nonzzero_18255 = !zzero_18254;
    
    // stdlib/common.fut:23:47-53
    
    bool nonzzero_cert_18256;
    
    if (!nonzzero_18255) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/common.fut:23:47-53\n   #1  probes/stdlib-tests.fut:29:43-58\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/common.fut:23:47-53
    
    int64_t zp_rhs_18257 = sdiv64(zs_lhs_18253, width_17738);
    
    // stdlib/common.fut:23:25-53
    
    int64_t sizze_18258 = add64(dz2080U_17737, zp_rhs_18257);
    
    // stdlib/common.fut:26:77-86
    
    bool bounds_invalid_upwards_18259 = slt64(sizze_18258, (int64_t) 0);
    
    // stdlib/common.fut:26:77-86
    
    bool valid_18260 = !bounds_invalid_upwards_18259;
    
    // stdlib/common.fut:26:77-86
    
    bool range_valid_c_18261;
    
    if (!valid_18260) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) (int64_t) 0, "..", (long long) (int64_t) 1, "..<", (long long) sizze_18258, " is invalid.", "-> #0  stdlib/common.fut:26:77-86\n   #1  probes/stdlib-tests.fut:29:43-58\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/common.fut:22:3-26:87
    
    bool assert_c_18263;
    
    if (!tmp_18251) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "Assertion is false: (width > 0)", "-> #0  stdlib/common.fut:22:3-26:87\n   #1  probes/stdlib-tests.fut:29:43-58\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/common.fut:25:21-23
    
    int64_t zs_rhs_18264 = add64((int64_t) 1, width_17738);
    
    // stdlib/common.fut:25:14-23
    
    bool zzero_18265 = zs_rhs_18264 == (int64_t) 0;
    
    // stdlib/common.fut:25:14-23
    
    bool nonzzero_18266 = !zzero_18265;
    
    // stdlib/common.fut:25:14-23
    
    bool nonzzero_cert_18267;
    
    if (!nonzzero_18266) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/common.fut:25:14-23\n   #1  probes/stdlib-tests.fut:29:43-58\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    if (memblock_alloc(ctx, &mem_20796, sizze_18258, "mem_20796")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/common.fut:22:3-26:87
    for (int64_t i_20781 = 0; i_20781 < sizze_18258; i_20781++) {
        // stdlib/common.fut:25:14-23
        
        int64_t zt_lhs_18270 = sdiv64(i_20781, zs_rhs_18264);
        
        // stdlib/common.fut:25:24-30
        
        int64_t zp_lhs_18271 = mul64(width_17738, zt_lhs_18270);
        
        // stdlib/common.fut:25:32-41
        
        int64_t zp_rhs_18272 = smod64(i_20781, zs_rhs_18264);
        
        // stdlib/common.fut:25:30-41
        
        int64_t j_18273 = add64(zp_lhs_18271, zp_rhs_18272);
        
        // stdlib/common.fut:26:8-74
        
        bool cond_18274 = zp_rhs_18272 == width_17738;
        
        // stdlib/common.fut:26:37-51
        
        bool cond_f_res_18275 = sle64(dz2080U_17737, j_18273);
        
        // probes/stdlib-tests.fut:29:43-58
        
        bool x_18276 = !cond_18274;
        
        // probes/stdlib-tests.fut:29:43-58
        
        bool y_18277 = cond_f_res_18275 && x_18276;
        
        // probes/stdlib-tests.fut:29:43-58
        
        bool cond_18278 = cond_18274 || y_18277;
        
        // stdlib/common.fut:26:8-74
        
        int8_t lifted_lambda_res_18279;
        
        if (cond_18278) {
            lifted_lambda_res_18279 = (int8_t) 10;
        } else {
            // stdlib/common.fut:26:67-74
            
            bool x_18280 = sle64((int64_t) 0, j_18273);
            
            // stdlib/common.fut:26:67-74
            
            bool y_18281 = slt64(j_18273, dz2080U_17737);
            
            // stdlib/common.fut:26:67-74
            
            bool bounds_check_18282 = x_18280 && y_18281;
            
            // stdlib/common.fut:26:67-74
            
            bool index_certs_18283;
            
            if (!bounds_check_18282) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) j_18273, "] out of bounds for array of shape [", (long long) dz2080U_17737, "].", "-> #0  stdlib/common.fut:26:67-74\n   #1  probes/stdlib-tests.fut:29:43-58\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/common.fut:26:67-74
            
            int8_t lifted_lambda_res_f_res_18284 = ((int8_t *) text_mem_20795.mem)[j_18273];
            
            lifted_lambda_res_18279 = lifted_lambda_res_f_res_18284;
        }
        ((int8_t *) mem_20796.mem)[i_20781] = lifted_lambda_res_18279;
    }
    if (memblock_set(ctx, &mem_out_20998, &mem_20796, "mem_20796") != 0)
        return 1;
    prim_out_20999 = sizze_18258;
    if (memblock_set(ctx, &*mem_out_p_21100, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    *out_prim_out_21101 = prim_out_20999;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_20796, "mem_20796") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_find_11321(struct futhark_context *ctx, int64_t *out_prim_out_21102, int64_t *out_prim_out_21103, struct memblock z30U_mem_20795, struct memblock z31U_mem_20796, struct memblock z32U_mem_20797, struct memblock z33U_mem_20798, struct memblock text_mem_20799, int64_t dz2080U_15867, int64_t dz2081U_15868, int64_t dz2082U_15869, int64_t dz2083U_15870, int64_t dz2080U_15871, int64_t start_15877)
{
    (void) ctx;
    
    int err = 0;
    int64_t prim_out_20998;
    int64_t prim_out_20999;
    
    // stdlib/regex.fut:69:24-78:56
    
    bool cond_15883 = sle64(start_15877, dz2080U_15871);
    
    // stdlib/regex.fut:69:24-78:56
    
    bool find_res_15886;
    int64_t find_res_15887;
    int64_t find_res_15888;
    bool loop_while_15889;
    int64_t position_15890;
    int64_t end_15891;
    
    loop_while_15889 = cond_15883;
    position_15890 = start_15877;
    end_15891 = (int64_t) -1;
    while (loop_while_15889) {
        // stdlib/regex.fut:71:17-77:82
        
        int64_t end_15894;
        int64_t result_15896 = (int64_t) -1;
        
        for (int64_t branch_15895 = 0; branch_15895 < dz2082U_15869; branch_15895++) {
            // stdlib/regex.fut:72:9-77:82
            
            bool cond_15897 = sle64((int64_t) 0, result_15896);
            
            // stdlib/regex.fut:72:9-77:82
            
            int64_t loopres_15898;
            
            if (cond_15897) {
                loopres_15898 = result_15896;
            } else {
                // stdlib/regex.fut:74:25-37
                
                bool y_15906 = slt64(branch_15895, dz2083U_15870);
                
                // stdlib/regex.fut:74:25-37
                
                bool index_certs_15908;
                
                if (!y_15906) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) branch_15895, "] out of bounds for array of shape [", (long long) dz2083U_15870, "].", "-> #0  stdlib/regex.fut:74:25-37\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:73:26-38
                
                int64_t first_15903 = ((int64_t *) z32U_mem_20797.mem)[branch_15895];
                
                // stdlib/regex.fut:74:25-37
                
                int64_t last_15909 = ((int64_t *) z33U_mem_20798.mem)[branch_15895];
                
                // stdlib/regex.fut:75:29-76:81
                
                bool cond_15911 = first_15903 == last_15909;
                
                // stdlib/regex.fut:75:29-76:81
                
                bool cond_15912;
                
                if (cond_15911) {
                    cond_15912 = 1;
                } else {
                    // stdlib/regex.fut:75:46-57
                    
                    bool x_15913 = sle64((int64_t) 0, first_15903);
                    
                    // stdlib/regex.fut:75:46-57
                    
                    bool y_15914 = slt64(first_15903, dz2081U_15868);
                    
                    // stdlib/regex.fut:75:46-57
                    
                    bool bounds_check_15915 = x_15913 && y_15914;
                    
                    // stdlib/regex.fut:75:46-57
                    
                    bool index_certs_15916;
                    
                    if (!bounds_check_15915) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) first_15903, "] out of bounds for array of shape [", (long long) dz2081U_15868, "].", "-> #0  stdlib/regex.fut:75:46-57\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:75:46-57
                    
                    int64_t zeze_lhs_15917 = ((int64_t *) z31U_mem_20796.mem)[first_15903];
                    
                    // stdlib/regex.fut:75:58-62
                    
                    bool cond_f_res_15919 = zeze_lhs_15917 == (int64_t) 0;
                    
                    cond_15912 = cond_f_res_15919;
                }
                // stdlib/regex.fut:75:29-76:81
                
                bool cond_15920;
                
                if (cond_15912) {
                    cond_15920 = 1;
                } else {
                    // stdlib/regex.fut:75:66-77
                    
                    bool x_15921 = sle64((int64_t) 0, first_15903);
                    
                    // stdlib/regex.fut:75:66-77
                    
                    bool y_15922 = slt64(first_15903, dz2081U_15868);
                    
                    // stdlib/regex.fut:75:66-77
                    
                    bool bounds_check_15923 = x_15921 && y_15922;
                    
                    // stdlib/regex.fut:75:66-77
                    
                    bool index_certs_15924;
                    
                    if (!bounds_check_15923) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) first_15903, "] out of bounds for array of shape [", (long long) dz2081U_15868, "].", "-> #0  stdlib/regex.fut:75:66-77\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:75:66-77
                    
                    int64_t zeze_lhs_15925 = ((int64_t *) z31U_mem_20796.mem)[first_15903];
                    
                    // stdlib/regex.fut:75:78-82
                    
                    bool cond_f_res_15927 = zeze_lhs_15925 == (int64_t) 3;
                    
                    cond_15920 = cond_f_res_15927;
                }
                // stdlib/regex.fut:75:29-76:81
                
                bool possible_15928;
                
                if (cond_15920) {
                    possible_15928 = 1;
                } else {
                    // stdlib/regex.fut:76:17-80
                    
                    bool cond_15931 = slt64(position_15890, dz2080U_15871);
                    
                    // stdlib/regex.fut:76:17-80
                    
                    bool possible_f_res_15932;
                    
                    if (cond_15931) {
                        // stdlib/regex.fut:76:66-80
                        
                        bool x_20386 = sle64((int64_t) 0, position_15890);
                        
                        // stdlib/regex.fut:76:66-80
                        
                        bool bounds_check_20387 = cond_15931 && x_20386;
                        
                        // stdlib/regex.fut:76:66-80
                        
                        bool index_certs_20388;
                        
                        if (!bounds_check_20387) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) position_15890, "] out of bounds for array of shape [", (long long) dz2080U_15871, "].", "-> #0  stdlib/regex.fut:76:66-80\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:76:66-80
                        
                        int8_t token_accepts_arg2_20389 = ((int8_t *) text_mem_20799.mem)[position_15890];
                        
                        // stdlib/regex.fut:55:26-34
                        
                        int64_t u8_res_20390 = zext_i8_i64(token_accepts_arg2_20389);
                        
                        // stdlib/regex.fut:55:34-37
                        
                        int64_t zgzg_lhs_20391 = sdiv64(u8_res_20390, (int64_t) 64);
                        
                        // stdlib/regex.fut:55:15-38
                        
                        int64_t zgzg_lhs_20392 = ((int64_t *) z30U_mem_20795.mem)[first_15903 * (int64_t) 4 + zgzg_lhs_20391];
                        
                        // stdlib/regex.fut:55:51-54
                        
                        int8_t u8_arg0_20393 = umod8(token_accepts_arg2_20389, (int8_t) 64);
                        
                        // stdlib/regex.fut:55:42-54
                        
                        int64_t u8_res_20394 = zext_i8_i64(u8_arg0_20393);
                        
                        // stdlib/regex.fut:55:39-55
                        
                        int64_t za_lhs_20395 = lshr64(zgzg_lhs_20392, u8_res_20394);
                        
                        // stdlib/regex.fut:55:56-58
                        
                        int64_t znze_lhs_20396 = (int64_t) 1 & za_lhs_20395;
                        
                        // stdlib/regex.fut:55:60-64
                        
                        bool token_accepts_res_20397 = znze_lhs_20396 == (int64_t) 0;
                        
                        // stdlib/regex.fut:55:60-64
                        
                        bool token_accepts_res_20398 = !token_accepts_res_20397;
                        
                        possible_f_res_15932 = token_accepts_res_20398;
                    } else {
                        possible_f_res_15932 = 0;
                    }
                    possible_15928 = possible_f_res_15932;
                }
                // stdlib/regex.fut:77:17-82
                
                int64_t loopres_f_res_15941;
                
                if (possible_15928) {
                    // stdlib/regex.fut:68:11-77:74
                    
                    int64_t loopres_f_res_t_res_20399;
                    
                    if (futrts_match_branch_11323(ctx, &loopres_f_res_t_res_20399, z30U_mem_20795, z31U_mem_20796, z32U_mem_20797, z33U_mem_20798, text_mem_20799, dz2080U_15867, dz2081U_15868, dz2082U_15869, dz2083U_15870, dz2080U_15871, first_15903, last_15909, position_15890) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    loopres_f_res_15941 = loopres_f_res_t_res_20399;
                } else {
                    loopres_f_res_15941 = (int64_t) -1;
                }
                loopres_15898 = loopres_f_res_15941;
            }
            
            int64_t result_tmp_21003 = loopres_15898;
            
            result_15896 = result_tmp_21003;
        }
        end_15894 = result_15896;
        // stdlib/regex.fut:78:11-51
        
        bool cond_15945 = slt64(end_15894, (int64_t) 0);
        
        // stdlib/regex.fut:78:11-51
        
        int64_t tmp_15946;
        
        if (cond_15945) {
            // stdlib/regex.fut:78:35-37
            
            int64_t tmp_t_res_20400 = add64((int64_t) 1, position_15890);
            
            tmp_15946 = tmp_t_res_20400;
        } else {
            tmp_15946 = position_15890;
        }
        // stdlib/regex.fut:70:11-45
        
        bool cond_15953 = sle64(tmp_15946, dz2080U_15871);
        bool x_17745 = cond_15945 && cond_15953;
        bool loop_while_tmp_21000 = x_17745;
        int64_t position_tmp_21001 = tmp_15946;
        int64_t end_tmp_21002 = end_15894;
        
        loop_while_15889 = loop_while_tmp_21000;
        position_15890 = position_tmp_21001;
        end_15891 = end_tmp_21002;
    }
    find_res_15886 = loop_while_15889;
    find_res_15887 = position_15890;
    find_res_15888 = end_15891;
    prim_out_20998 = find_res_15887;
    prim_out_20999 = find_res_15888;
    *out_prim_out_21102 = prim_out_20998;
    *out_prim_out_21103 = prim_out_20999;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_grow_11344(struct futhark_context *ctx, struct memblock *mem_out_p_21104, struct memblock *mem_out_p_21105, struct memblock *mem_out_p_21106, int64_t *out_prim_out_21107, int64_t *out_prim_out_21108, struct memblock z30U_mem_20795, struct memblock z30U_mem_20796, struct memblock z30U_mem_20797, int64_t dz2080U_16954, int64_t z31U_16958)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_param_tmp_21005;
    
    mem_param_tmp_21005.references = NULL;
    
    struct memblock mem_param_tmp_21004;
    
    mem_param_tmp_21004.references = NULL;
    
    struct memblock mem_param_tmp_21003;
    
    mem_param_tmp_21003.references = NULL;
    
    struct memblock ext_mem_20816;
    
    ext_mem_20816.references = NULL;
    
    struct memblock ext_mem_20817;
    
    ext_mem_20817.references = NULL;
    
    struct memblock ext_mem_20818;
    
    ext_mem_20818.references = NULL;
    
    struct memblock ext_mem_20821;
    
    ext_mem_20821.references = NULL;
    
    struct memblock ext_mem_20824;
    
    ext_mem_20824.references = NULL;
    
    struct memblock ext_mem_20827;
    
    ext_mem_20827.references = NULL;
    
    struct memblock mem_param_20809;
    
    mem_param_20809.references = NULL;
    
    struct memblock mem_param_20806;
    
    mem_param_20806.references = NULL;
    
    struct memblock mem_param_20803;
    
    mem_param_20803.references = NULL;
    
    struct memblock ext_mem_20834;
    
    ext_mem_20834.references = NULL;
    
    struct memblock ext_mem_20835;
    
    ext_mem_20835.references = NULL;
    
    struct memblock ext_mem_20836;
    
    ext_mem_20836.references = NULL;
    
    struct memblock ext_mem_20798;
    
    ext_mem_20798.references = NULL;
    
    struct memblock ext_mem_20799;
    
    ext_mem_20799.references = NULL;
    
    struct memblock ext_mem_20800;
    
    ext_mem_20800.references = NULL;
    
    struct memblock mem_out_21000;
    
    mem_out_21000.references = NULL;
    
    struct memblock mem_out_20999;
    
    mem_out_20999.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t prim_out_21001;
    int64_t prim_out_21002;
    
    // stdlib/hashmap.fut:26:26-40
    
    int64_t dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_16964 = mul64((int64_t) 2, dz2080U_16954);
    
    // stdlib/hashmap.fut:26:19-40
    
    int64_t grow_res_16973;
    
    if (futrts_empty_7765(ctx, &ext_mem_20800, &ext_mem_20799, &ext_mem_20798, &grow_res_16973, dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_16964) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:26:6-27:73
    
    int64_t grow_res_16991;
    int64_t fresh_16996;
    
    if (memblock_set(ctx, &mem_param_20803, &ext_mem_20800, "ext_mem_20800") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_20806, &ext_mem_20799, "ext_mem_20799") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_20809, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    fresh_16996 = grow_res_16973;
    for (int64_t i_16992 = 0; i_16992 < dz2080U_16954; i_16992++) {
        // stdlib/hashmap.fut:27:11-19
        
        bool loopres_17003 = ((bool *) z30U_mem_20797.mem)[i_16992];
        
        // stdlib/hashmap.fut:27:52-60
        
        int64_t loopres_t_res_20386;
        
        if (loopres_17003) {
            int64_t x_20977 = ((int64_t *) z30U_mem_20796.mem)[i_16992];
            
            loopres_t_res_20386 = x_20977;
        } else {
            loopres_t_res_20386 = (int64_t) 0;
        }
        // stdlib/hashmap.fut:27:41-49
        
        int64_t loopres_t_res_20387;
        
        if (loopres_17003) {
            int64_t x_20979 = ((int64_t *) z30U_mem_20795.mem)[i_16992];
            
            loopres_t_res_20387 = x_20979;
        } else {
            loopres_t_res_20387 = (int64_t) 0;
        }
        
        int64_t loopres_17010;
        
        if (loopres_17003) {
            // stdlib/hashmap.fut:27:27-62
            
            int64_t loopres_t_res_20391;
            
            if (futrts_put_raw_11346(ctx, &ext_mem_20818, &ext_mem_20817, &ext_mem_20816, &loopres_t_res_20391, mem_param_20803, mem_param_20806, mem_param_20809, dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_16964, fresh_16996, loopres_t_res_20387, loopres_t_res_20386) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_20827, &ext_mem_20818, "ext_mem_20818") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20824, &ext_mem_20817, "ext_mem_20817") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20821, &ext_mem_20816, "ext_mem_20816") != 0)
                return 1;
            loopres_17010 = loopres_t_res_20391;
        } else {
            if (memblock_set(ctx, &ext_mem_20827, &mem_param_20803, "mem_param_20803") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20824, &mem_param_20806, "mem_param_20806") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_20821, &mem_param_20809, "mem_param_20809") != 0)
                return 1;
            loopres_17010 = fresh_16996;
        }
        if (memblock_set(ctx, &mem_param_tmp_21003, &ext_mem_20827, "ext_mem_20827") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_21004, &ext_mem_20824, "ext_mem_20824") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_21005, &ext_mem_20821, "ext_mem_20821") != 0)
            return 1;
        
        int64_t fresh_tmp_21009 = loopres_17010;
        
        if (memblock_set(ctx, &mem_param_20803, &mem_param_tmp_21003, "mem_param_tmp_21003") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20806, &mem_param_tmp_21004, "mem_param_tmp_21004") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_20809, &mem_param_tmp_21005, "mem_param_tmp_21005") != 0)
            return 1;
        fresh_16996 = fresh_tmp_21009;
    }
    if (memblock_set(ctx, &ext_mem_20836, &mem_param_20803, "mem_param_20803") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_20835, &mem_param_20806, "mem_param_20806") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_20834, &mem_param_20809, "mem_param_20809") != 0)
        return 1;
    grow_res_16991 = fresh_16996;
    if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20799, "ext_mem_20799") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_20800, "ext_mem_20800") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20998, &ext_mem_20836, "ext_mem_20836") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20999, &ext_mem_20835, "ext_mem_20835") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_21000, &ext_mem_20834, "ext_mem_20834") != 0)
        return 1;
    prim_out_21001 = dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_16964;
    prim_out_21002 = grow_res_16991;
    if (memblock_set(ctx, &*mem_out_p_21104, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21105, &mem_out_20999, "mem_out_20999") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21106, &mem_out_21000, "mem_out_21000") != 0)
        return 1;
    *out_prim_out_21107 = prim_out_21001;
    *out_prim_out_21108 = prim_out_21002;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_param_tmp_21005, "mem_param_tmp_21005") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21004, "mem_param_tmp_21004") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_21003, "mem_param_tmp_21003") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20816, "ext_mem_20816") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20817, "ext_mem_20817") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20818, "ext_mem_20818") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20821, "ext_mem_20821") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20824, "ext_mem_20824") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20827, "ext_mem_20827") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20809, "mem_param_20809") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20806, "mem_param_20806") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_20803, "mem_param_20803") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20834, "ext_mem_20834") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20835, "ext_mem_20835") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20836, "ext_mem_20836") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20798, "ext_mem_20798") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20799, "ext_mem_20799") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_20800, "ext_mem_20800") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_21000, "mem_out_21000") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20999, "mem_out_20999") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_literal_11305(struct futhark_context *ctx, int8_t *out_prim_out_21109, int64_t *out_prim_out_21110, struct memblock pattern_mem_20795, int64_t dz2080U_15180, int64_t p_15182)
{
    (void) ctx;
    
    int err = 0;
    int8_t prim_out_20998;
    int64_t prim_out_20999;
    
    // stdlib/regex.fut:8:6-16
    
    bool x_15183 = sle64((int64_t) 0, p_15182);
    
    // stdlib/regex.fut:8:6-16
    
    bool y_15184 = slt64(p_15182, dz2080U_15180);
    
    // stdlib/regex.fut:8:6-16
    
    bool bounds_check_15185 = x_15183 && y_15184;
    
    // stdlib/regex.fut:8:6-16
    
    bool index_certs_15186;
    
    if (!bounds_check_15185) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) p_15182, "] out of bounds for array of shape [", (long long) dz2080U_15180, "].", "-> #0  stdlib/regex.fut:8:6-16\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:8:6-16
    
    int8_t zeze_lhs_15187 = ((int8_t *) pattern_mem_20795.mem)[p_15182];
    
    // stdlib/regex.fut:8:3-9:24
    
    bool cond_15189 = zeze_lhs_15187 == (int8_t) 92;
    
    // stdlib/regex.fut:8:27-29
    
    int64_t zl_lhs_15191 = add64((int64_t) 1, p_15182);
    
    // stdlib/regex.fut:8:30-46
    
    bool cond_t_res_15195 = slt64(zl_lhs_15191, dz2080U_15180);
    bool x_20321 = cond_15189 && cond_t_res_15195;
    
    // stdlib/regex.fut:8:3-9:24
    
    int8_t literal_res_15196;
    int64_t literal_res_15197;
    
    if (x_20321) {
        // stdlib/regex.fut:8:61-73
        
        bool x_20386 = sle64((int64_t) 0, zl_lhs_15191);
        
        // stdlib/regex.fut:8:61-73
        
        bool bounds_check_20387 = cond_t_res_15195 && x_20386;
        
        // stdlib/regex.fut:8:61-73
        
        bool index_certs_20388;
        
        if (!bounds_check_20387) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_15191, "] out of bounds for array of shape [", (long long) dz2080U_15180, "].", "-> #0  stdlib/regex.fut:8:61-73\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:8:61-73
        
        int8_t escaped_arg0_20389 = ((int8_t *) pattern_mem_20795.mem)[zl_lhs_15191];
        
        // stdlib/regex.fut:6:25-100
        
        bool cond_20390 = escaped_arg0_20389 == (int8_t) 110;
        
        // stdlib/regex.fut:6:25-100
        
        int8_t escaped_res_20391;
        
        if (cond_20390) {
            escaped_res_20391 = (int8_t) 10;
        } else {
            // stdlib/regex.fut:6:50-100
            
            bool cond_20392 = escaped_arg0_20389 == (int8_t) 114;
            
            // stdlib/regex.fut:6:50-100
            
            int8_t escaped_res_f_res_20393;
            
            if (cond_20392) {
                escaped_res_f_res_20393 = (int8_t) 13;
            } else {
                // stdlib/regex.fut:6:75-100
                
                bool cond_20394 = escaped_arg0_20389 == (int8_t) 116;
                
                // stdlib/regex.fut:6:75-100
                
                int8_t escaped_res_f_res_f_res_20395;
                
                if (cond_20394) {
                    escaped_res_f_res_f_res_20395 = (int8_t) 9;
                } else {
                    escaped_res_f_res_f_res_20395 = escaped_arg0_20389;
                }
                escaped_res_f_res_20393 = escaped_res_f_res_f_res_20395;
            }
            escaped_res_20391 = escaped_res_f_res_20393;
        }
        // stdlib/regex.fut:8:75-77
        
        int64_t tmp_20396 = add64((int64_t) 2, p_15182);
        
        literal_res_15196 = escaped_res_20391;
        literal_res_15197 = tmp_20396;
    } else {
        literal_res_15196 = zeze_lhs_15187;
        literal_res_15197 = zl_lhs_15191;
    }
    prim_out_20998 = literal_res_15196;
    prim_out_20999 = literal_res_15197;
    *out_prim_out_21109 = prim_out_20998;
    *out_prim_out_21110 = prim_out_20999;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_mask_set_8212(struct futhark_context *ctx, struct memblock *mem_out_p_21111, struct memblock mask_mem_20795, int8_t c_15233)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    // stdlib/regex.fut:11:14-22
    
    int64_t u8_res_17772 = zext_i8_i64(c_15233);
    
    // stdlib/regex.fut:11:22-25
    
    int64_t word_15239 = sdiv64(u8_res_17772, (int64_t) 64);
    
    // stdlib/regex.fut:12:20-30
    
    bool x_15241 = sle64((int64_t) 0, word_15239);
    
    // stdlib/regex.fut:12:20-30
    
    bool y_15242 = slt64(word_15239, (int64_t) 4);
    
    // stdlib/regex.fut:12:20-30
    
    bool bounds_check_15243 = x_15241 && y_15242;
    
    // stdlib/regex.fut:12:20-30
    
    bool index_certs_15244;
    
    if (!bounds_check_15243) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) word_15239, "] out of bounds for array of shape [", (long long) (int64_t) 4, "].", "-> #0  stdlib/regex.fut:12:20-30\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:12:20-30
    
    int64_t zb_lhs_15245 = ((int64_t *) mask_mem_20795.mem)[word_15239];
    
    // stdlib/regex.fut:12:51-54
    
    int8_t u8_arg0_15250 = umod8(c_15233, (int8_t) 64);
    
    // stdlib/regex.fut:12:42-54
    
    int64_t u8_res_17896 = zext_i8_i64(u8_arg0_15250);
    
    // stdlib/regex.fut:12:39-55
    
    int64_t zb_rhs_15254 = shl64((int64_t) 1, u8_res_17896);
    
    // stdlib/regex.fut:12:31-55
    
    int64_t tmp_15256 = zb_lhs_15245 | zb_rhs_15254;
    
    // stdlib/regex.fut:12:7-56
    ((int64_t *) mask_mem_20795.mem)[word_15239] = tmp_15256;
    if (memblock_set(ctx, &mem_out_20998, &mask_mem_20795, "mask_mem_20795") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21111, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_match_branch_11323(struct futhark_context *ctx, int64_t *out_prim_out_21112, struct memblock z30U_mem_20795, struct memblock z31U_mem_20796, struct memblock z32U_mem_20797, struct memblock z33U_mem_20798, struct memblock text_mem_20799, int64_t dz2080U_15775, int64_t dz2081U_15776, int64_t dz2082U_15777, int64_t dz2083U_15778, int64_t dz2080U_15779, int64_t token_15784, int64_t last_15785, int64_t position_15787)
{
    (void) ctx;
    
    int err = 0;
    int64_t prim_out_20998;
    
    // stdlib/regex.fut:57:3-67:14
    
    bool cond_15788 = token_15784 == last_15785;
    
    // stdlib/regex.fut:57:3-67:14
    
    int64_t match_branch_res_15789;
    
    if (cond_15788) {
        match_branch_res_15789 = position_15787;
    } else {
        // stdlib/regex.fut:59:16-27
        
        bool x_15790 = sle64((int64_t) 0, token_15784);
        
        // stdlib/regex.fut:59:16-27
        
        bool y_15791 = slt64(token_15784, dz2081U_15776);
        
        // stdlib/regex.fut:59:16-27
        
        bool bounds_check_15792 = x_15790 && y_15791;
        
        // stdlib/regex.fut:59:16-27
        
        bool index_certs_15793;
        
        if (!bounds_check_15792) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) token_15784, "] out of bounds for array of shape [", (long long) dz2081U_15776, "].", "-> #0  stdlib/regex.fut:59:16-27\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:59:16-27
        
        int64_t kind_15794 = ((int64_t *) z31U_mem_20796.mem)[token_15784];
        
        // stdlib/regex.fut:60:19-58
        
        bool cond_15796 = kind_15794 == (int64_t) 0;
        
        // stdlib/regex.fut:60:40-44
        
        bool cond_f_res_15798 = kind_15794 == (int64_t) 3;
        bool x_17743 = !cond_15796;
        bool y_17744 = cond_f_res_15798 && x_17743;
        bool cond_15797 = cond_15796 || y_17744;
        bool cond_neg_17745 = !cond_15797;
        int64_t minimum_15799 = btoi_bool_i64(cond_neg_17745);
        
        // stdlib/regex.fut:61:17-99
        
        bool cond_15801 = kind_15794 == (int64_t) 1;
        bool x_17747 = !cond_15801;
        bool y_17748 = cond_f_res_15798 && x_17747;
        bool cond_15802 = cond_15801 || y_17748;
        
        // stdlib/regex.fut:61:17-99
        
        int64_t limit_15804;
        
        if (cond_15802) {
            // stdlib/regex.fut:61:65-67
            
            int64_t min_arg0_20386 = add64((int64_t) 1, position_15787);
            
            // stdlib/regex.fut:61:48-81
            
            int64_t min_res_20387 = smin64(dz2080U_15779, min_arg0_20386);
            
            limit_15804 = min_res_20387;
        } else {
            limit_15804 = dz2080U_15779;
        }
        // stdlib/regex.fut:62:15-97
        
        bool cond_15813 = slt64(position_15787, limit_15804);
        
        // stdlib/regex.fut:62:15-97
        
        bool loop_cond_15814;
        
        if (cond_15813) {
            // stdlib/regex.fut:62:79-88
            
            bool x_20388 = sle64((int64_t) 0, position_15787);
            
            // stdlib/regex.fut:62:79-88
            
            bool y_20389 = slt64(position_15787, dz2080U_15779);
            
            // stdlib/regex.fut:62:79-88
            
            bool bounds_check_20390 = x_20388 && y_20389;
            
            // stdlib/regex.fut:62:79-88
            
            bool index_certs_20391;
            
            if (!bounds_check_20390) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) position_15787, "] out of bounds for array of shape [", (long long) dz2080U_15779, "].", "-> #0  stdlib/regex.fut:62:79-88\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:62:79-88
            
            int8_t token_accepts_arg2_20392 = ((int8_t *) text_mem_20799.mem)[position_15787];
            
            // stdlib/regex.fut:55:26-34
            
            int64_t u8_res_20393 = zext_i8_i64(token_accepts_arg2_20392);
            
            // stdlib/regex.fut:55:34-37
            
            int64_t zgzg_lhs_20394 = sdiv64(u8_res_20393, (int64_t) 64);
            
            // stdlib/regex.fut:55:15-38
            
            int64_t zgzg_lhs_20395 = ((int64_t *) z30U_mem_20795.mem)[token_15784 * (int64_t) 4 + zgzg_lhs_20394];
            
            // stdlib/regex.fut:55:51-54
            
            int8_t u8_arg0_20396 = umod8(token_accepts_arg2_20392, (int8_t) 64);
            
            // stdlib/regex.fut:55:42-54
            
            int64_t u8_res_20397 = zext_i8_i64(u8_arg0_20396);
            
            // stdlib/regex.fut:55:39-55
            
            int64_t za_lhs_20398 = lshr64(zgzg_lhs_20395, u8_res_20397);
            
            // stdlib/regex.fut:55:56-58
            
            int64_t znze_lhs_20399 = (int64_t) 1 & za_lhs_20398;
            
            // stdlib/regex.fut:55:60-64
            
            bool token_accepts_res_20400 = znze_lhs_20399 == (int64_t) 0;
            
            // stdlib/regex.fut:55:60-64
            
            bool token_accepts_res_20401 = !token_accepts_res_20400;
            
            loop_cond_15814 = token_accepts_res_20401;
        } else {
            loop_cond_15814 = 0;
        }
        // stdlib/regex.fut:62:15-97
        
        bool end_15822;
        int64_t end_15823;
        bool loop_while_15824;
        int64_t end_15825;
        
        loop_while_15824 = loop_cond_15814;
        end_15825 = position_15787;
        while (loop_while_15824) {
            // stdlib/regex.fut:62:95-97
            
            int64_t loopres_15826 = add64((int64_t) 1, end_15825);
            
            // stdlib/regex.fut:62:41-88
            
            bool cond_15828 = slt64(loopres_15826, limit_15804);
            
            // stdlib/regex.fut:62:41-88
            
            bool loop_cond_15829;
            
            if (cond_15828) {
                // stdlib/regex.fut:62:79-88
                
                bool x_20402 = sle64((int64_t) 0, loopres_15826);
                
                // stdlib/regex.fut:62:79-88
                
                bool y_20403 = slt64(loopres_15826, dz2080U_15779);
                
                // stdlib/regex.fut:62:79-88
                
                bool bounds_check_20404 = x_20402 && y_20403;
                
                // stdlib/regex.fut:62:79-88
                
                bool index_certs_20405;
                
                if (!bounds_check_20404) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_15826, "] out of bounds for array of shape [", (long long) dz2080U_15779, "].", "-> #0  stdlib/regex.fut:62:79-88\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:62:79-88
                
                int8_t token_accepts_arg2_20406 = ((int8_t *) text_mem_20799.mem)[loopres_15826];
                
                // stdlib/regex.fut:55:26-34
                
                int64_t u8_res_20407 = zext_i8_i64(token_accepts_arg2_20406);
                
                // stdlib/regex.fut:55:34-37
                
                int64_t zgzg_lhs_20408 = sdiv64(u8_res_20407, (int64_t) 64);
                
                // stdlib/regex.fut:55:15-38
                
                int64_t zgzg_lhs_20409 = ((int64_t *) z30U_mem_20795.mem)[token_15784 * (int64_t) 4 + zgzg_lhs_20408];
                
                // stdlib/regex.fut:55:51-54
                
                int8_t u8_arg0_20410 = umod8(token_accepts_arg2_20406, (int8_t) 64);
                
                // stdlib/regex.fut:55:42-54
                
                int64_t u8_res_20411 = zext_i8_i64(u8_arg0_20410);
                
                // stdlib/regex.fut:55:39-55
                
                int64_t za_lhs_20412 = lshr64(zgzg_lhs_20409, u8_res_20411);
                
                // stdlib/regex.fut:55:56-58
                
                int64_t znze_lhs_20413 = (int64_t) 1 & za_lhs_20412;
                
                // stdlib/regex.fut:55:60-64
                
                bool token_accepts_res_20414 = znze_lhs_20413 == (int64_t) 0;
                
                // stdlib/regex.fut:55:60-64
                
                bool token_accepts_res_20415 = !token_accepts_res_20414;
                
                loop_cond_15829 = token_accepts_res_20415;
            } else {
                loop_cond_15829 = 0;
            }
            
            bool loop_while_tmp_20999 = loop_cond_15829;
            int64_t end_tmp_21000 = loopres_15826;
            
            loop_while_15824 = loop_while_tmp_20999;
            end_15825 = end_tmp_21000;
        }
        end_15822 = loop_while_15824;
        end_15823 = end_15825;
        // stdlib/regex.fut:63:27-66:28
        
        int64_t zgze_rhs_15841 = add64(position_15787, minimum_15799);
        
        // stdlib/regex.fut:63:27-66:28
        
        bool cond_15843 = sle64(zgze_rhs_15841, end_15823);
        
        // stdlib/regex.fut:65:44-46
        
        int64_t match_branch_arg1_15852 = add64((int64_t) 1, token_15784);
        
        // stdlib/regex.fut:63:27-66:28
        
        bool match_branch_res_f_res_15846;
        int64_t match_branch_res_f_res_15847;
        int64_t match_branch_res_f_res_15848;
        bool loop_while_15849;
        int64_t trial_15850;
        int64_t result_15851;
        
        loop_while_15849 = cond_15843;
        trial_15850 = end_15823;
        result_15851 = (int64_t) -1;
        while (loop_while_15849) {
            // stdlib/regex.fut:56:19-65:63
            
            int64_t result_15854;
            
            if (futrts_match_branch_11323(ctx, &result_15854, z30U_mem_20795, z31U_mem_20796, z32U_mem_20797, z33U_mem_20798, text_mem_20799, dz2080U_15775, dz2081U_15776, dz2082U_15777, dz2083U_15778, dz2080U_15779, match_branch_arg1_15852, last_15785, trial_15850) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/regex.fut:66:18-20
            
            int64_t tmp_15856 = sub64(trial_15850, (int64_t) 1);
            
            // stdlib/regex.fut:64:13-52
            
            bool cond_15862 = sle64(zgze_rhs_15841, tmp_15856);
            
            // stdlib/regex.fut:64:49-52
            
            bool loop_cond_t_res_15864 = slt64(result_15854, (int64_t) 0);
            bool x_17752 = cond_15862 && loop_cond_t_res_15864;
            bool loop_while_tmp_21001 = x_17752;
            int64_t trial_tmp_21002 = tmp_15856;
            int64_t result_tmp_21003 = result_15854;
            
            loop_while_15849 = loop_while_tmp_21001;
            trial_15850 = trial_tmp_21002;
            result_15851 = result_tmp_21003;
        }
        match_branch_res_f_res_15846 = loop_while_15849;
        match_branch_res_f_res_15847 = trial_15850;
        match_branch_res_f_res_15848 = result_15851;
        match_branch_res_15789 = match_branch_res_f_res_15848;
    }
    prim_out_20998 = match_branch_res_15789;
    *out_prim_out_21112 = prim_out_20998;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_mul_abs_11275(struct futhark_context *ctx, struct memblock *mem_out_p_21113, int64_t *out_prim_out_21114, struct memblock a_mem_20795, int64_t dz2080U_14603, int32_t b_14605)
{
    (void) ctx;
    
    int err = 0;
    struct memblock ext_mem_20806;
    
    ext_mem_20806.references = NULL;
    
    struct memblock mem_20797;
    
    mem_20797.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t prim_out_20999;
    
    // stdlib/bigint.fut:44:52-62
    
    int64_t dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_14607 = add64((int64_t) 1, dz2080U_14603);
    int64_t bytes_20796 = (int64_t) 4 * dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_14607;
    
    // stdlib/bigint.fut:48:7-36
    
    bool y_14655 = slt64(dz2080U_14603, dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_14607);
    
    // stdlib/bigint.fut:48:7-36
    
    bool index_certs_14657;
    
    if (!y_14655) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) dz2080U_14603, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_14607, "].", "-> #0  stdlib/bigint.fut:48:7-36\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    if (memblock_alloc(ctx, &mem_20797, bytes_20796, "mem_20797")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:44:21-47:27
    for (int64_t nest_i_21000 = 0; nest_i_21000 < dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_14607; nest_i_21000++) {
        ((int32_t *) mem_20797.mem)[nest_i_21000] = 0;
    }
    
    int64_t u32_res_17896 = zext_i32_i64(b_14605);
    
    // stdlib/bigint.fut:44:21-47:27
    
    int64_t mul_abs_res_14620;
    int64_t carry_14623 = (int64_t) 0;
    
    for (int64_t i_14621 = 0; i_14621 < dz2080U_14603; i_14621++) {
        // stdlib/bigint.fut:46:9-33
        
        bool y_14641 = slt64(i_14621, dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_14607);
        
        // stdlib/bigint.fut:46:9-33
        
        bool index_certs_14643;
        
        if (!y_14641) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_14621, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_14607, "].", "-> #0  stdlib/bigint.fut:46:9-33\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint.fut:45:27-31
        
        int32_t u32_arg0_14628 = ((int32_t *) a_mem_20795.mem)[i_14621];
        
        // stdlib/bigint.fut:45:19-31
        
        int64_t u32_res_17898 = zext_i32_i64(u32_arg0_14628);
        
        // stdlib/bigint.fut:45:31-41
        
        int64_t zp_lhs_14634 = mul64(u32_res_17896, u32_res_17898);
        
        // stdlib/bigint.fut:45:41-47
        
        int64_t product_14636 = add64(carry_14623, zp_lhs_14634);
        
        // stdlib/bigint.fut:46:18-33
        
        int32_t u64_res_17900 = zext_i64_i32(product_14636);
        
        // stdlib/bigint.fut:46:9-33
        ((int32_t *) mem_20797.mem)[i_14621] = u64_res_17900;
        // stdlib/bigint.fut:47:21-26
        
        int64_t tmp_14646 = lshr64(product_14636, (int64_t) 32);
        int64_t carry_tmp_21002 = tmp_14646;
        
        carry_14623 = carry_tmp_21002;
    }
    mul_abs_res_14620 = carry_14623;
    // stdlib/bigint.fut:48:23-36
    
    int32_t u64_res_17902 = zext_i64_i32(mul_abs_res_14620);
    
    // stdlib/bigint.fut:48:7-36
    ((int32_t *) mem_20797.mem)[dz2080U_14603] = u64_res_17902;
    // stdlib/bigint.fut:49:7-26
    
    int64_t mul_abs_res_14660;
    bool mul_abs_res_14661;
    
    if (futrts_normalise_11258(ctx, &ext_mem_20806, &mul_abs_res_14660, &mul_abs_res_14661, mem_20797, dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_14607, 0) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &mem_20797, "mem_20797") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20998, &ext_mem_20806, "ext_mem_20806") != 0)
        return 1;
    prim_out_20999 = mul_abs_res_14660;
    if (memblock_set(ctx, &*mem_out_p_21113, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    *out_prim_out_21114 = prim_out_20999;
    
  cleanup:
    {
        if (memblock_unref(ctx, &ext_mem_20806, "ext_mem_20806") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_20797, "mem_20797") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_normalise_11258(struct futhark_context *ctx, struct memblock *mem_out_p_21115, int64_t *out_prim_out_21116, bool *out_prim_out_21117, struct memblock a_mem_20795, int64_t dz2080U_14197, bool negative_14198)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t prim_out_20999;
    bool prim_out_21000;
    
    // stdlib/bigint.fut:5:14-65
    
    bool cond_14203 = slt64((int64_t) 1, dz2080U_14197);
    
    // stdlib/bigint.fut:5:50-52
    
    int64_t zeze_lhs_14205 = sub64(dz2080U_14197, (int64_t) 1);
    
    // stdlib/bigint.fut:5:47-53
    
    bool x_14207 = sle64((int64_t) 0, zeze_lhs_14205);
    
    // stdlib/bigint.fut:5:47-53
    
    bool y_14208 = slt64(zeze_lhs_14205, dz2080U_14197);
    
    // stdlib/bigint.fut:5:47-53
    
    bool bounds_check_14209 = x_14207 && y_14208;
    bool loop_not_taken_17744 = !cond_14203;
    bool protect_assert_disj_17745 = bounds_check_14209 || loop_not_taken_17744;
    
    // stdlib/bigint.fut:5:47-53
    
    bool index_certs_14210;
    
    if (!protect_assert_disj_17745) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zeze_lhs_14205, "] out of bounds for array of shape [", (long long) dz2080U_14197, "].", "-> #0  stdlib/bigint.fut:5:47-53\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:5:47-53
    
    int32_t zeze_lhs_14211;
    
    if (cond_14203) {
        int32_t x_20386 = ((int32_t *) a_mem_20795.mem)[zeze_lhs_14205];
        
        zeze_lhs_14211 = x_20386;
    } else {
        zeze_lhs_14211 = 0;
    }
    // stdlib/bigint.fut:5:54-58
    
    bool loop_cond_t_res_14213 = zeze_lhs_14211 == 0;
    bool x_17748 = cond_14203 && loop_cond_t_res_14213;
    
    // stdlib/bigint.fut:5:14-65
    
    bool used_14214;
    int64_t used_14215;
    bool loop_while_14216;
    int64_t k_14217;
    
    loop_while_14216 = x_17748;
    k_14217 = dz2080U_14197;
    while (loop_while_14216) {
        // stdlib/bigint.fut:5:63-65
        
        int64_t loopres_14218 = sub64(k_14217, (int64_t) 1);
        
        // stdlib/bigint.fut:5:38-58
        
        bool cond_14220 = slt64((int64_t) 1, loopres_14218);
        
        // stdlib/bigint.fut:5:38-58
        
        bool loop_cond_14221;
        
        if (cond_14220) {
            // stdlib/bigint.fut:5:50-52
            
            int64_t zeze_lhs_20387 = sub64(loopres_14218, (int64_t) 1);
            
            // stdlib/bigint.fut:5:47-53
            
            bool x_20388 = sle64((int64_t) 0, zeze_lhs_20387);
            
            // stdlib/bigint.fut:5:47-53
            
            bool y_20389 = slt64(zeze_lhs_20387, dz2080U_14197);
            
            // stdlib/bigint.fut:5:47-53
            
            bool bounds_check_20390 = x_20388 && y_20389;
            
            // stdlib/bigint.fut:5:47-53
            
            bool index_certs_20391;
            
            if (!bounds_check_20390) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zeze_lhs_20387, "] out of bounds for array of shape [", (long long) dz2080U_14197, "].", "-> #0  stdlib/bigint.fut:5:47-53\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/bigint.fut:5:47-53
            
            int32_t zeze_lhs_20392 = ((int32_t *) a_mem_20795.mem)[zeze_lhs_20387];
            
            // stdlib/bigint.fut:5:54-58
            
            bool loop_cond_t_res_20393 = zeze_lhs_20392 == 0;
            
            loop_cond_14221 = loop_cond_t_res_20393;
        } else {
            loop_cond_14221 = 0;
        }
        
        bool loop_while_tmp_21001 = loop_cond_14221;
        int64_t k_tmp_21002 = loopres_14218;
        
        loop_while_14216 = loop_while_tmp_21001;
        k_14217 = k_tmp_21002;
    }
    used_14214 = loop_while_14216;
    used_14215 = k_14217;
    // stdlib/bigint.fut:6:15-26
    
    bool empty_slice_17776 = used_14215 == (int64_t) 0;
    
    // stdlib/bigint.fut:6:15-26
    
    int64_t m_17777 = sub64(used_14215, (int64_t) 1);
    
    // stdlib/bigint.fut:6:15-26
    
    bool zzero_leq_i_p_m_t_s_17778 = sle64((int64_t) 0, m_17777);
    
    // stdlib/bigint.fut:6:15-26
    
    bool i_p_m_t_s_leq_w_17779 = slt64(m_17777, dz2080U_14197);
    
    // stdlib/bigint.fut:6:15-26
    
    bool i_lte_j_17780 = sle64((int64_t) 0, used_14215);
    
    // stdlib/bigint.fut:6:15-26
    
    bool y_17781 = zzero_leq_i_p_m_t_s_17778 && i_p_m_t_s_leq_w_17779;
    
    // stdlib/bigint.fut:6:15-26
    
    bool forwards_ok_17782 = i_lte_j_17780 && y_17781;
    
    // stdlib/bigint.fut:6:15-26
    
    bool ok_or_empty_17783 = empty_slice_17776 || forwards_ok_17782;
    
    // stdlib/bigint.fut:6:15-26
    
    bool index_certs_17784;
    
    if (!ok_or_empty_17783) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) used_14215, "] out of bounds for array of shape [", (long long) dz2080U_14197, "].", "-> #0  stdlib/bigint.fut:6:15-26\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:7:7-48
    
    bool tmp_14234;
    
    if (negative_14198) {
        // stdlib/bigint.fut:7:21-47
        
        bool cond_20398 = used_14215 == (int64_t) 1;
        
        // stdlib/bigint.fut:7:21-47
        
        bool tmp_20399;
        
        if (cond_20398) {
            // stdlib/bigint.fut:7:34-42
            
            bool y_20400 = slt64((int64_t) 0, used_14215);
            
            // stdlib/bigint.fut:7:34-42
            
            bool index_certs_20401;
            
            if (!y_20400) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) used_14215, "].", "-> #0  stdlib/bigint.fut:7:34-42\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            
            int32_t zeze_lhs_20402 = ((int32_t *) a_mem_20795.mem)[(int64_t) 0];
            
            // stdlib/bigint.fut:7:43-47
            
            bool tmp_t_res_20403 = zeze_lhs_20402 == 0;
            
            tmp_20399 = tmp_t_res_20403;
        } else {
            tmp_20399 = 0;
        }
        // stdlib/bigint.fut:7:19-48
        
        bool tmp_t_res_20404 = !tmp_20399;
        
        tmp_14234 = tmp_t_res_20404;
    } else {
        tmp_14234 = 0;
    }
    if (memblock_set(ctx, &mem_out_20998, &a_mem_20795, "a_mem_20795") != 0)
        return 1;
    prim_out_20999 = used_14215;
    prim_out_21000 = tmp_14234;
    if (memblock_set(ctx, &*mem_out_p_21115, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    *out_prim_out_21116 = prim_out_20999;
    *out_prim_out_21117 = prim_out_21000;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_put_raw_11346(struct futhark_context *ctx, struct memblock *mem_out_p_21118, struct memblock *mem_out_p_21119, struct memblock *mem_out_p_21120, int64_t *out_prim_out_21121, struct memblock z30U_mem_20795, struct memblock z30U_mem_20796, struct memblock z30U_mem_20797, int64_t dz2080U_16917, int64_t z31U_16921, int64_t key_16922, int64_t value_16923)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_21000;
    
    mem_out_21000.references = NULL;
    
    struct memblock mem_out_20999;
    
    mem_out_20999.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    int64_t prim_out_21001;
    
    // stdlib/hashmap.fut:11:26-28
    
    int64_t mask_20329 = sub64(dz2080U_16917, (int64_t) 1);
    
    // stdlib/hashmap.fut:7:19-24
    
    int64_t zc_rhs_20330 = lshr64(key_16922, (int64_t) 30);
    
    // stdlib/hashmap.fut:7:14-24
    
    int64_t zt_lhs_20331 = key_16922 ^ zc_rhs_20330;
    
    // stdlib/hashmap.fut:7:26-48
    
    int64_t x_20332 = mul64((int64_t) -4658895280553007687, zt_lhs_20331);
    
    // stdlib/hashmap.fut:8:19-24
    
    int64_t zc_rhs_20333 = lshr64(x_20332, (int64_t) 27);
    
    // stdlib/hashmap.fut:8:14-24
    
    int64_t zt_lhs_20334 = x_20332 ^ zc_rhs_20333;
    
    // stdlib/hashmap.fut:8:26-48
    
    int64_t x_20335 = mul64((int64_t) -7723592293110705685, zt_lhs_20334);
    
    // stdlib/hashmap.fut:9:13-18
    
    int64_t zc_rhs_20336 = lshr64(x_20335, (int64_t) 31);
    
    // stdlib/hashmap.fut:9:8-18
    
    int64_t hash_res_20337 = x_20335 ^ zc_rhs_20336;
    
    // stdlib/hashmap.fut:12:43-57
    
    int64_t u64_arg0_20338 = mask_20329 & hash_res_20337;
    
    // stdlib/hashmap.fut:12:16-13:57
    
    bool loop_cond_20339 = ((bool *) z30U_mem_20797.mem)[u64_arg0_20338];
    
    // stdlib/hashmap.fut:12:16-13:57
    
    bool loop_cond_20340;
    
    if (loop_cond_20339) {
        // stdlib/hashmap.fut:13:26-34
        
        int64_t loop_cond_t_res_20386 = ((int64_t *) z30U_mem_20795.mem)[u64_arg0_20338];
        
        // stdlib/hashmap.fut:13:37-43
        
        bool loop_cond_t_res_20387 = loop_cond_t_res_20386 == key_16922;
        
        // stdlib/hashmap.fut:13:37-43
        
        bool loop_cond_t_res_20388 = !loop_cond_t_res_20387;
        
        loop_cond_20340 = loop_cond_t_res_20388;
    } else {
        loop_cond_20340 = 0;
    }
    // stdlib/hashmap.fut:12:16-13:57
    
    bool slot_res_20344;
    int64_t slot_res_20345;
    bool loop_while_20346;
    int64_t i_20347;
    
    loop_while_20346 = loop_cond_20340;
    i_20347 = u64_arg0_20338;
    while (loop_while_20346) {
        // stdlib/hashmap.fut:13:49-51
        
        int64_t za_lhs_20348 = add64((int64_t) 1, i_20347);
        
        // stdlib/hashmap.fut:13:52-57
        
        int64_t loopres_20349 = mask_20329 & za_lhs_20348;
        
        // stdlib/hashmap.fut:13:12-20
        
        bool loop_cond_20350 = ((bool *) z30U_mem_20797.mem)[loopres_20349];
        
        // stdlib/hashmap.fut:13:12-43
        
        bool loop_cond_20351;
        
        if (loop_cond_20350) {
            // stdlib/hashmap.fut:13:26-34
            
            int64_t loop_cond_t_res_20389 = ((int64_t *) z30U_mem_20795.mem)[loopres_20349];
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20390 = loop_cond_t_res_20389 == key_16922;
            
            // stdlib/hashmap.fut:13:37-43
            
            bool loop_cond_t_res_20391 = !loop_cond_t_res_20390;
            
            loop_cond_20351 = loop_cond_t_res_20391;
        } else {
            loop_cond_20351 = 0;
        }
        
        bool loop_while_tmp_21002 = loop_cond_20351;
        int64_t i_tmp_21003 = loopres_20349;
        
        loop_while_20346 = loop_while_tmp_21002;
        i_20347 = i_tmp_21003;
    }
    slot_res_20344 = loop_while_20346;
    slot_res_20345 = i_20347;
    // stdlib/hashmap.fut:21:27-35
    
    bool x_16930 = sle64((int64_t) 0, slot_res_20345);
    
    // stdlib/hashmap.fut:21:27-35
    
    bool y_16931 = slt64(slot_res_20345, dz2080U_16917);
    
    // stdlib/hashmap.fut:21:27-35
    
    bool bounds_check_16932 = x_16930 && y_16931;
    
    // stdlib/hashmap.fut:21:27-35
    
    bool index_certs_16933;
    
    if (!bounds_check_16932) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_20345, "] out of bounds for array of shape [", (long long) dz2080U_16917, "].", "-> #0  stdlib/hashmap.fut:21:27-35\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/hashmap.fut:21:27-35
    
    bool put_raw_res_16936 = ((bool *) z30U_mem_20797.mem)[slot_res_20345];
    bool cond_neg_17742 = !put_raw_res_16936;
    int64_t zp_rhs_16940 = btoi_bool_i64(cond_neg_17742);
    
    // stdlib/hashmap.fut:21:21-51
    
    int64_t count_16942 = add64(z31U_16921, zp_rhs_16940);
    
    // stdlib/hashmap.fut:22:7-34
    ((int64_t *) z30U_mem_20795.mem)[slot_res_20345] = key_16922;
    // stdlib/hashmap.fut:22:7-34
    ((int64_t *) z30U_mem_20796.mem)[slot_res_20345] = value_16923;
    // stdlib/hashmap.fut:22:7-34
    ((bool *) z30U_mem_20797.mem)[slot_res_20345] = 1;
    if (memblock_set(ctx, &mem_out_20998, &z30U_mem_20795, "z30U_mem_20795") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_20999, &z30U_mem_20796, "z30U_mem_20796") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_21000, &z30U_mem_20797, "z30U_mem_20797") != 0)
        return 1;
    prim_out_21001 = count_16942;
    if (memblock_set(ctx, &*mem_out_p_21118, &mem_out_20998, "mem_out_20998") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21119, &mem_out_20999, "mem_out_20999") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_21120, &mem_out_21000, "mem_out_21000") != 0)
        return 1;
    *out_prim_out_21121 = prim_out_21001;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_21000, "mem_out_21000") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20999, "mem_out_20999") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_20998, "mem_out_20998") != 0)
            return 1;
    }
    return err;
}

int futhark_entry_arithmetic(struct futhark_context *ctx, struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 **out, const bool in0, const struct futhark_u32_1d *in1, const bool in2, const struct futhark_u32_1d *in3, const uint32_t in4)
{
    int64_t dz2080U_15061 = (int64_t) 0;
    int64_t dz2081U_15062 = (int64_t) 0;
    bool negative_a_15063 = 0;
    bool negative_b_15065 = 0;
    int32_t factor_15067 = 0;
    int64_t prim_out_21002 = (int64_t) 0;
    int64_t prim_out_21003 = (int64_t) 0;
    int64_t prim_out_21004 = (int64_t) 0;
    int64_t prim_out_21005 = (int64_t) 0;
    bool prim_out_21006 = 0;
    bool prim_out_21007 = 0;
    bool prim_out_21008 = 0;
    bool prim_out_21009 = 0;
    int64_t prim_out_21010 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_21001;
    
    mem_out_21001.references = NULL;
    
    struct memblock mem_out_21000;
    
    mem_out_21000.references = NULL;
    
    struct memblock mem_out_20999;
    
    mem_out_20999.references = NULL;
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    struct memblock b_mem_20796;
    
    b_mem_20796.references = NULL;
    
    struct memblock a_mem_20795;
    
    a_mem_20795.references = NULL;
    negative_a_15063 = in0;
    a_mem_20795 = in1->mem;
    dz2080U_15061 = in1->shape[0];
    negative_b_15065 = in2;
    b_mem_20796 = in3->mem;
    dz2081U_15062 = in3->shape[0];
    factor_15067 = in4;
    if (!(dz2080U_15061 == in1->shape[0] && dz2081U_15062 == in3->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_arithmetic(ctx, &mem_out_20998, &mem_out_20999, &mem_out_21000, &mem_out_21001, &prim_out_21002, &prim_out_21003, &prim_out_21004, &prim_out_21005, &prim_out_21006, &prim_out_21007, &prim_out_21008, &prim_out_21009, &prim_out_21010, a_mem_20795, b_mem_20796, dz2080U_15061, dz2081U_15062, negative_a_15063, negative_b_15065, factor_15067);
        if (ret == 0) {
            assert((*out = (struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *) malloc(sizeof(struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64))) != NULL);
            (*out)->v0 = prim_out_21006;
            assert(((*out)->v1 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v1->mem = mem_out_20998;
            (*out)->v1->shape[0] = prim_out_21002;
            (*out)->v2 = prim_out_21007;
            assert(((*out)->v3 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v3->mem = mem_out_20999;
            (*out)->v3->shape[0] = prim_out_21003;
            (*out)->v4 = prim_out_21008;
            assert(((*out)->v5 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v5->mem = mem_out_21000;
            (*out)->v5->shape[0] = prim_out_21004;
            (*out)->v6 = prim_out_21009;
            assert(((*out)->v7 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v7->mem = mem_out_21001;
            (*out)->v7->shape[0] = prim_out_21005;
            (*out)->v8 = prim_out_21010;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_hashmap_increment(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0, const struct futhark_i64_1d *in1)
{
    int64_t dz2080U_17344 = (int64_t) 0;
    int64_t dz2081U_17345 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    struct memblock deltas_mem_20796;
    
    deltas_mem_20796.references = NULL;
    
    struct memblock keys_mem_20795;
    
    keys_mem_20795.references = NULL;
    keys_mem_20795 = in0->mem;
    dz2080U_17344 = in0->shape[0];
    deltas_mem_20796 = in1->mem;
    dz2081U_17345 = in1->shape[0];
    if (!(dz2080U_17344 == in0->shape[0] && dz2081U_17345 == in1->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_hashmap_increment(ctx, &mem_out_20998, keys_mem_20795, deltas_mem_20796, dz2080U_17344, dz2081U_17345);
        if (ret == 0) {
            assert((*out = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d))) != NULL);
            (*out)->mem = mem_out_20998;
            (*out)->shape[0] = dz2080U_17344;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_hashmap_reserved(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0, const struct futhark_i64_1d *in1, const int64_t in2)
{
    int64_t dz2080U_17606 = (int64_t) 0;
    int64_t dz2081U_17607 = (int64_t) 0;
    int64_t batch_17610 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    struct memblock deltas_mem_20796;
    
    deltas_mem_20796.references = NULL;
    
    struct memblock keys_mem_20795;
    
    keys_mem_20795.references = NULL;
    keys_mem_20795 = in0->mem;
    dz2080U_17606 = in0->shape[0];
    deltas_mem_20796 = in1->mem;
    dz2081U_17607 = in1->shape[0];
    batch_17610 = in2;
    if (!(dz2080U_17606 == in0->shape[0] && dz2081U_17607 == in1->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_hashmap_reserved(ctx, &mem_out_20998, keys_mem_20795, deltas_mem_20796, dz2080U_17606, dz2081U_17607, batch_17610);
        if (ret == 0) {
            assert((*out = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d))) != NULL);
            (*out)->mem = mem_out_20998;
            (*out)->shape[0] = dz2080U_17606;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_hashmap_test(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0)
{
    int64_t dz2080U_17166 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    struct memblock keys_mem_20795;
    
    keys_mem_20795.references = NULL;
    keys_mem_20795 = in0->mem;
    dz2080U_17166 = in0->shape[0];
    if (!(dz2080U_17166 == in0->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_hashmap_test(ctx, &mem_out_20998, keys_mem_20795, dz2080U_17166);
        if (ret == 0) {
            assert((*out = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d))) != NULL);
            (*out)->mem = mem_out_20998;
            (*out)->shape[0] = dz2080U_17166;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_regex_test(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 **out, const struct futhark_u8_1d *in0, const struct futhark_u8_1d *in1, const struct futhark_u8_1d *in2)
{
    int64_t dz2080U_16711 = (int64_t) 0;
    int64_t dz2081U_16712 = (int64_t) 0;
    int64_t dz2082U_16713 = (int64_t) 0;
    int64_t prim_out_20999 = (int64_t) 0;
    int64_t prim_out_21000 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    struct memblock replacement_mem_20797;
    
    replacement_mem_20797.references = NULL;
    
    struct memblock text_mem_20796;
    
    text_mem_20796.references = NULL;
    
    struct memblock pattern_mem_20795;
    
    pattern_mem_20795.references = NULL;
    pattern_mem_20795 = in0->mem;
    dz2080U_16711 = in0->shape[0];
    text_mem_20796 = in1->mem;
    dz2081U_16712 = in1->shape[0];
    replacement_mem_20797 = in2->mem;
    dz2082U_16713 = in2->shape[0];
    if (!(dz2080U_16711 == in0->shape[0] && (dz2081U_16712 == in1->shape[0] && dz2082U_16713 == in2->shape[0]))) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_regex_test(ctx, &mem_out_20998, &prim_out_20999, &prim_out_21000, pattern_mem_20795, text_mem_20796, replacement_mem_20797, dz2080U_16711, dz2081U_16712, dz2082U_16713);
        if (ret == 0) {
            assert((*out = (struct futhark_opaque_tup2_i64_arr1d_u8 *) malloc(sizeof(struct futhark_opaque_tup2_i64_arr1d_u8))) != NULL);
            (*out)->v0 = prim_out_21000;
            assert(((*out)->v1 = (struct futhark_u8_1d *) malloc(sizeof(struct futhark_u8_1d))) != NULL);
            (*out)->v1->mem = mem_out_20998;
            (*out)->v1->shape[0] = prim_out_20999;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_wrap_test(struct futhark_context *ctx, struct futhark_u8_1d **out, const int64_t in0, const struct futhark_u8_1d *in1)
{
    int64_t dz2080U_17737 = (int64_t) 0;
    int64_t width_17738 = (int64_t) 0;
    int64_t prim_out_20999 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_20998;
    
    mem_out_20998.references = NULL;
    
    struct memblock text_mem_20795;
    
    text_mem_20795.references = NULL;
    width_17738 = in0;
    text_mem_20795 = in1->mem;
    dz2080U_17737 = in1->shape[0];
    if (!(dz2080U_17737 == in1->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_wrap_test(ctx, &mem_out_20998, &prim_out_20999, text_mem_20795, dz2080U_17737, width_17738);
        if (ret == 0) {
            assert((*out = (struct futhark_u8_1d *) malloc(sizeof(struct futhark_u8_1d))) != NULL);
            (*out)->mem = mem_out_20998;
            (*out)->shape[0] = prim_out_20999;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
  
