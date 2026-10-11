
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
int futhark_entry_hashmap_test(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0);
int futhark_entry_regex_test(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 **out, const struct futhark_u8_1d *in0, const struct futhark_u8_1d *in1, const struct futhark_u8_1d *in2);

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
    
    struct entry_point_entry entry_points[] = {{.name ="arithmetic", .fun =futrts_cli_entry_arithmetic}, {.name ="hashmap_test", .fun =futrts_cli_entry_hashmap_test}, {.name ="regex_test", .fun =futrts_cli_entry_regex_test}};
    
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
static int32_t arithmeticzistatic_array_realtype_19379[2] = { 1,0};
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

FUTHARK_FUN_ATTR int futrts_entry_arithmetic(struct futhark_context *ctx, struct memblock *mem_out_p_19366, struct memblock *mem_out_p_19367, struct memblock *mem_out_p_19368, struct memblock *mem_out_p_19369, int64_t *out_prim_out_19370, int64_t *out_prim_out_19371, int64_t *out_prim_out_19372, int64_t *out_prim_out_19373, bool *out_prim_out_19374, bool *out_prim_out_19375, bool *out_prim_out_19376, bool *out_prim_out_19377, int64_t *out_prim_out_19378, struct memblock a_mem_19086, struct memblock b_mem_19087, int64_t dz2080U_13919, int64_t dz2081U_13920, bool negative_a_13921, bool negative_b_13923, int32_t factor_13925);
FUTHARK_FUN_ATTR int futrts_entry_hashmap_test(struct futhark_context *ctx, struct memblock *mem_out_p_19380, struct memblock keys_mem_19086, int64_t dz2080U_16031);
FUTHARK_FUN_ATTR int futrts_entry_regex_test(struct futhark_context *ctx, struct memblock *mem_out_p_19381, int64_t *out_prim_out_19382, int64_t *out_prim_out_19383, struct memblock pattern_mem_19086, struct memblock text_mem_19087, struct memblock replacement_mem_19088, int64_t dz2080U_15585, int64_t dz2081U_15586, int64_t dz2082U_15587);
FUTHARK_FUN_ATTR int futrts_find_10646(struct futhark_context *ctx, int64_t *out_prim_out_19385, int64_t *out_prim_out_19386, struct memblock z30U_mem_19086, struct memblock z31U_mem_19087, struct memblock z32U_mem_19088, struct memblock z33U_mem_19089, struct memblock text_mem_19090, int64_t dz2080U_14728, int64_t dz2081U_14729, int64_t dz2082U_14730, int64_t dz2083U_14731, int64_t dz2080U_14732, int64_t start_14738);
FUTHARK_FUN_ATTR int futrts_grow_10668(struct futhark_context *ctx, struct memblock *mem_out_p_19387, struct memblock *mem_out_p_19388, struct memblock *mem_out_p_19389, int64_t *out_prim_out_19390, int64_t *out_prim_out_19391, struct memblock z30U_mem_19086, struct memblock z30U_mem_19087, struct memblock z30U_mem_19088, int64_t dz2080U_15819, int64_t z31U_15823);
FUTHARK_FUN_ATTR int futrts_literal_10630(struct futhark_context *ctx, int8_t *out_prim_out_19392, int64_t *out_prim_out_19393, struct memblock pattern_mem_19086, int64_t dz2080U_14038, int64_t p_14040);
FUTHARK_FUN_ATTR int futrts_mask_set_8060(struct futhark_context *ctx, struct memblock *mem_out_p_19394, struct memblock mask_mem_19086, int8_t c_14091);
FUTHARK_FUN_ATTR int futrts_match_branch_10647(struct futhark_context *ctx, int64_t *out_prim_out_19395, struct memblock z30U_mem_19086, struct memblock z31U_mem_19087, struct memblock z32U_mem_19088, struct memblock z33U_mem_19089, struct memblock text_mem_19090, int64_t dz2080U_14624, int64_t dz2081U_14625, int64_t dz2082U_14626, int64_t dz2083U_14627, int64_t dz2080U_14628, int64_t token_14633, int64_t last_14634, int64_t position_14636);
FUTHARK_FUN_ATTR int futrts_mul_abs_10600(struct futhark_context *ctx, struct memblock *mem_out_p_19396, int64_t *out_prim_out_19397, struct memblock a_mem_19086, int64_t dz2080U_13461, int32_t b_13463);
FUTHARK_FUN_ATTR int futrts_normalise_10583(struct futhark_context *ctx, struct memblock *mem_out_p_19398, int64_t *out_prim_out_19399, bool *out_prim_out_19400, struct memblock a_mem_19086, int64_t dz2080U_13055, bool negative_13056);
FUTHARK_FUN_ATTR int futrts_put_raw_10670(struct futhark_context *ctx, struct memblock *mem_out_p_19401, struct memblock *mem_out_p_19402, struct memblock *mem_out_p_19403, int64_t *out_prim_out_19404, struct memblock z30U_mem_19086, struct memblock z30U_mem_19087, struct memblock z30U_mem_19088, int64_t dz2080U_15782, int64_t z31U_15786, int64_t key_15787, int64_t value_15788);

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

FUTHARK_FUN_ATTR int futrts_entry_arithmetic(struct futhark_context *ctx, struct memblock *mem_out_p_19366, struct memblock *mem_out_p_19367, struct memblock *mem_out_p_19368, struct memblock *mem_out_p_19369, int64_t *out_prim_out_19370, int64_t *out_prim_out_19371, int64_t *out_prim_out_19372, int64_t *out_prim_out_19373, bool *out_prim_out_19374, bool *out_prim_out_19375, bool *out_prim_out_19376, bool *out_prim_out_19377, int64_t *out_prim_out_19378, struct memblock a_mem_19086, struct memblock b_mem_19087, int64_t dz2080U_13919, int64_t dz2081U_13920, bool negative_a_13921, bool negative_b_13923, int32_t factor_13925)
{
    (void) ctx;
    
    int err = 0;
    struct memblock ext_mem_19262;
    
    ext_mem_19262.references = NULL;
    
    struct memblock ext_mem_19235;
    
    ext_mem_19235.references = NULL;
    
    struct memblock ext_mem_19234;
    
    ext_mem_19234.references = NULL;
    
    struct memblock mem_19225;
    
    mem_19225.references = NULL;
    
    struct memblock ext_mem_19247;
    
    ext_mem_19247.references = NULL;
    
    struct memblock ext_mem_19246;
    
    ext_mem_19246.references = NULL;
    
    struct memblock mem_19237;
    
    mem_19237.references = NULL;
    
    struct memblock ext_mem_19248;
    
    ext_mem_19248.references = NULL;
    
    struct memblock ext_mem_19260;
    
    ext_mem_19260.references = NULL;
    
    struct memblock ext_mem_19259;
    
    ext_mem_19259.references = NULL;
    
    struct memblock mem_19250;
    
    mem_19250.references = NULL;
    
    struct memblock ext_mem_19261;
    
    ext_mem_19261.references = NULL;
    
    struct memblock ext_mem_19223;
    
    ext_mem_19223.references = NULL;
    
    struct memblock ext_mem_19263;
    
    ext_mem_19263.references = NULL;
    
    struct memblock ext_mem_19264;
    
    ext_mem_19264.references = NULL;
    
    struct memblock ext_mem_19265;
    
    ext_mem_19265.references = NULL;
    
    struct memblock mem_19222;
    
    mem_19222.references = NULL;
    
    struct memblock ext_mem_19221;
    
    ext_mem_19221.references = NULL;
    
    struct memblock ext_mem_19168;
    
    ext_mem_19168.references = NULL;
    
    struct memblock ext_mem_19167;
    
    ext_mem_19167.references = NULL;
    
    struct memblock ext_mem_19140;
    
    ext_mem_19140.references = NULL;
    
    struct memblock ext_mem_19139;
    
    ext_mem_19139.references = NULL;
    
    struct memblock mem_19130;
    
    mem_19130.references = NULL;
    
    struct memblock ext_mem_19152;
    
    ext_mem_19152.references = NULL;
    
    struct memblock ext_mem_19151;
    
    ext_mem_19151.references = NULL;
    
    struct memblock mem_19142;
    
    mem_19142.references = NULL;
    
    struct memblock ext_mem_19153;
    
    ext_mem_19153.references = NULL;
    
    struct memblock ext_mem_19165;
    
    ext_mem_19165.references = NULL;
    
    struct memblock ext_mem_19164;
    
    ext_mem_19164.references = NULL;
    
    struct memblock mem_19155;
    
    mem_19155.references = NULL;
    
    struct memblock ext_mem_19166;
    
    ext_mem_19166.references = NULL;
    
    struct memblock ext_mem_19101;
    
    ext_mem_19101.references = NULL;
    
    struct memblock ext_mem_19100;
    
    ext_mem_19100.references = NULL;
    
    struct memblock mem_19091;
    
    mem_19091.references = NULL;
    
    struct memblock ext_mem_19113;
    
    ext_mem_19113.references = NULL;
    
    struct memblock ext_mem_19112;
    
    ext_mem_19112.references = NULL;
    
    struct memblock mem_19103;
    
    mem_19103.references = NULL;
    
    struct memblock ext_mem_19114;
    
    ext_mem_19114.references = NULL;
    
    struct memblock ext_mem_19126;
    
    ext_mem_19126.references = NULL;
    
    struct memblock ext_mem_19125;
    
    ext_mem_19125.references = NULL;
    
    struct memblock mem_19116;
    
    mem_19116.references = NULL;
    
    struct memblock ext_mem_19127;
    
    ext_mem_19127.references = NULL;
    
    struct memblock mem_param_tmp_19303;
    
    mem_param_tmp_19303.references = NULL;
    
    struct memblock ext_mem_19212;
    
    ext_mem_19212.references = NULL;
    
    struct memblock mem_19203;
    
    mem_19203.references = NULL;
    
    struct memblock mem_param_tmp_19312;
    
    mem_param_tmp_19312.references = NULL;
    
    struct memblock ext_mem_19198;
    
    ext_mem_19198.references = NULL;
    
    struct memblock mem_19189;
    
    mem_19189.references = NULL;
    
    struct memblock mem_param_19187;
    
    mem_param_19187.references = NULL;
    
    struct memblock ext_mem_19201;
    
    ext_mem_19201.references = NULL;
    
    struct memblock ext_mem_19184;
    
    ext_mem_19184.references = NULL;
    
    struct memblock mem_19183;
    
    mem_19183.references = NULL;
    
    struct memblock ext_mem_19181;
    
    ext_mem_19181.references = NULL;
    
    struct memblock mem_param_19178;
    
    mem_param_19178.references = NULL;
    
    struct memblock ext_mem_19217;
    
    ext_mem_19217.references = NULL;
    
    struct memblock mem_19172;
    
    mem_19172.references = NULL;
    
    struct memblock ext_mem_19170;
    
    ext_mem_19170.references = NULL;
    
    struct memblock ext_mem_19169;
    
    ext_mem_19169.references = NULL;
    
    struct memblock ext_mem_19128;
    
    ext_mem_19128.references = NULL;
    
    struct memblock ext_mem_19089;
    
    ext_mem_19089.references = NULL;
    
    struct memblock ext_mem_19088;
    
    ext_mem_19088.references = NULL;
    
    struct memblock mem_out_19292;
    
    mem_out_19292.references = NULL;
    
    struct memblock mem_out_19291;
    
    mem_out_19291.references = NULL;
    
    struct memblock mem_out_19290;
    
    mem_out_19290.references = NULL;
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    int64_t prim_out_19293;
    int64_t prim_out_19294;
    int64_t prim_out_19295;
    int64_t prim_out_19296;
    bool prim_out_19297;
    bool prim_out_19298;
    bool prim_out_19299;
    bool prim_out_19300;
    int64_t prim_out_19301;
    
    // probes/stdlib-tests.fut:5:11-33
    
    int64_t arithmetic_res_17230;
    bool arithmetic_res_17231;
    
    if (futrts_normalise_10583(ctx, &ext_mem_19088, &arithmetic_res_17230, &arithmetic_res_17231, a_mem_19086, dz2080U_13919, negative_a_13921) != 0) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:6:11-33
    
    int64_t arithmetic_res_17233;
    bool arithmetic_res_17234;
    
    if (futrts_normalise_10583(ctx, &ext_mem_19089, &arithmetic_res_17233, &arithmetic_res_17234, b_mem_19087, dz2081U_13920, negative_b_13923) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:38:3-40:39
    
    bool cond_17236 = arithmetic_res_17231 == arithmetic_res_17234;
    
    // probes/stdlib-tests.fut:5:7-7:10
    
    bool x_17237 = !cond_17236;
    
    // stdlib/bigint.fut:21:14-42
    
    int64_t max_res_17284 = smax64(arithmetic_res_17230, arithmetic_res_17233);
    
    // stdlib/bigint.fut:22:52-58
    
    int64_t dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17285 = add64((int64_t) 1, max_res_17284);
    
    // stdlib/bigint.fut:28:7-32
    
    bool x_17310 = sle64((int64_t) 0, max_res_17284);
    
    // stdlib/bigint.fut:28:7-32
    
    bool y_17311 = slt64(max_res_17284, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17285);
    
    // stdlib/bigint.fut:28:7-32
    
    bool bounds_check_17312 = x_17310 && y_17311;
    bool protect_assert_disj_17972 = x_17237 || bounds_check_17312;
    
    // stdlib/bigint.fut:28:7-32
    
    bool index_certs_17313;
    
    if (!protect_assert_disj_17972) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) max_res_17284, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17285, "].", "-> #0  stdlib/bigint.fut:28:7-32\n   #1  stdlib/bigint.fut:38:37-52\n   #2  probes/stdlib-tests.fut:5:7-7:10\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:41:34-54
    
    bool normalise_arg_17374 = !arithmetic_res_17234;
    
    // stdlib/bigint.fut:41:34-54
    
    int64_t negate_res_17375;
    bool negate_res_17376;
    
    if (futrts_normalise_10583(ctx, &ext_mem_19128, &negate_res_17375, &negate_res_17376, ext_mem_19089, arithmetic_res_17233, normalise_arg_17374) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:38:3-40:39
    
    bool cond_17378 = arithmetic_res_17231 == negate_res_17376;
    
    // stdlib/bigint.fut:42:15-63
    
    bool x_17379 = !cond_17378;
    
    // stdlib/bigint.fut:21:14-42
    
    int64_t max_res_17426 = smax64(arithmetic_res_17230, negate_res_17375);
    
    // stdlib/bigint.fut:22:52-58
    
    int64_t dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17427 = add64((int64_t) 1, max_res_17426);
    
    // stdlib/bigint.fut:28:7-32
    
    bool x_17452 = sle64((int64_t) 0, max_res_17426);
    
    // stdlib/bigint.fut:28:7-32
    
    bool y_17453 = slt64(max_res_17426, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17427);
    
    // stdlib/bigint.fut:28:7-32
    
    bool bounds_check_17454 = x_17452 && y_17453;
    bool protect_assert_disj_17968 = x_17379 || bounds_check_17454;
    
    // stdlib/bigint.fut:28:7-32
    
    bool index_certs_17455;
    
    if (!protect_assert_disj_17968) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) max_res_17426, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17427, "].", "-> #0  stdlib/bigint.fut:28:7-32\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:5:7-7:23\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:52:13-43
    
    bool cond_17521 = arithmetic_res_17233 == (int64_t) 1;
    
    // stdlib/bigint.fut:52:13-43
    
    bool tmp_17522;
    
    if (cond_17521) {
        // stdlib/bigint.fut:52:32-38
        
        bool y_18877 = slt64((int64_t) 0, arithmetic_res_17233);
        
        // stdlib/bigint.fut:52:32-38
        
        bool index_certs_18878;
        
        if (!y_18877) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) arithmetic_res_17233, "].", "-> #0  stdlib/bigint.fut:52:32-38\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint.fut:52:32-38
        
        int32_t zeze_lhs_18879 = ((int32_t *) ext_mem_19089.mem)[(int64_t) 0];
        
        // stdlib/bigint.fut:52:39-43
        
        bool tmp_t_res_18880 = zeze_lhs_18879 == 0;
        
        tmp_17522 = tmp_t_res_18880;
    } else {
        tmp_17522 = 0;
    }
    // stdlib/bigint.fut:53:62-64
    
    int64_t u32_arg0_17527 = sub64(arithmetic_res_17233, (int64_t) 1);
    
    // stdlib/bigint.fut:53:48-65
    
    bool x_17528 = sle64((int64_t) 0, u32_arg0_17527);
    
    // stdlib/bigint.fut:53:48-65
    
    bool y_17529 = slt64(u32_arg0_17527, arithmetic_res_17233);
    
    // stdlib/bigint.fut:53:48-65
    
    bool bounds_check_17530 = x_17528 && y_17529;
    
    // stdlib/bigint.fut:53:48-65
    
    bool index_certs_17531;
    
    if (!bounds_check_17530) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u32_arg0_17527, "] out of bounds for array of shape [", (long long) arithmetic_res_17233, "].", "-> #0  stdlib/bigint.fut:53:48-65\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:53:48-65
    
    int32_t u32_arg0_17532 = ((int32_t *) ext_mem_19089.mem)[u32_arg0_17527];
    
    // stdlib/bigint.fut:53:40-65
    
    int64_t u32_res_17533 = zext_i32_i64(u32_arg0_17532);
    
    // stdlib/bigint.fut:53:65-67
    
    int64_t zs_rhs_17534 = add64((int64_t) 1, u32_res_17533);
    
    // stdlib/bigint.fut:53:38-67
    
    bool zzero_17535 = zs_rhs_17534 == (int64_t) 0;
    
    // stdlib/bigint.fut:53:38-67
    
    bool nonzzero_17536 = !zzero_17535;
    
    // stdlib/bigint.fut:53:38-67
    
    bool nonzzero_cert_17537;
    
    if (!nonzzero_17536) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/bigint.fut:53:38-67\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:53:38-67
    
    int64_t u64_arg0_17538 = udiv64((int64_t) 4294967296, zs_rhs_17534);
    
    // stdlib/bigint.fut:53:16-67
    
    int32_t u64_res_17539 = zext_i64_i32(u64_arg0_17538);
    
    // stdlib/bigint.fut:54:19-37
    
    int64_t numerator_17540;
    
    if (futrts_mul_abs_10600(ctx, &ext_mem_19169, &numerator_17540, ext_mem_19088, arithmetic_res_17230, u64_res_17539) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:55:17-35
    
    int64_t divisor_17542;
    
    if (futrts_mul_abs_10600(ctx, &ext_mem_19170, &divisor_17542, ext_mem_19089, arithmetic_res_17233, u64_res_17539) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:56:42-57
    
    int64_t zp_lhs_17544 = sub64(numerator_17540, divisor_17542);
    
    // stdlib/bigint.fut:56:57-59
    
    int64_t max_arg1_17545 = add64((int64_t) 1, zp_lhs_17544);
    
    // stdlib/bigint.fut:56:15-59
    
    int64_t max_res_17546 = smax64((int64_t) 1, max_arg1_17545);
    int64_t bytes_19171 = (int64_t) 4 * max_res_17546;
    
    if (memblock_alloc(ctx, &mem_19172, bytes_19171, "mem_19172")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:57:37-57
    for (int64_t nest_i_19302 = 0; nest_i_19302 < max_res_17546; nest_i_19302++) {
        ((int32_t *) mem_19172.mem)[nest_i_19302] = 0;
    }
    // stdlib/bigint.fut:57:23-69:15
    
    int64_t w_minus_1_17548 = sub64(max_res_17546, (int64_t) 1);
    
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool loop_nonempty_17549 = slt64((int64_t) 0, max_res_17546);
    
    // stdlib/bigint.fut:62:103-105
    
    int64_t u32_arg0_17550 = sub64(divisor_17542, (int64_t) 1);
    
    // stdlib/bigint.fut:62:81-106
    
    bool x_17551 = sle64((int64_t) 0, u32_arg0_17550);
    
    // stdlib/bigint.fut:62:81-106
    
    bool y_17552 = slt64(u32_arg0_17550, divisor_17542);
    
    // stdlib/bigint.fut:62:81-106
    
    bool bounds_check_17553 = x_17551 && y_17552;
    
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool loop_not_taken_17554 = !loop_nonempty_17549;
    
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool protect_assert_disj_17555 = bounds_check_17553 || loop_not_taken_17554;
    
    // stdlib/bigint.fut:62:81-106
    
    bool index_certs_17556;
    
    if (!protect_assert_disj_17555) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u32_arg0_17550, "] out of bounds for array of shape [", (long long) divisor_17542, "].", "-> #0  stdlib/bigint.fut:62:81-106\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:62:81-106
    
    int32_t u32_arg0_17557;
    
    if (loop_nonempty_17549) {
        // probes/stdlib-tests.fut:5:7-7:60
        
        int32_t x_18881 = ((int32_t *) ext_mem_19170.mem)[u32_arg0_17550];
        
        u32_arg0_17557 = x_18881;
    } else {
        u32_arg0_17557 = 0;
    }
    // probes/stdlib-tests.fut:5:7-7:60
    
    int64_t u32_res_17559 = zext_i32_i64(u32_arg0_17557);
    
    // stdlib/bigint.fut:62:72-106
    
    bool zzero_17560 = u32_res_17559 == (int64_t) 0;
    
    // stdlib/bigint.fut:62:72-106
    
    bool nonzzero_17561 = !zzero_17560;
    
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool protect_assert_disj_17562 = loop_not_taken_17554 || nonzzero_17561;
    
    // stdlib/bigint.fut:62:72-106
    
    bool nonzzero_cert_17563;
    
    if (!protect_assert_disj_17562) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "division by zero", "-> #0  stdlib/bigint.fut:62:72-106\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:57:23-69:15
    
    int64_t quotient_res_17564;
    int64_t loop_dz2081Uz2080U_17568;
    
    if (memblock_set(ctx, &mem_param_19178, &ext_mem_19169, "ext_mem_19169") != 0)
        return 1;
    loop_dz2081Uz2080U_17568 = numerator_17540;
    for (int64_t i_17567 = 0; i_17567 < max_res_17546; i_17567++) {
        // probes/stdlib-tests.fut:5:7-7:60
        
        int64_t binop_x_17571 = w_minus_1_17548 - i_17567;
        
        // stdlib/bigint.fut:59:38-44
        
        int64_t high_index_17572 = add64(divisor_17542, binop_x_17571);
        
        // stdlib/bigint.fut:60:18-76
        
        bool cond_17573 = slt64(high_index_17572, loop_dz2081Uz2080U_17568);
        
        // stdlib/bigint.fut:60:56-69
        
        bool x_17574 = sle64((int64_t) 0, high_index_17572);
        
        // stdlib/bigint.fut:60:56-69
        
        bool bounds_check_17575 = cond_17573 && x_17574;
        
        // probes/stdlib-tests.fut:5:7-7:60
        
        bool loop_not_taken_17576 = !cond_17573;
        
        // probes/stdlib-tests.fut:5:7-7:60
        
        bool protect_assert_disj_17577 = bounds_check_17575 || loop_not_taken_17576;
        
        // stdlib/bigint.fut:60:56-69
        
        bool index_certs_17578;
        
        if (!protect_assert_disj_17577) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) high_index_17572, "] out of bounds for array of shape [", (long long) loop_dz2081Uz2080U_17568, "].", "-> #0  stdlib/bigint.fut:60:56-69\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint.fut:60:56-69
        
        int32_t u32_arg0_17579;
        
        if (cond_17573) {
            // probes/stdlib-tests.fut:5:7-7:60
            
            int32_t x_18882 = ((int32_t *) mem_param_19178.mem)[high_index_17572];
            
            u32_arg0_17579 = x_18882;
        } else {
            u32_arg0_17579 = 0;
        }
        // stdlib/bigint.fut:60:48-69
        
        int64_t u32_res_17581 = zext_i32_i64(u32_arg0_17579);
        
        // stdlib/bigint.fut:60:18-76
        
        int64_t high_17582;
        
        if (cond_17573) {
            high_17582 = u32_res_17581;
        } else {
            high_17582 = (int64_t) 0;
        }
        // stdlib/bigint.fut:61:30-32
        
        int64_t zl_lhs_17583 = sub64(high_index_17572, (int64_t) 1);
        
        // stdlib/bigint.fut:61:17-79
        
        bool cond_17584 = slt64(zl_lhs_17583, loop_dz2081Uz2080U_17568);
        
        // stdlib/bigint.fut:61:57-72
        
        bool x_17585 = sle64((int64_t) 0, zl_lhs_17583);
        
        // stdlib/bigint.fut:61:57-72
        
        bool bounds_check_17586 = cond_17584 && x_17585;
        
        // probes/stdlib-tests.fut:5:7-7:60
        
        bool loop_not_taken_17587 = !cond_17584;
        
        // probes/stdlib-tests.fut:5:7-7:60
        
        bool protect_assert_disj_17588 = bounds_check_17586 || loop_not_taken_17587;
        
        // stdlib/bigint.fut:61:57-72
        
        bool index_certs_17589;
        
        if (!protect_assert_disj_17588) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_17583, "] out of bounds for array of shape [", (long long) loop_dz2081Uz2080U_17568, "].", "-> #0  stdlib/bigint.fut:61:57-72\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint.fut:61:57-72
        
        int32_t u32_arg0_17590;
        
        if (cond_17584) {
            // probes/stdlib-tests.fut:5:7-7:60
            
            int32_t x_18883 = ((int32_t *) mem_param_19178.mem)[zl_lhs_17583];
            
            u32_arg0_17590 = x_18883;
        } else {
            u32_arg0_17590 = 0;
        }
        // stdlib/bigint.fut:61:49-72
        
        int64_t u32_res_17592 = zext_i32_i64(u32_arg0_17590);
        
        // stdlib/bigint.fut:61:17-79
        
        int64_t low_17593;
        
        if (cond_17584) {
            low_17593 = u32_res_17592;
        } else {
            low_17593 = (int64_t) 0;
        }
        // stdlib/bigint.fut:62:61-66
        
        int64_t zb_lhs_17594 = shl64(high_17582, (int64_t) 32);
        
        // stdlib/bigint.fut:62:67-71
        
        int64_t zs_lhs_17595 = low_17593 | zb_lhs_17594;
        
        // stdlib/bigint.fut:62:72-106
        
        int64_t min_arg1_17596 = udiv64(zs_lhs_17595, u32_res_17559);
        
        // stdlib/bigint.fut:62:31-106
        
        int64_t unsign_arg0_17597 = umin64((int64_t) 4294967295, min_arg1_17596);
        
        // stdlib/bigint.fut:62:22-107
        
        int32_t u64_res_17598 = zext_i64_i32(unsign_arg0_17597);
        
        // stdlib/bigint.fut:64:63-87
        
        int64_t zpzp_rhs_17601;
        
        if (futrts_mul_abs_10600(ctx, &ext_mem_19181, &zpzp_rhs_17601, ext_mem_19170, divisor_17542, u64_res_17598) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:64:39-87
        
        int64_t conc_tmp_17603 = binop_x_17571 + zpzp_rhs_17601;
        int64_t bytes_19182 = (int64_t) 4 * conc_tmp_17603;
        int64_t bytes_19202 = (int64_t) 4 * loop_dz2081Uz2080U_17568;
        
        // stdlib/bigint.fut:68:11-27
        
        bool y_17723 = slt64(binop_x_17571, max_res_17546);
        
        // stdlib/bigint.fut:68:11-27
        
        bool index_certs_17724;
        
        if (!y_17723) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) binop_x_17571, "] out of bounds for array of shape [", (long long) max_res_17546, "].", "-> #0  stdlib/bigint.fut:68:11-27\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        if (memblock_alloc(ctx, &mem_19183, bytes_19182, "mem_19183")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:63:21-41
        for (int64_t nest_i_19307 = 0; nest_i_19307 < binop_x_17571; nest_i_19307++) {
            ((int32_t *) mem_19183.mem)[nest_i_19307] = 0;
        }
        // stdlib/bigint.fut:64:39-87
        
        int64_t tmp_offs_19308 = (int64_t) 0;
        
        if (!(tmp_offs_19308 == (int64_t) 0)) {
            // stdlib/bigint.fut:64:39-87
            lmad_copy_4b(ctx, 1, (uint32_t *) mem_19183.mem, tmp_offs_19308, (int64_t []) {(int64_t) 1}, (uint32_t *) mem_19183.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {binop_x_17571});
        }
        tmp_offs_19308 += binop_x_17571;
        // stdlib/bigint.fut:64:39-87
        lmad_copy_4b(ctx, 1, (uint32_t *) mem_19183.mem, tmp_offs_19308, (int64_t []) {(int64_t) 1}, (uint32_t *) ext_mem_19181.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {zpzp_rhs_17601});
        tmp_offs_19308 += zpzp_rhs_17601;
        if (memblock_unref(ctx, &ext_mem_19181, "ext_mem_19181") != 0)
            return 1;
        // stdlib/bigint.fut:64:22-87
        
        int64_t loopres_17605;
        bool loopres_17606;
        
        if (futrts_normalise_10583(ctx, &ext_mem_19184, &loopres_17605, &loopres_17606, mem_19183, conc_tmp_17603, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_19183, "mem_19183") != 0)
            return 1;
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_17608 = loopres_17605 == loop_dz2081Uz2080U_17568;
        
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_17609 = !cond_17608;
        
        // stdlib/bigint.fut:12:3-16:17
        
        int64_t compare_abs_res_17610;
        
        if (cond_17609) {
            // stdlib/bigint.fut:12:32-69
            
            bool cond_18884 = slt64(loopres_17605, loop_dz2081Uz2080U_17568);
            
            // stdlib/bigint.fut:12:32-69
            
            int64_t compare_abs_res_t_res_18885;
            
            if (cond_18884) {
                compare_abs_res_t_res_18885 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_18885 = (int64_t) 1;
            }
            compare_abs_res_17610 = compare_abs_res_t_res_18885;
        } else {
            // stdlib/bigint.fut:13:53-55
            
            int64_t tmp_17613 = sub64(loopres_17605, (int64_t) 1);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool cond_17614 = sle64((int64_t) 0, tmp_17613);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool compare_abs_res_f_res_17615;
            int64_t compare_abs_res_f_res_17616;
            int64_t compare_abs_res_f_res_17617;
            bool loop_while_17618;
            int64_t i_17619;
            int64_t result_17620;
            
            loop_while_17618 = cond_17614;
            i_17619 = tmp_17613;
            result_17620 = (int64_t) 0;
            while (loop_while_17618) {
                // stdlib/bigint.fut:15:20-24
                
                bool x_17622 = sle64((int64_t) 0, i_17619);
                
                // stdlib/bigint.fut:15:20-24
                
                bool y_17623 = slt64(i_17619, loopres_17605);
                
                // stdlib/bigint.fut:15:20-24
                
                bool bounds_check_17624 = x_17622 && y_17623;
                
                // stdlib/bigint.fut:15:20-24
                
                bool index_certs_17625;
                
                if (!bounds_check_17624) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17619, "] out of bounds for array of shape [", (long long) loopres_17605, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:65:29-66:77\n   #2  probes/stdlib-tests.fut:5:7-7:60\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:27-31
                
                bool y_17627 = slt64(i_17619, loop_dz2081Uz2080U_17568);
                
                // stdlib/bigint.fut:15:27-31
                
                bool bounds_check_17628 = x_17622 && y_17627;
                
                // stdlib/bigint.fut:15:27-31
                
                bool index_certs_17629;
                
                if (!bounds_check_17628) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17619, "] out of bounds for array of shape [", (long long) loop_dz2081Uz2080U_17568, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:65:29-66:77\n   #2  probes/stdlib-tests.fut:5:7-7:60\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:14-16
                
                int64_t tmp_17621 = sub64(i_17619, (int64_t) 1);
                
                // stdlib/bigint.fut:15:20-24
                
                int32_t zl_lhs_17626 = ((int32_t *) ext_mem_19184.mem)[i_17619];
                
                // stdlib/bigint.fut:15:27-31
                
                int32_t zl_rhs_17630 = ((int32_t *) mem_param_19178.mem)[i_17619];
                
                // stdlib/bigint.fut:15:17-73
                
                bool cond_17631 = ult32(zl_lhs_17626, zl_rhs_17630);
                
                // stdlib/bigint.fut:15:17-73
                
                int64_t tmp_17632;
                
                if (cond_17631) {
                    tmp_17632 = (int64_t) -1;
                } else {
                    // stdlib/bigint.fut:15:45-73
                    
                    bool cond_17633 = ult32(zl_rhs_17630, zl_lhs_17626);
                    
                    // stdlib/bigint.fut:65:29-66:77
                    
                    int64_t tmp_f_res_17634 = btoi_bool_i64(cond_17633);
                    
                    tmp_17632 = tmp_f_res_17634;
                }
                // stdlib/bigint.fut:14:16-37
                
                bool cond_17635 = sle64((int64_t) 0, tmp_17621);
                
                // stdlib/bigint.fut:14:33-37
                
                bool loop_cond_t_res_17636 = tmp_17632 == (int64_t) 0;
                
                // stdlib/bigint.fut:65:29-66:77
                
                bool x_17637 = cond_17635 && loop_cond_t_res_17636;
                bool loop_while_tmp_19309 = x_17637;
                int64_t i_tmp_19310 = tmp_17621;
                int64_t result_tmp_19311 = tmp_17632;
                
                loop_while_17618 = loop_while_tmp_19309;
                i_17619 = i_tmp_19310;
                result_17620 = result_tmp_19311;
            }
            compare_abs_res_f_res_17615 = loop_while_17618;
            compare_abs_res_f_res_17616 = i_17619;
            compare_abs_res_f_res_17617 = result_17620;
            compare_abs_res_17610 = compare_abs_res_f_res_17617;
        }
        // stdlib/bigint.fut:65:29-66:77
        
        bool loop_cond_17638 = slt64((int64_t) 0, compare_abs_res_17610);
        
        // stdlib/bigint.fut:65:29-66:77
        
        int64_t loopres_17639;
        bool loopres_17640;
        int32_t loopres_17641;
        int64_t loop_dz2083Uz2081U_17643;
        bool loop_while_17644;
        int32_t digit_17645;
        
        if (memblock_set(ctx, &mem_param_19187, &ext_mem_19184, "ext_mem_19184") != 0)
            return 1;
        loop_dz2083Uz2081U_17643 = loopres_17605;
        loop_while_17644 = loop_cond_17638;
        digit_17645 = u64_res_17598;
        while (loop_while_17644) {
            int64_t bytes_19188 = (int64_t) 4 * loop_dz2083Uz2081U_17643;
            
            // stdlib/bigint.fut:66:50-52
            
            int32_t tmp_17647 = sub32(digit_17645, 1);
            
            if (memblock_alloc(ctx, &mem_19189, bytes_19188, "mem_19189")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_19317 = 0; nest_i_19317 < loop_dz2083Uz2081U_17643; nest_i_19317++) {
                ((int32_t *) mem_19189.mem)[nest_i_19317] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_17650;
            int64_t borrow_17653 = (int64_t) 0;
            
            for (int64_t i_17651 = 0; i_17651 < loop_dz2083Uz2081U_17643; i_17651++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_17654 = ((int32_t *) mem_param_19187.mem)[i_17651];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_17655 = zext_i32_i64(u32_arg0_17654);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_17656 = slt64(i_17651, high_index_17572);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_17657;
                
                if (cond_17656) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_18888;
                    
                    if (!cond_17656) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17651, "] out of bounds for array of shape [", (long long) high_index_17572, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:66:53-76\n   #2  probes/stdlib-tests.fut:5:7-7:60\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    
                    bool index_concat_cmp_18889 = sle64(binop_x_17571, i_17651);
                    int32_t index_concat_branch_18890;
                    
                    if (index_concat_cmp_18889) {
                        int64_t index_concat_i_18891 = sub64(i_17651, binop_x_17571);
                        int32_t index_concat_18892 = ((int32_t *) ext_mem_19170.mem)[index_concat_i_18891];
                        
                        index_concat_branch_18890 = index_concat_18892;
                    } else {
                        index_concat_branch_18890 = 0;
                    }
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_18893 = zext_i32_i64(index_concat_branch_18890);
                    
                    zp_lhs_17657 = u32_res_18893;
                } else {
                    zp_lhs_17657 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_17661 = add64(borrow_17653, zp_lhs_17657);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_17662 = sub64(u32_res_17655, y_17661);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_17663 = zext_i64_i32(u64_arg0_17662);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_19189.mem)[i_17651] = u64_res_17663;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_17665 = ult64(u32_res_17655, y_17661);
                
                // stdlib/bigint.fut:66:53-76
                
                int64_t tmp_17666 = btoi_bool_i64(cond_17665);
                int64_t borrow_tmp_19319 = tmp_17666;
                
                borrow_17653 = borrow_tmp_19319;
            }
            sub_abs_res_17650 = borrow_17653;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_17667;
            bool sub_abs_res_17668;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19198, &sub_abs_res_17667, &sub_abs_res_17668, mem_19189, loop_dz2083Uz2081U_17643, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_19189, "mem_19189") != 0)
                return 1;
            // stdlib/bigint.fut:12:3-16:17
            
            bool cond_17670 = sub_abs_res_17667 == loop_dz2081Uz2080U_17568;
            
            // stdlib/bigint.fut:12:3-16:17
            
            bool cond_17671 = !cond_17670;
            
            // stdlib/bigint.fut:12:3-16:17
            
            int64_t compare_abs_res_17672;
            
            if (cond_17671) {
                // stdlib/bigint.fut:12:32-69
                
                bool cond_18894 = slt64(sub_abs_res_17667, loop_dz2081Uz2080U_17568);
                
                // stdlib/bigint.fut:12:32-69
                
                int64_t compare_abs_res_t_res_18895;
                
                if (cond_18894) {
                    compare_abs_res_t_res_18895 = (int64_t) -1;
                } else {
                    compare_abs_res_t_res_18895 = (int64_t) 1;
                }
                compare_abs_res_17672 = compare_abs_res_t_res_18895;
            } else {
                // stdlib/bigint.fut:13:53-55
                
                int64_t tmp_17675 = sub64(sub_abs_res_17667, (int64_t) 1);
                
                // stdlib/bigint.fut:13:26-15:74
                
                bool cond_17676 = sle64((int64_t) 0, tmp_17675);
                
                // stdlib/bigint.fut:13:26-15:74
                
                bool compare_abs_res_f_res_17677;
                int64_t compare_abs_res_f_res_17678;
                int64_t compare_abs_res_f_res_17679;
                bool loop_while_17680;
                int64_t i_17681;
                int64_t result_17682;
                
                loop_while_17680 = cond_17676;
                i_17681 = tmp_17675;
                result_17682 = (int64_t) 0;
                while (loop_while_17680) {
                    // stdlib/bigint.fut:15:20-24
                    
                    bool x_17684 = sle64((int64_t) 0, i_17681);
                    
                    // stdlib/bigint.fut:15:20-24
                    
                    bool y_17685 = slt64(i_17681, sub_abs_res_17667);
                    
                    // stdlib/bigint.fut:15:20-24
                    
                    bool bounds_check_17686 = x_17684 && y_17685;
                    
                    // stdlib/bigint.fut:15:20-24
                    
                    bool index_certs_17687;
                    
                    if (!bounds_check_17686) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17681, "] out of bounds for array of shape [", (long long) sub_abs_res_17667, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:66:15-36\n   #2  probes/stdlib-tests.fut:5:7-7:60\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:15:27-31
                    
                    bool y_17689 = slt64(i_17681, loop_dz2081Uz2080U_17568);
                    
                    // stdlib/bigint.fut:15:27-31
                    
                    bool bounds_check_17690 = x_17684 && y_17689;
                    
                    // stdlib/bigint.fut:15:27-31
                    
                    bool index_certs_17691;
                    
                    if (!bounds_check_17690) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17681, "] out of bounds for array of shape [", (long long) loop_dz2081Uz2080U_17568, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:66:15-36\n   #2  probes/stdlib-tests.fut:5:7-7:60\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:15:14-16
                    
                    int64_t tmp_17683 = sub64(i_17681, (int64_t) 1);
                    
                    // stdlib/bigint.fut:15:20-24
                    
                    int32_t zl_lhs_17688 = ((int32_t *) ext_mem_19198.mem)[i_17681];
                    
                    // stdlib/bigint.fut:15:27-31
                    
                    int32_t zl_rhs_17692 = ((int32_t *) mem_param_19178.mem)[i_17681];
                    
                    // stdlib/bigint.fut:15:17-73
                    
                    bool cond_17693 = ult32(zl_lhs_17688, zl_rhs_17692);
                    
                    // stdlib/bigint.fut:15:17-73
                    
                    int64_t tmp_17694;
                    
                    if (cond_17693) {
                        tmp_17694 = (int64_t) -1;
                    } else {
                        // stdlib/bigint.fut:15:45-73
                        
                        bool cond_17695 = ult32(zl_rhs_17692, zl_lhs_17688);
                        
                        // stdlib/bigint.fut:66:15-36
                        
                        int64_t tmp_f_res_17696 = btoi_bool_i64(cond_17695);
                        
                        tmp_17694 = tmp_f_res_17696;
                    }
                    // stdlib/bigint.fut:14:16-37
                    
                    bool cond_17697 = sle64((int64_t) 0, tmp_17683);
                    
                    // stdlib/bigint.fut:14:33-37
                    
                    bool loop_cond_t_res_17698 = tmp_17694 == (int64_t) 0;
                    
                    // stdlib/bigint.fut:66:15-36
                    
                    bool x_17699 = cond_17697 && loop_cond_t_res_17698;
                    bool loop_while_tmp_19320 = x_17699;
                    int64_t i_tmp_19321 = tmp_17683;
                    int64_t result_tmp_19322 = tmp_17694;
                    
                    loop_while_17680 = loop_while_tmp_19320;
                    i_17681 = i_tmp_19321;
                    result_17682 = result_tmp_19322;
                }
                compare_abs_res_f_res_17677 = loop_while_17680;
                compare_abs_res_f_res_17678 = i_17681;
                compare_abs_res_f_res_17679 = result_17682;
                compare_abs_res_17672 = compare_abs_res_f_res_17679;
            }
            // stdlib/bigint.fut:66:37-40
            
            bool loop_cond_17700 = slt64((int64_t) 0, compare_abs_res_17672);
            
            if (memblock_set(ctx, &mem_param_tmp_19312, &ext_mem_19198, "ext_mem_19198") != 0)
                return 1;
            
            int64_t loop_dz2083Uz2081U_tmp_19313 = sub_abs_res_17667;
            bool loop_while_tmp_19314 = loop_cond_17700;
            int32_t digit_tmp_19315 = tmp_17647;
            
            if (memblock_set(ctx, &mem_param_19187, &mem_param_tmp_19312, "mem_param_tmp_19312") != 0)
                return 1;
            loop_dz2083Uz2081U_17643 = loop_dz2083Uz2081U_tmp_19313;
            loop_while_17644 = loop_while_tmp_19314;
            digit_17645 = digit_tmp_19315;
        }
        if (memblock_set(ctx, &ext_mem_19201, &mem_param_19187, "mem_param_19187") != 0)
            return 1;
        loopres_17639 = loop_dz2083Uz2081U_17643;
        loopres_17640 = loop_while_17644;
        loopres_17641 = digit_17645;
        if (memblock_unref(ctx, &ext_mem_19184, "ext_mem_19184") != 0)
            return 1;
        if (memblock_alloc(ctx, &mem_19203, bytes_19202, "mem_19203")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:31:44-69
        for (int64_t nest_i_19323 = 0; nest_i_19323 < loop_dz2081Uz2080U_17568; nest_i_19323++) {
            ((int32_t *) mem_19203.mem)[nest_i_19323] = 0;
        }
        // stdlib/bigint.fut:31:23-35:39
        
        int64_t sub_abs_res_17703;
        int64_t borrow_17706 = (int64_t) 0;
        
        for (int64_t i_17704 = 0; i_17704 < loop_dz2081Uz2080U_17568; i_17704++) {
            // stdlib/bigint.fut:32:21-25
            
            int32_t u32_arg0_17707 = ((int32_t *) mem_param_19178.mem)[i_17704];
            
            // stdlib/bigint.fut:32:13-25
            
            int64_t u32_res_17708 = zext_i32_i64(u32_arg0_17707);
            
            // stdlib/bigint.fut:33:14-54
            
            bool cond_17709 = slt64(i_17704, loopres_17639);
            
            // stdlib/bigint.fut:33:14-54
            
            int64_t zp_lhs_17710;
            
            if (cond_17709) {
                // stdlib/bigint.fut:33:43-47
                
                bool index_certs_18896;
                
                if (!cond_17709) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17704, "] out of bounds for array of shape [", (long long) loopres_17639, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:67:15-32\n   #2  probes/stdlib-tests.fut:5:7-7:60\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:33:43-47
                
                int32_t u32_arg0_18897 = ((int32_t *) ext_mem_19201.mem)[i_17704];
                
                // stdlib/bigint.fut:33:35-47
                
                int64_t u32_res_18898 = zext_i32_i64(u32_arg0_18897);
                
                zp_lhs_17710 = u32_res_18898;
            } else {
                zp_lhs_17710 = (int64_t) 0;
            }
            // stdlib/bigint.fut:33:55-62
            
            int64_t y_17714 = add64(borrow_17706, zp_lhs_17710);
            
            // stdlib/bigint.fut:34:28-30
            
            int64_t u64_arg0_17715 = sub64(u32_res_17708, y_17714);
            
            // stdlib/bigint.fut:34:18-30
            
            int32_t u64_res_17716 = zext_i64_i32(u64_arg0_17715);
            
            // stdlib/bigint.fut:34:9-31
            ((int32_t *) mem_19203.mem)[i_17704] = u64_res_17716;
            // stdlib/bigint.fut:35:13-38
            
            bool cond_17718 = ult64(u32_res_17708, y_17714);
            
            // stdlib/bigint.fut:67:15-32
            
            int64_t tmp_17719 = btoi_bool_i64(cond_17718);
            int64_t borrow_tmp_19325 = tmp_17719;
            
            borrow_17706 = borrow_tmp_19325;
        }
        sub_abs_res_17703 = borrow_17706;
        if (memblock_unref(ctx, &ext_mem_19201, "ext_mem_19201") != 0)
            return 1;
        // stdlib/bigint.fut:36:7-26
        
        int64_t sub_abs_res_17720;
        bool sub_abs_res_17721;
        
        if (futrts_normalise_10583(ctx, &ext_mem_19212, &sub_abs_res_17720, &sub_abs_res_17721, mem_19203, loop_dz2081Uz2080U_17568, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_19203, "mem_19203") != 0)
            return 1;
        // stdlib/bigint.fut:68:11-27
        ((int32_t *) mem_19172.mem)[binop_x_17571] = loopres_17641;
        if (memblock_set(ctx, &mem_param_tmp_19303, &ext_mem_19212, "ext_mem_19212") != 0)
            return 1;
        
        int64_t loop_dz2081Uz2080U_tmp_19304 = sub_abs_res_17720;
        
        if (memblock_set(ctx, &mem_param_19178, &mem_param_tmp_19303, "mem_param_tmp_19303") != 0)
            return 1;
        loop_dz2081Uz2080U_17568 = loop_dz2081Uz2080U_tmp_19304;
    }
    if (memblock_set(ctx, &ext_mem_19217, &mem_param_19178, "mem_param_19178") != 0)
        return 1;
    quotient_res_17564 = loop_dz2081Uz2080U_17568;
    if (memblock_unref(ctx, &ext_mem_19169, "ext_mem_19169") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_19170, "ext_mem_19170") != 0)
        return 1;
    // stdlib/bigint.fut:52:3-74:42
    
    bool assert_cond_17730 = !tmp_17522;
    
    // stdlib/bigint.fut:52:3-74:42
    
    bool assert_c_17731;
    
    if (!assert_cond_17730) {
        set_error(ctx, msgprintf("Error: %s\n\nBacktrace:\n%s", "Assertion is false: (!(length b.1 == 1 && b.1[0] == 0))", "-> #0  stdlib/bigint.fut:52:3-74:42\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:73:11-74:42
    
    bool cond_17733 = quotient_res_17564 == (int64_t) 1;
    
    // stdlib/bigint.fut:73:39-51
    
    bool y_17734 = slt64((int64_t) 0, quotient_res_17564);
    
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool loop_not_taken_17735 = !cond_17733;
    
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool protect_assert_disj_17736 = y_17734 || loop_not_taken_17735;
    
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool protect_assert_disj_17737 = cond_17236 || protect_assert_disj_17736;
    
    // stdlib/bigint.fut:73:39-51
    
    bool index_certs_17738;
    
    if (!protect_assert_disj_17737) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) quotient_res_17564, "].", "-> #0  stdlib/bigint.fut:73:39-51\n   #1  probes/stdlib-tests.fut:5:7-7:60\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:39:11-30
    
    int64_t zgze_lhs_17238;
    
    if (x_17237) {
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_18679 = arithmetic_res_17230 == arithmetic_res_17233;
        
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_18680 = !cond_18679;
        
        // stdlib/bigint.fut:12:3-16:17
        
        int64_t compare_abs_res_18681;
        
        if (cond_18680) {
            // stdlib/bigint.fut:12:32-69
            
            bool cond_18682 = slt64(arithmetic_res_17230, arithmetic_res_17233);
            
            // stdlib/bigint.fut:12:32-69
            
            int64_t compare_abs_res_t_res_18683;
            
            if (cond_18682) {
                compare_abs_res_t_res_18683 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_18683 = (int64_t) 1;
            }
            compare_abs_res_18681 = compare_abs_res_t_res_18683;
        } else {
            // stdlib/bigint.fut:13:53-55
            
            int64_t tmp_18684 = sub64(arithmetic_res_17230, (int64_t) 1);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool cond_18685 = sle64((int64_t) 0, tmp_18684);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool compare_abs_res_f_res_18686;
            int64_t compare_abs_res_f_res_18687;
            int64_t compare_abs_res_f_res_18688;
            bool loop_while_18689;
            int64_t i_18690;
            int64_t result_18691;
            
            loop_while_18689 = cond_18685;
            i_18690 = tmp_18684;
            result_18691 = (int64_t) 0;
            while (loop_while_18689) {
                // stdlib/bigint.fut:15:20-24
                
                bool x_18693 = sle64((int64_t) 0, i_18690);
                
                // stdlib/bigint.fut:15:20-24
                
                bool y_18694 = slt64(i_18690, arithmetic_res_17230);
                
                // stdlib/bigint.fut:15:20-24
                
                bool bounds_check_18695 = x_18693 && y_18694;
                
                // stdlib/bigint.fut:15:20-24
                
                bool index_certs_18696;
                
                if (!bounds_check_18695) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18690, "] out of bounds for array of shape [", (long long) arithmetic_res_17230, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  probes/stdlib-tests.fut:5:7-7:10\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:27-31
                
                bool y_18698 = slt64(i_18690, arithmetic_res_17233);
                
                // stdlib/bigint.fut:15:27-31
                
                bool bounds_check_18699 = x_18693 && y_18698;
                
                // stdlib/bigint.fut:15:27-31
                
                bool index_certs_18700;
                
                if (!bounds_check_18699) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18690, "] out of bounds for array of shape [", (long long) arithmetic_res_17233, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  probes/stdlib-tests.fut:5:7-7:10\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:14-16
                
                int64_t tmp_18692 = sub64(i_18690, (int64_t) 1);
                
                // stdlib/bigint.fut:15:20-24
                
                int32_t zl_lhs_18697 = ((int32_t *) ext_mem_19088.mem)[i_18690];
                
                // stdlib/bigint.fut:15:27-31
                
                int32_t zl_rhs_18701 = ((int32_t *) ext_mem_19089.mem)[i_18690];
                
                // stdlib/bigint.fut:15:17-73
                
                bool cond_18702 = ult32(zl_lhs_18697, zl_rhs_18701);
                
                // stdlib/bigint.fut:15:17-73
                
                int64_t tmp_18703;
                
                if (cond_18702) {
                    tmp_18703 = (int64_t) -1;
                } else {
                    // stdlib/bigint.fut:15:45-73
                    
                    bool cond_18704 = ult32(zl_rhs_18701, zl_lhs_18697);
                    
                    // probes/stdlib-tests.fut:5:7-7:10
                    
                    int64_t tmp_f_res_18705 = btoi_bool_i64(cond_18704);
                    
                    tmp_18703 = tmp_f_res_18705;
                }
                // stdlib/bigint.fut:14:16-37
                
                bool cond_18706 = sle64((int64_t) 0, tmp_18692);
                
                // stdlib/bigint.fut:14:33-37
                
                bool loop_cond_t_res_18707 = tmp_18703 == (int64_t) 0;
                
                // probes/stdlib-tests.fut:5:7-7:10
                
                bool x_18708 = cond_18706 && loop_cond_t_res_18707;
                bool loop_while_tmp_19326 = x_18708;
                int64_t i_tmp_19327 = tmp_18692;
                int64_t result_tmp_19328 = tmp_18703;
                
                loop_while_18689 = loop_while_tmp_19326;
                i_18690 = i_tmp_19327;
                result_18691 = result_tmp_19328;
            }
            compare_abs_res_f_res_18686 = loop_while_18689;
            compare_abs_res_f_res_18687 = i_18690;
            compare_abs_res_f_res_18688 = result_18691;
            compare_abs_res_18681 = compare_abs_res_f_res_18688;
        }
        zgze_lhs_17238 = compare_abs_res_18681;
    } else {
        zgze_lhs_17238 = (int64_t) 0;
    }
    // stdlib/bigint.fut:39:8-40:39
    
    bool cond_17280 = sle64((int64_t) 0, zgze_lhs_17238);
    int64_t bytes_19090 = (int64_t) 4 * arithmetic_res_17233;
    int64_t bytes_19102 = (int64_t) 4 * arithmetic_res_17230;
    int64_t bytes_19115 = (int64_t) 4 * dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17285;
    
    // stdlib/bigint.fut:38:3-40:39
    
    int64_t add_res_17281;
    bool add_res_17282;
    
    if (cond_17236) {
        if (memblock_alloc(ctx, &mem_19116, bytes_19115, "mem_19116")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:22:41-64
        for (int64_t nest_i_19329 = 0; nest_i_19329 < dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17285; nest_i_19329++) {
            ((int32_t *) mem_19116.mem)[nest_i_19329] = 0;
        }
        // stdlib/bigint.fut:22:21-27:23
        
        int64_t add_abs_res_18717;
        int64_t carry_18720 = (int64_t) 0;
        
        for (int64_t i_18718 = 0; i_18718 < max_res_17284; i_18718++) {
            // stdlib/bigint.fut:26:9-29
            
            bool y_18734 = slt64(i_18718, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17285);
            
            // stdlib/bigint.fut:26:9-29
            
            bool index_certs_18735;
            
            if (!y_18734) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18718, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17285, "].", "-> #0  stdlib/bigint.fut:26:9-29\n   #1  stdlib/bigint.fut:38:37-52\n   #2  probes/stdlib-tests.fut:5:7-7:10\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/bigint.fut:23:13-53
            
            bool cond_18721 = slt64(i_18718, arithmetic_res_17230);
            
            // stdlib/bigint.fut:23:13-53
            
            int64_t x_18722;
            
            if (cond_18721) {
                // stdlib/bigint.fut:23:42-46
                
                bool index_certs_18723;
                
                if (!cond_18721) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18718, "] out of bounds for array of shape [", (long long) arithmetic_res_17230, "].", "-> #0  stdlib/bigint.fut:23:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  probes/stdlib-tests.fut:5:7-7:10\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:23:42-46
                
                int32_t u32_arg0_18724 = ((int32_t *) ext_mem_19088.mem)[i_18718];
                
                // stdlib/bigint.fut:23:34-46
                
                int64_t u32_res_18725 = zext_i32_i64(u32_arg0_18724);
                
                x_18722 = u32_res_18725;
            } else {
                x_18722 = (int64_t) 0;
            }
            // stdlib/bigint.fut:24:13-53
            
            bool cond_18726 = slt64(i_18718, arithmetic_res_17233);
            
            // stdlib/bigint.fut:24:13-53
            
            int64_t y_18727;
            
            if (cond_18726) {
                // stdlib/bigint.fut:24:42-46
                
                bool index_certs_18728;
                
                if (!cond_18726) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18718, "] out of bounds for array of shape [", (long long) arithmetic_res_17233, "].", "-> #0  stdlib/bigint.fut:24:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  probes/stdlib-tests.fut:5:7-7:10\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:24:42-46
                
                int32_t u32_arg0_18729 = ((int32_t *) ext_mem_19089.mem)[i_18718];
                
                // stdlib/bigint.fut:24:34-46
                
                int64_t u32_res_18730 = zext_i32_i64(u32_arg0_18729);
                
                y_18727 = u32_res_18730;
            } else {
                y_18727 = (int64_t) 0;
            }
            // stdlib/bigint.fut:25:16-18
            
            int64_t zp_lhs_18731 = add64(x_18722, y_18727);
            
            // stdlib/bigint.fut:25:18-24
            
            int64_t sum_18732 = add64(carry_18720, zp_lhs_18731);
            
            // stdlib/bigint.fut:26:18-29
            
            int32_t u64_res_18733 = zext_i64_i32(sum_18732);
            
            // stdlib/bigint.fut:26:9-29
            ((int32_t *) mem_19116.mem)[i_18718] = u64_res_18733;
            // stdlib/bigint.fut:27:17-22
            
            int64_t tmp_18737 = lshr64(sum_18732, (int64_t) 32);
            int64_t carry_tmp_19331 = tmp_18737;
            
            carry_18720 = carry_tmp_19331;
        }
        add_abs_res_18717 = carry_18720;
        // stdlib/bigint.fut:28:19-32
        
        int32_t u64_res_18738 = zext_i64_i32(add_abs_res_18717);
        
        // stdlib/bigint.fut:28:7-32
        ((int32_t *) mem_19116.mem)[max_res_17284] = u64_res_18738;
        // stdlib/bigint.fut:29:7-26
        
        int64_t add_abs_res_18740;
        bool add_abs_res_18741;
        
        if (futrts_normalise_10583(ctx, &ext_mem_19125, &add_abs_res_18740, &add_abs_res_18741, mem_19116, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17285, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_19116, "mem_19116") != 0)
            return 1;
        // stdlib/bigint.fut:38:22-52
        
        int64_t add_res_t_res_18743;
        bool add_res_t_res_18744;
        
        if (futrts_normalise_10583(ctx, &ext_mem_19126, &add_res_t_res_18743, &add_res_t_res_18744, ext_mem_19125, add_abs_res_18740, arithmetic_res_17231) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_19125, "ext_mem_19125") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_19127, &ext_mem_19126, "ext_mem_19126") != 0)
            return 1;
        add_res_17281 = add_res_t_res_18743;
        add_res_17282 = add_res_t_res_18744;
    } else {
        // stdlib/bigint.fut:39:8-40:39
        
        int64_t add_res_f_res_17321;
        bool add_res_f_res_17322;
        
        if (cond_17280) {
            if (memblock_alloc(ctx, &mem_19103, bytes_19102, "mem_19103")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_19332 = 0; nest_i_19332 < arithmetic_res_17230; nest_i_19332++) {
                ((int32_t *) mem_19103.mem)[nest_i_19332] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_18751;
            int64_t borrow_18754 = (int64_t) 0;
            
            for (int64_t i_18752 = 0; i_18752 < arithmetic_res_17230; i_18752++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_18755 = ((int32_t *) ext_mem_19088.mem)[i_18752];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_18756 = zext_i32_i64(u32_arg0_18755);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_18757 = slt64(i_18752, arithmetic_res_17233);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_18758;
                
                if (cond_18757) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_18759;
                    
                    if (!cond_18757) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18752, "] out of bounds for array of shape [", (long long) arithmetic_res_17233, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:39:56-71\n   #2  probes/stdlib-tests.fut:5:7-7:10\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:33:43-47
                    
                    int32_t u32_arg0_18760 = ((int32_t *) ext_mem_19089.mem)[i_18752];
                    
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_18761 = zext_i32_i64(u32_arg0_18760);
                    
                    zp_lhs_18758 = u32_res_18761;
                } else {
                    zp_lhs_18758 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_18762 = add64(borrow_18754, zp_lhs_18758);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_18763 = sub64(u32_res_18756, y_18762);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_18764 = zext_i64_i32(u64_arg0_18763);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_19103.mem)[i_18752] = u64_res_18764;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_18766 = ult64(u32_res_18756, y_18762);
                
                // stdlib/bigint.fut:39:56-71
                
                int64_t tmp_18767 = btoi_bool_i64(cond_18766);
                int64_t borrow_tmp_19334 = tmp_18767;
                
                borrow_18754 = borrow_tmp_19334;
            }
            sub_abs_res_18751 = borrow_18754;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_18768;
            bool sub_abs_res_18769;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19112, &sub_abs_res_18768, &sub_abs_res_18769, mem_19103, arithmetic_res_17230, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_19103, "mem_19103") != 0)
                return 1;
            // stdlib/bigint.fut:39:41-71
            
            int64_t add_res_f_res_t_res_18771;
            bool add_res_f_res_t_res_18772;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19113, &add_res_f_res_t_res_18771, &add_res_f_res_t_res_18772, ext_mem_19112, sub_abs_res_18768, arithmetic_res_17231) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_19112, "ext_mem_19112") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19114, &ext_mem_19113, "ext_mem_19113") != 0)
                return 1;
            add_res_f_res_17321 = add_res_f_res_t_res_18771;
            add_res_f_res_17322 = add_res_f_res_t_res_18772;
        } else {
            if (memblock_alloc(ctx, &mem_19091, bytes_19090, "mem_19091")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_19335 = 0; nest_i_19335 < arithmetic_res_17233; nest_i_19335++) {
                ((int32_t *) mem_19091.mem)[nest_i_19335] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_17351;
            int64_t borrow_17354 = (int64_t) 0;
            
            for (int64_t i_17352 = 0; i_17352 < arithmetic_res_17233; i_17352++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_17355 = ((int32_t *) ext_mem_19089.mem)[i_17352];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_17356 = zext_i32_i64(u32_arg0_17355);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_17357 = slt64(i_17352, arithmetic_res_17230);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_17358;
                
                if (cond_17357) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_18774;
                    
                    if (!cond_17357) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17352, "] out of bounds for array of shape [", (long long) arithmetic_res_17230, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:40:23-38\n   #2  probes/stdlib-tests.fut:5:7-7:10\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:33:43-47
                    
                    int32_t u32_arg0_18775 = ((int32_t *) ext_mem_19088.mem)[i_17352];
                    
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_18776 = zext_i32_i64(u32_arg0_18775);
                    
                    zp_lhs_17358 = u32_res_18776;
                } else {
                    zp_lhs_17358 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_17362 = add64(borrow_17354, zp_lhs_17358);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_17363 = sub64(u32_res_17356, y_17362);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_17364 = zext_i64_i32(u64_arg0_17363);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_19091.mem)[i_17352] = u64_res_17364;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_17366 = ult64(u32_res_17356, y_17362);
                
                // stdlib/bigint.fut:40:23-38
                
                int64_t tmp_17367 = btoi_bool_i64(cond_17366);
                int64_t borrow_tmp_19337 = tmp_17367;
                
                borrow_17354 = borrow_tmp_19337;
            }
            sub_abs_res_17351 = borrow_17354;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_17368;
            bool sub_abs_res_17369;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19100, &sub_abs_res_17368, &sub_abs_res_17369, mem_19091, arithmetic_res_17233, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_19091, "mem_19091") != 0)
                return 1;
            // stdlib/bigint.fut:40:8-38
            
            int64_t add_res_f_res_f_res_17371;
            bool add_res_f_res_f_res_17372;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19101, &add_res_f_res_f_res_17371, &add_res_f_res_f_res_17372, ext_mem_19100, sub_abs_res_17368, arithmetic_res_17234) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_19100, "ext_mem_19100") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19114, &ext_mem_19101, "ext_mem_19101") != 0)
                return 1;
            add_res_f_res_17321 = add_res_f_res_f_res_17371;
            add_res_f_res_17322 = add_res_f_res_f_res_17372;
        }
        if (memblock_set(ctx, &ext_mem_19127, &ext_mem_19114, "ext_mem_19114") != 0)
            return 1;
        add_res_17281 = add_res_f_res_17321;
        add_res_17282 = add_res_f_res_17322;
    }
    // stdlib/bigint.fut:39:11-30
    
    int64_t zgze_lhs_17380;
    
    if (x_17379) {
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_18779 = arithmetic_res_17230 == negate_res_17375;
        
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_18780 = !cond_18779;
        
        // stdlib/bigint.fut:12:3-16:17
        
        int64_t compare_abs_res_18781;
        
        if (cond_18780) {
            // stdlib/bigint.fut:12:32-69
            
            bool cond_18782 = slt64(arithmetic_res_17230, negate_res_17375);
            
            // stdlib/bigint.fut:12:32-69
            
            int64_t compare_abs_res_t_res_18783;
            
            if (cond_18782) {
                compare_abs_res_t_res_18783 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_18783 = (int64_t) 1;
            }
            compare_abs_res_18781 = compare_abs_res_t_res_18783;
        } else {
            // stdlib/bigint.fut:13:53-55
            
            int64_t tmp_18784 = sub64(arithmetic_res_17230, (int64_t) 1);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool cond_18785 = sle64((int64_t) 0, tmp_18784);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool compare_abs_res_f_res_18786;
            int64_t compare_abs_res_f_res_18787;
            int64_t compare_abs_res_f_res_18788;
            bool loop_while_18789;
            int64_t i_18790;
            int64_t result_18791;
            
            loop_while_18789 = cond_18785;
            i_18790 = tmp_18784;
            result_18791 = (int64_t) 0;
            while (loop_while_18789) {
                // stdlib/bigint.fut:15:20-24
                
                bool x_18793 = sle64((int64_t) 0, i_18790);
                
                // stdlib/bigint.fut:15:20-24
                
                bool y_18794 = slt64(i_18790, arithmetic_res_17230);
                
                // stdlib/bigint.fut:15:20-24
                
                bool bounds_check_18795 = x_18793 && y_18794;
                
                // stdlib/bigint.fut:15:20-24
                
                bool index_certs_18796;
                
                if (!bounds_check_18795) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18790, "] out of bounds for array of shape [", (long long) arithmetic_res_17230, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:42:15-63\n   #2  probes/stdlib-tests.fut:5:7-7:23\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:27-31
                
                bool y_18798 = slt64(i_18790, negate_res_17375);
                
                // stdlib/bigint.fut:15:27-31
                
                bool bounds_check_18799 = x_18793 && y_18798;
                
                // stdlib/bigint.fut:15:27-31
                
                bool index_certs_18800;
                
                if (!bounds_check_18799) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18790, "] out of bounds for array of shape [", (long long) negate_res_17375, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:42:15-63\n   #2  probes/stdlib-tests.fut:5:7-7:23\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:14-16
                
                int64_t tmp_18792 = sub64(i_18790, (int64_t) 1);
                
                // stdlib/bigint.fut:15:20-24
                
                int32_t zl_lhs_18797 = ((int32_t *) ext_mem_19088.mem)[i_18790];
                
                // stdlib/bigint.fut:15:27-31
                
                int32_t zl_rhs_18801 = ((int32_t *) ext_mem_19128.mem)[i_18790];
                
                // stdlib/bigint.fut:15:17-73
                
                bool cond_18802 = ult32(zl_lhs_18797, zl_rhs_18801);
                
                // stdlib/bigint.fut:15:17-73
                
                int64_t tmp_18803;
                
                if (cond_18802) {
                    tmp_18803 = (int64_t) -1;
                } else {
                    // stdlib/bigint.fut:15:45-73
                    
                    bool cond_18804 = ult32(zl_rhs_18801, zl_lhs_18797);
                    
                    // stdlib/bigint.fut:42:15-63
                    
                    int64_t tmp_f_res_18805 = btoi_bool_i64(cond_18804);
                    
                    tmp_18803 = tmp_f_res_18805;
                }
                // stdlib/bigint.fut:14:16-37
                
                bool cond_18806 = sle64((int64_t) 0, tmp_18792);
                
                // stdlib/bigint.fut:14:33-37
                
                bool loop_cond_t_res_18807 = tmp_18803 == (int64_t) 0;
                
                // stdlib/bigint.fut:42:15-63
                
                bool x_18808 = cond_18806 && loop_cond_t_res_18807;
                bool loop_while_tmp_19338 = x_18808;
                int64_t i_tmp_19339 = tmp_18792;
                int64_t result_tmp_19340 = tmp_18803;
                
                loop_while_18789 = loop_while_tmp_19338;
                i_18790 = i_tmp_19339;
                result_18791 = result_tmp_19340;
            }
            compare_abs_res_f_res_18786 = loop_while_18789;
            compare_abs_res_f_res_18787 = i_18790;
            compare_abs_res_f_res_18788 = result_18791;
            compare_abs_res_18781 = compare_abs_res_f_res_18788;
        }
        zgze_lhs_17380 = compare_abs_res_18781;
    } else {
        zgze_lhs_17380 = (int64_t) 0;
    }
    // stdlib/bigint.fut:39:8-40:39
    
    bool cond_17422 = sle64((int64_t) 0, zgze_lhs_17380);
    int64_t bytes_19129 = (int64_t) 4 * negate_res_17375;
    int64_t bytes_19154 = (int64_t) 4 * dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17427;
    
    // stdlib/bigint.fut:38:3-40:39
    
    int64_t add_res_17423;
    bool add_res_17424;
    
    if (cond_17378) {
        if (memblock_alloc(ctx, &mem_19155, bytes_19154, "mem_19155")) {
            err = 1;
            goto cleanup;
        }
        // stdlib/bigint.fut:22:41-64
        for (int64_t nest_i_19341 = 0; nest_i_19341 < dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17427; nest_i_19341++) {
            ((int32_t *) mem_19155.mem)[nest_i_19341] = 0;
        }
        // stdlib/bigint.fut:22:21-27:23
        
        int64_t add_abs_res_18817;
        int64_t carry_18820 = (int64_t) 0;
        
        for (int64_t i_18818 = 0; i_18818 < max_res_17426; i_18818++) {
            // stdlib/bigint.fut:26:9-29
            
            bool y_18834 = slt64(i_18818, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17427);
            
            // stdlib/bigint.fut:26:9-29
            
            bool index_certs_18835;
            
            if (!y_18834) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18818, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17427, "].", "-> #0  stdlib/bigint.fut:26:9-29\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:5:7-7:23\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/bigint.fut:23:13-53
            
            bool cond_18821 = slt64(i_18818, arithmetic_res_17230);
            
            // stdlib/bigint.fut:23:13-53
            
            int64_t x_18822;
            
            if (cond_18821) {
                // stdlib/bigint.fut:23:42-46
                
                bool index_certs_18823;
                
                if (!cond_18821) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18818, "] out of bounds for array of shape [", (long long) arithmetic_res_17230, "].", "-> #0  stdlib/bigint.fut:23:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:5:7-7:23\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:23:42-46
                
                int32_t u32_arg0_18824 = ((int32_t *) ext_mem_19088.mem)[i_18818];
                
                // stdlib/bigint.fut:23:34-46
                
                int64_t u32_res_18825 = zext_i32_i64(u32_arg0_18824);
                
                x_18822 = u32_res_18825;
            } else {
                x_18822 = (int64_t) 0;
            }
            // stdlib/bigint.fut:24:13-53
            
            bool cond_18826 = slt64(i_18818, negate_res_17375);
            
            // stdlib/bigint.fut:24:13-53
            
            int64_t y_18827;
            
            if (cond_18826) {
                // stdlib/bigint.fut:24:42-46
                
                bool index_certs_18828;
                
                if (!cond_18826) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18818, "] out of bounds for array of shape [", (long long) negate_res_17375, "].", "-> #0  stdlib/bigint.fut:24:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:5:7-7:23\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:24:42-46
                
                int32_t u32_arg0_18829 = ((int32_t *) ext_mem_19128.mem)[i_18818];
                
                // stdlib/bigint.fut:24:34-46
                
                int64_t u32_res_18830 = zext_i32_i64(u32_arg0_18829);
                
                y_18827 = u32_res_18830;
            } else {
                y_18827 = (int64_t) 0;
            }
            // stdlib/bigint.fut:25:16-18
            
            int64_t zp_lhs_18831 = add64(x_18822, y_18827);
            
            // stdlib/bigint.fut:25:18-24
            
            int64_t sum_18832 = add64(carry_18820, zp_lhs_18831);
            
            // stdlib/bigint.fut:26:18-29
            
            int32_t u64_res_18833 = zext_i64_i32(sum_18832);
            
            // stdlib/bigint.fut:26:9-29
            ((int32_t *) mem_19155.mem)[i_18818] = u64_res_18833;
            // stdlib/bigint.fut:27:17-22
            
            int64_t tmp_18837 = lshr64(sum_18832, (int64_t) 32);
            int64_t carry_tmp_19343 = tmp_18837;
            
            carry_18820 = carry_tmp_19343;
        }
        add_abs_res_18817 = carry_18820;
        // stdlib/bigint.fut:28:19-32
        
        int32_t u64_res_18838 = zext_i64_i32(add_abs_res_18817);
        
        // stdlib/bigint.fut:28:7-32
        ((int32_t *) mem_19155.mem)[max_res_17426] = u64_res_18838;
        // stdlib/bigint.fut:29:7-26
        
        int64_t add_abs_res_18840;
        bool add_abs_res_18841;
        
        if (futrts_normalise_10583(ctx, &ext_mem_19164, &add_abs_res_18840, &add_abs_res_18841, mem_19155, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17427, 0) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &mem_19155, "mem_19155") != 0)
            return 1;
        // stdlib/bigint.fut:38:22-52
        
        int64_t add_res_t_res_18843;
        bool add_res_t_res_18844;
        
        if (futrts_normalise_10583(ctx, &ext_mem_19165, &add_res_t_res_18843, &add_res_t_res_18844, ext_mem_19164, add_abs_res_18840, arithmetic_res_17231) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_19164, "ext_mem_19164") != 0)
            return 1;
        if (memblock_set(ctx, &ext_mem_19166, &ext_mem_19165, "ext_mem_19165") != 0)
            return 1;
        add_res_17423 = add_res_t_res_18843;
        add_res_17424 = add_res_t_res_18844;
    } else {
        // stdlib/bigint.fut:39:8-40:39
        
        int64_t add_res_f_res_17463;
        bool add_res_f_res_17464;
        
        if (cond_17422) {
            if (memblock_alloc(ctx, &mem_19142, bytes_19102, "mem_19142")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_19344 = 0; nest_i_19344 < arithmetic_res_17230; nest_i_19344++) {
                ((int32_t *) mem_19142.mem)[nest_i_19344] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_18851;
            int64_t borrow_18854 = (int64_t) 0;
            
            for (int64_t i_18852 = 0; i_18852 < arithmetic_res_17230; i_18852++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_18855 = ((int32_t *) ext_mem_19088.mem)[i_18852];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_18856 = zext_i32_i64(u32_arg0_18855);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_18857 = slt64(i_18852, negate_res_17375);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_18858;
                
                if (cond_18857) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_18859;
                    
                    if (!cond_18857) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18852, "] out of bounds for array of shape [", (long long) negate_res_17375, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:39:56-71\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:5:7-7:23\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:33:43-47
                    
                    int32_t u32_arg0_18860 = ((int32_t *) ext_mem_19128.mem)[i_18852];
                    
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_18861 = zext_i32_i64(u32_arg0_18860);
                    
                    zp_lhs_18858 = u32_res_18861;
                } else {
                    zp_lhs_18858 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_18862 = add64(borrow_18854, zp_lhs_18858);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_18863 = sub64(u32_res_18856, y_18862);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_18864 = zext_i64_i32(u64_arg0_18863);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_19142.mem)[i_18852] = u64_res_18864;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_18866 = ult64(u32_res_18856, y_18862);
                
                // stdlib/bigint.fut:39:56-71
                
                int64_t tmp_18867 = btoi_bool_i64(cond_18866);
                int64_t borrow_tmp_19346 = tmp_18867;
                
                borrow_18854 = borrow_tmp_19346;
            }
            sub_abs_res_18851 = borrow_18854;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_18868;
            bool sub_abs_res_18869;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19151, &sub_abs_res_18868, &sub_abs_res_18869, mem_19142, arithmetic_res_17230, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_19142, "mem_19142") != 0)
                return 1;
            // stdlib/bigint.fut:39:41-71
            
            int64_t add_res_f_res_t_res_18871;
            bool add_res_f_res_t_res_18872;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19152, &add_res_f_res_t_res_18871, &add_res_f_res_t_res_18872, ext_mem_19151, sub_abs_res_18868, arithmetic_res_17231) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_19151, "ext_mem_19151") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19153, &ext_mem_19152, "ext_mem_19152") != 0)
                return 1;
            add_res_f_res_17463 = add_res_f_res_t_res_18871;
            add_res_f_res_17464 = add_res_f_res_t_res_18872;
        } else {
            if (memblock_alloc(ctx, &mem_19130, bytes_19129, "mem_19130")) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:31:44-69
            for (int64_t nest_i_19347 = 0; nest_i_19347 < negate_res_17375; nest_i_19347++) {
                ((int32_t *) mem_19130.mem)[nest_i_19347] = 0;
            }
            // stdlib/bigint.fut:31:23-35:39
            
            int64_t sub_abs_res_17493;
            int64_t borrow_17496 = (int64_t) 0;
            
            for (int64_t i_17494 = 0; i_17494 < negate_res_17375; i_17494++) {
                // stdlib/bigint.fut:32:21-25
                
                int32_t u32_arg0_17497 = ((int32_t *) ext_mem_19128.mem)[i_17494];
                
                // stdlib/bigint.fut:32:13-25
                
                int64_t u32_res_17498 = zext_i32_i64(u32_arg0_17497);
                
                // stdlib/bigint.fut:33:14-54
                
                bool cond_17499 = slt64(i_17494, arithmetic_res_17230);
                
                // stdlib/bigint.fut:33:14-54
                
                int64_t zp_lhs_17500;
                
                if (cond_17499) {
                    // stdlib/bigint.fut:33:43-47
                    
                    bool index_certs_18874;
                    
                    if (!cond_17499) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17494, "] out of bounds for array of shape [", (long long) arithmetic_res_17230, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:40:23-38\n   #2  stdlib/bigint.fut:42:15-63\n   #3  probes/stdlib-tests.fut:5:7-7:23\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:33:43-47
                    
                    int32_t u32_arg0_18875 = ((int32_t *) ext_mem_19088.mem)[i_17494];
                    
                    // stdlib/bigint.fut:33:35-47
                    
                    int64_t u32_res_18876 = zext_i32_i64(u32_arg0_18875);
                    
                    zp_lhs_17500 = u32_res_18876;
                } else {
                    zp_lhs_17500 = (int64_t) 0;
                }
                // stdlib/bigint.fut:33:55-62
                
                int64_t y_17504 = add64(borrow_17496, zp_lhs_17500);
                
                // stdlib/bigint.fut:34:28-30
                
                int64_t u64_arg0_17505 = sub64(u32_res_17498, y_17504);
                
                // stdlib/bigint.fut:34:18-30
                
                int32_t u64_res_17506 = zext_i64_i32(u64_arg0_17505);
                
                // stdlib/bigint.fut:34:9-31
                ((int32_t *) mem_19130.mem)[i_17494] = u64_res_17506;
                // stdlib/bigint.fut:35:13-38
                
                bool cond_17508 = ult64(u32_res_17498, y_17504);
                
                // stdlib/bigint.fut:40:23-38
                
                int64_t tmp_17509 = btoi_bool_i64(cond_17508);
                int64_t borrow_tmp_19349 = tmp_17509;
                
                borrow_17496 = borrow_tmp_19349;
            }
            sub_abs_res_17493 = borrow_17496;
            // stdlib/bigint.fut:36:7-26
            
            int64_t sub_abs_res_17510;
            bool sub_abs_res_17511;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19139, &sub_abs_res_17510, &sub_abs_res_17511, mem_19130, negate_res_17375, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &mem_19130, "mem_19130") != 0)
                return 1;
            // stdlib/bigint.fut:40:8-38
            
            int64_t add_res_f_res_f_res_17513;
            bool add_res_f_res_f_res_17514;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19140, &add_res_f_res_f_res_17513, &add_res_f_res_f_res_17514, ext_mem_19139, sub_abs_res_17510, negate_res_17376) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_19139, "ext_mem_19139") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19153, &ext_mem_19140, "ext_mem_19140") != 0)
                return 1;
            add_res_f_res_17463 = add_res_f_res_f_res_17513;
            add_res_f_res_17464 = add_res_f_res_f_res_17514;
        }
        if (memblock_set(ctx, &ext_mem_19166, &ext_mem_19153, "ext_mem_19153") != 0)
            return 1;
        add_res_17423 = add_res_f_res_17463;
        add_res_17424 = add_res_f_res_17464;
    }
    if (memblock_unref(ctx, &ext_mem_19128, "ext_mem_19128") != 0)
        return 1;
    // stdlib/bigint.fut:50:65-78
    
    int64_t normalise_arg1_17516;
    
    if (futrts_mul_abs_10600(ctx, &ext_mem_19167, &normalise_arg1_17516, ext_mem_19088, arithmetic_res_17230, factor_13925) != 0) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:50:50-78
    
    int64_t multiply_small_res_17518;
    bool multiply_small_res_17519;
    
    if (futrts_normalise_10583(ctx, &ext_mem_19168, &multiply_small_res_17518, &multiply_small_res_17519, ext_mem_19167, normalise_arg1_17516, arithmetic_res_17231) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &ext_mem_19167, "ext_mem_19167") != 0)
        return 1;
    // stdlib/bigint.fut:70:16-33
    
    int64_t quotient_res_17726;
    bool quotient_res_17727;
    
    if (futrts_normalise_10583(ctx, &ext_mem_19221, &quotient_res_17726, &quotient_res_17727, mem_19172, max_res_17546, 0) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &mem_19172, "mem_19172") != 0)
        return 1;
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool protect_cond_conj_17739 = x_17237 && cond_17733;
    
    // stdlib/bigint.fut:73:39-51
    
    int32_t zeze_lhs_17740;
    
    if (protect_cond_conj_17739) {
        // probes/stdlib-tests.fut:5:7-7:60
        
        int32_t x_18899 = ((int32_t *) ext_mem_19217.mem)[(int64_t) 0];
        
        zeze_lhs_17740 = x_18899;
    } else {
        zeze_lhs_17740 = 0;
    }
    if (memblock_unref(ctx, &ext_mem_19217, "ext_mem_19217") != 0)
        return 1;
    // stdlib/bigint.fut:73:52-56
    
    bool cond_t_res_17742 = zeze_lhs_17740 == 0;
    
    // probes/stdlib-tests.fut:5:7-7:60
    
    bool x_17743 = cond_17733 && cond_t_res_17742;
    
    if (memblock_alloc(ctx, &mem_19222, (int64_t) 8, "mem_19222")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:10:6-69
    
    struct memblock arithmeticzistatic_array_19350 = (struct memblock) {NULL, (unsigned char *) arithmeticzistatic_array_realtype_19379, 0, "arithmetic.static_array_19350"};
    
    // stdlib/bigint.fut:10:6-69
    lmad_copy_4b(ctx, 1, (uint32_t *) mem_19222.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (uint32_t *) arithmeticzistatic_array_19350.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {(int64_t) 2});
    // stdlib/bigint.fut:41:34-54
    
    bool normalise_arg_17745 = !quotient_res_17727;
    int64_t bytes_19236 = (int64_t) 4 * quotient_res_17726;
    
    // stdlib/bigint.fut:52:3-74:42
    
    int64_t quotient_res_17746;
    bool quotient_res_17747;
    
    if (cond_17236) {
        if (memblock_set(ctx, &ext_mem_19265, &ext_mem_19221, "ext_mem_19221") != 0)
            return 1;
        quotient_res_17746 = quotient_res_17726;
        quotient_res_17747 = quotient_res_17727;
    } else {
        // stdlib/bigint.fut:73:11-74:42
        
        int64_t quotient_res_f_res_17749;
        bool quotient_res_f_res_17750;
        
        if (x_17743) {
            // stdlib/bigint.fut:41:34-54
            
            int64_t negate_res_18900;
            bool negate_res_18901;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19263, &negate_res_18900, &negate_res_18901, ext_mem_19221, quotient_res_17726, normalise_arg_17745) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_19264, &ext_mem_19263, "ext_mem_19263") != 0)
                return 1;
            quotient_res_f_res_17749 = negate_res_18900;
            quotient_res_f_res_17750 = negate_res_18901;
        } else {
            // stdlib/bigint.fut:10:6-69
            
            int64_t integer_res_17755;
            bool integer_res_17756;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19223, &integer_res_17755, &integer_res_17756, mem_19222, (int64_t) 2, 0) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/bigint.fut:38:3-40:39
            
            bool cond_17758 = quotient_res_17727 == integer_res_17756;
            
            // stdlib/bigint.fut:70:7-74:40
            
            bool x_17759 = !cond_17758;
            
            // stdlib/bigint.fut:21:14-42
            
            int64_t max_res_17806 = smax64(quotient_res_17726, integer_res_17755);
            
            // stdlib/bigint.fut:22:52-58
            
            int64_t dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17807 = add64((int64_t) 1, max_res_17806);
            
            // stdlib/bigint.fut:28:7-32
            
            bool x_17832 = sle64((int64_t) 0, max_res_17806);
            
            // stdlib/bigint.fut:28:7-32
            
            bool y_17833 = slt64(max_res_17806, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17807);
            
            // stdlib/bigint.fut:28:7-32
            
            bool bounds_check_17834 = x_17832 && y_17833;
            bool protect_assert_disj_17950 = x_17759 || bounds_check_17834;
            
            // stdlib/bigint.fut:28:7-32
            
            bool index_certs_17835;
            
            if (!protect_assert_disj_17950) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) max_res_17806, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17807, "].", "-> #0  stdlib/bigint.fut:28:7-32\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:5:7-7:60\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/bigint.fut:39:11-30
            
            int64_t zgze_lhs_17760;
            
            if (x_17759) {
                // stdlib/bigint.fut:12:3-16:17
                
                bool cond_18926 = quotient_res_17726 == integer_res_17755;
                
                // stdlib/bigint.fut:12:3-16:17
                
                bool cond_18927 = !cond_18926;
                
                // stdlib/bigint.fut:13:53-55
                
                int64_t tmp_18928 = sub64(quotient_res_17726, (int64_t) 1);
                
                // stdlib/bigint.fut:13:26-15:74
                
                bool cond_18929 = sle64((int64_t) 0, tmp_18928);
                int64_t compare_abs_res_f_res_18930;
                
                if (cond_18926) {
                    bool x_18931;
                    int64_t x_18932;
                    int64_t x_18933;
                    bool loop_while_18934;
                    int64_t i_18935;
                    int64_t result_18936;
                    
                    loop_while_18934 = cond_18929;
                    i_18935 = tmp_18928;
                    result_18936 = (int64_t) 0;
                    while (loop_while_18934) {
                        // stdlib/bigint.fut:15:20-24
                        
                        bool x_18938 = sle64((int64_t) 0, i_18935);
                        
                        // stdlib/bigint.fut:15:20-24
                        
                        bool y_18939 = slt64(i_18935, quotient_res_17726);
                        
                        // stdlib/bigint.fut:15:20-24
                        
                        bool bounds_check_18940 = x_18938 && y_18939;
                        
                        // stdlib/bigint.fut:15:20-24
                        
                        bool index_certs_18941;
                        
                        if (!bounds_check_18940) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18935, "] out of bounds for array of shape [", (long long) quotient_res_17726, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:70:7-74:40\n   #2  probes/stdlib-tests.fut:5:7-7:60\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/bigint.fut:15:27-31
                        
                        bool y_18943 = slt64(i_18935, integer_res_17755);
                        
                        // stdlib/bigint.fut:15:27-31
                        
                        bool bounds_check_18944 = x_18938 && y_18943;
                        
                        // stdlib/bigint.fut:15:27-31
                        
                        bool index_certs_18945;
                        
                        if (!bounds_check_18944) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18935, "] out of bounds for array of shape [", (long long) integer_res_17755, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:70:7-74:40\n   #2  probes/stdlib-tests.fut:5:7-7:60\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/bigint.fut:15:14-16
                        
                        int64_t tmp_18937 = sub64(i_18935, (int64_t) 1);
                        
                        // stdlib/bigint.fut:15:20-24
                        
                        int32_t zl_lhs_18942 = ((int32_t *) ext_mem_19221.mem)[i_18935];
                        
                        // stdlib/bigint.fut:15:27-31
                        
                        int32_t zl_rhs_18946 = ((int32_t *) ext_mem_19223.mem)[i_18935];
                        
                        // stdlib/bigint.fut:15:17-73
                        
                        bool cond_18947 = ult32(zl_lhs_18942, zl_rhs_18946);
                        
                        // stdlib/bigint.fut:15:17-73
                        
                        int64_t tmp_18948;
                        
                        if (cond_18947) {
                            tmp_18948 = (int64_t) -1;
                        } else {
                            // stdlib/bigint.fut:15:45-73
                            
                            bool cond_18949 = ult32(zl_rhs_18946, zl_lhs_18942);
                            
                            // stdlib/bigint.fut:70:7-74:40
                            
                            int64_t tmp_f_res_18950 = btoi_bool_i64(cond_18949);
                            
                            tmp_18948 = tmp_f_res_18950;
                        }
                        // stdlib/bigint.fut:14:16-37
                        
                        bool cond_18951 = sle64((int64_t) 0, tmp_18937);
                        
                        // stdlib/bigint.fut:14:33-37
                        
                        bool loop_cond_t_res_18952 = tmp_18948 == (int64_t) 0;
                        
                        // stdlib/bigint.fut:70:7-74:40
                        
                        bool x_18953 = cond_18951 && loop_cond_t_res_18952;
                        bool loop_while_tmp_19351 = x_18953;
                        int64_t i_tmp_19352 = tmp_18937;
                        int64_t result_tmp_19353 = tmp_18948;
                        
                        loop_while_18934 = loop_while_tmp_19351;
                        i_18935 = i_tmp_19352;
                        result_18936 = result_tmp_19353;
                    }
                    x_18931 = loop_while_18934;
                    x_18932 = i_18935;
                    x_18933 = result_18936;
                    compare_abs_res_f_res_18930 = x_18933;
                } else {
                    compare_abs_res_f_res_18930 = (int64_t) 0;
                }
                // stdlib/bigint.fut:12:32-69
                
                bool cond_18954 = slt64(quotient_res_17726, integer_res_17755);
                
                // stdlib/bigint.fut:12:32-69
                
                int64_t compare_abs_res_t_res_18955;
                
                if (cond_18954) {
                    compare_abs_res_t_res_18955 = (int64_t) -1;
                } else {
                    compare_abs_res_t_res_18955 = (int64_t) 1;
                }
                // stdlib/bigint.fut:12:3-16:17
                
                int64_t compare_abs_res_18956;
                
                if (cond_18927) {
                    compare_abs_res_18956 = compare_abs_res_t_res_18955;
                } else {
                    compare_abs_res_18956 = compare_abs_res_f_res_18930;
                }
                zgze_lhs_17760 = compare_abs_res_18956;
            } else {
                zgze_lhs_17760 = (int64_t) 0;
            }
            // stdlib/bigint.fut:39:8-40:39
            
            bool cond_17802 = sle64((int64_t) 0, zgze_lhs_17760);
            int64_t bytes_19224 = (int64_t) 4 * integer_res_17755;
            int64_t bytes_19249 = (int64_t) 4 * dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17807;
            
            // stdlib/bigint.fut:38:3-40:39
            
            int64_t add_res_17803;
            bool add_res_17804;
            
            if (cond_17758) {
                if (memblock_alloc(ctx, &mem_19250, bytes_19249, "mem_19250")) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/bigint.fut:22:41-64
                for (int64_t nest_i_19354 = 0; nest_i_19354 < dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17807; nest_i_19354++) {
                    ((int32_t *) mem_19250.mem)[nest_i_19354] = 0;
                }
                // stdlib/bigint.fut:22:21-27:23
                
                int64_t add_abs_res_18965;
                int64_t carry_18968 = (int64_t) 0;
                
                for (int64_t i_18966 = 0; i_18966 < max_res_17806; i_18966++) {
                    // stdlib/bigint.fut:26:9-29
                    
                    bool y_18982 = slt64(i_18966, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17807);
                    
                    // stdlib/bigint.fut:26:9-29
                    
                    bool index_certs_18983;
                    
                    if (!y_18982) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18966, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17807, "].", "-> #0  stdlib/bigint.fut:26:9-29\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:5:7-7:60\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:23:13-53
                    
                    bool cond_18969 = slt64(i_18966, quotient_res_17726);
                    
                    // stdlib/bigint.fut:23:13-53
                    
                    int64_t x_18970;
                    
                    if (cond_18969) {
                        // stdlib/bigint.fut:23:42-46
                        
                        bool index_certs_18971;
                        
                        if (!cond_18969) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18966, "] out of bounds for array of shape [", (long long) quotient_res_17726, "].", "-> #0  stdlib/bigint.fut:23:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:5:7-7:60\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/bigint.fut:23:42-46
                        
                        int32_t u32_arg0_18972 = ((int32_t *) ext_mem_19221.mem)[i_18966];
                        
                        // stdlib/bigint.fut:23:34-46
                        
                        int64_t u32_res_18973 = zext_i32_i64(u32_arg0_18972);
                        
                        x_18970 = u32_res_18973;
                    } else {
                        x_18970 = (int64_t) 0;
                    }
                    // stdlib/bigint.fut:24:13-53
                    
                    bool cond_18974 = slt64(i_18966, integer_res_17755);
                    
                    // stdlib/bigint.fut:24:13-53
                    
                    int64_t y_18975;
                    
                    if (cond_18974) {
                        // stdlib/bigint.fut:24:42-46
                        
                        bool index_certs_18976;
                        
                        if (!cond_18974) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18966, "] out of bounds for array of shape [", (long long) integer_res_17755, "].", "-> #0  stdlib/bigint.fut:24:42-46\n   #1  stdlib/bigint.fut:38:37-52\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:5:7-7:60\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/bigint.fut:24:42-46
                        
                        int32_t u32_arg0_18977 = ((int32_t *) ext_mem_19223.mem)[i_18966];
                        
                        // stdlib/bigint.fut:24:34-46
                        
                        int64_t u32_res_18978 = zext_i32_i64(u32_arg0_18977);
                        
                        y_18975 = u32_res_18978;
                    } else {
                        y_18975 = (int64_t) 0;
                    }
                    // stdlib/bigint.fut:25:16-18
                    
                    int64_t zp_lhs_18979 = add64(x_18970, y_18975);
                    
                    // stdlib/bigint.fut:25:18-24
                    
                    int64_t sum_18980 = add64(carry_18968, zp_lhs_18979);
                    
                    // stdlib/bigint.fut:26:18-29
                    
                    int32_t u64_res_18981 = zext_i64_i32(sum_18980);
                    
                    // stdlib/bigint.fut:26:9-29
                    ((int32_t *) mem_19250.mem)[i_18966] = u64_res_18981;
                    // stdlib/bigint.fut:27:17-22
                    
                    int64_t tmp_18985 = lshr64(sum_18980, (int64_t) 32);
                    int64_t carry_tmp_19356 = tmp_18985;
                    
                    carry_18968 = carry_tmp_19356;
                }
                add_abs_res_18965 = carry_18968;
                // stdlib/bigint.fut:28:19-32
                
                int32_t u64_res_18986 = zext_i64_i32(add_abs_res_18965);
                
                // stdlib/bigint.fut:28:7-32
                ((int32_t *) mem_19250.mem)[max_res_17806] = u64_res_18986;
                // stdlib/bigint.fut:29:7-26
                
                int64_t add_abs_res_18988;
                bool add_abs_res_18989;
                
                if (futrts_normalise_10583(ctx, &ext_mem_19259, &add_abs_res_18988, &add_abs_res_18989, mem_19250, dzlz7bUZLzpZRz20Usizzez20U1z7dUzg_17807, 0) != 0) {
                    err = 1;
                    goto cleanup;
                }
                if (memblock_unref(ctx, &mem_19250, "mem_19250") != 0)
                    return 1;
                // stdlib/bigint.fut:38:22-52
                
                int64_t add_res_t_res_18991;
                bool add_res_t_res_18992;
                
                if (futrts_normalise_10583(ctx, &ext_mem_19260, &add_res_t_res_18991, &add_res_t_res_18992, ext_mem_19259, add_abs_res_18988, quotient_res_17727) != 0) {
                    err = 1;
                    goto cleanup;
                }
                if (memblock_unref(ctx, &ext_mem_19259, "ext_mem_19259") != 0)
                    return 1;
                if (memblock_set(ctx, &ext_mem_19261, &ext_mem_19260, "ext_mem_19260") != 0)
                    return 1;
                add_res_17803 = add_res_t_res_18991;
                add_res_17804 = add_res_t_res_18992;
            } else {
                // stdlib/bigint.fut:39:8-40:39
                
                int64_t add_res_f_res_17843;
                bool add_res_f_res_17844;
                
                if (cond_17802) {
                    if (memblock_alloc(ctx, &mem_19237, bytes_19236, "mem_19237")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:31:44-69
                    for (int64_t nest_i_19357 = 0; nest_i_19357 < quotient_res_17726; nest_i_19357++) {
                        ((int32_t *) mem_19237.mem)[nest_i_19357] = 0;
                    }
                    // stdlib/bigint.fut:31:23-35:39
                    
                    int64_t sub_abs_res_18999;
                    int64_t borrow_19002 = (int64_t) 0;
                    
                    for (int64_t i_19000 = 0; i_19000 < quotient_res_17726; i_19000++) {
                        // stdlib/bigint.fut:32:21-25
                        
                        int32_t u32_arg0_19003 = ((int32_t *) ext_mem_19221.mem)[i_19000];
                        
                        // stdlib/bigint.fut:32:13-25
                        
                        int64_t u32_res_19004 = zext_i32_i64(u32_arg0_19003);
                        
                        // stdlib/bigint.fut:33:14-54
                        
                        bool cond_19005 = slt64(i_19000, integer_res_17755);
                        
                        // stdlib/bigint.fut:33:14-54
                        
                        int64_t zp_lhs_19006;
                        
                        if (cond_19005) {
                            // stdlib/bigint.fut:33:43-47
                            
                            bool index_certs_19007;
                            
                            if (!cond_19005) {
                                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_19000, "] out of bounds for array of shape [", (long long) integer_res_17755, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:39:56-71\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:5:7-7:60\n"));
                                err = FUTHARK_PROGRAM_ERROR;
                                goto cleanup;
                            }
                            // stdlib/bigint.fut:33:43-47
                            
                            int32_t u32_arg0_19008 = ((int32_t *) ext_mem_19223.mem)[i_19000];
                            
                            // stdlib/bigint.fut:33:35-47
                            
                            int64_t u32_res_19009 = zext_i32_i64(u32_arg0_19008);
                            
                            zp_lhs_19006 = u32_res_19009;
                        } else {
                            zp_lhs_19006 = (int64_t) 0;
                        }
                        // stdlib/bigint.fut:33:55-62
                        
                        int64_t y_19010 = add64(borrow_19002, zp_lhs_19006);
                        
                        // stdlib/bigint.fut:34:28-30
                        
                        int64_t u64_arg0_19011 = sub64(u32_res_19004, y_19010);
                        
                        // stdlib/bigint.fut:34:18-30
                        
                        int32_t u64_res_19012 = zext_i64_i32(u64_arg0_19011);
                        
                        // stdlib/bigint.fut:34:9-31
                        ((int32_t *) mem_19237.mem)[i_19000] = u64_res_19012;
                        // stdlib/bigint.fut:35:13-38
                        
                        bool cond_19014 = ult64(u32_res_19004, y_19010);
                        
                        // stdlib/bigint.fut:39:56-71
                        
                        int64_t tmp_19015 = btoi_bool_i64(cond_19014);
                        int64_t borrow_tmp_19359 = tmp_19015;
                        
                        borrow_19002 = borrow_tmp_19359;
                    }
                    sub_abs_res_18999 = borrow_19002;
                    // stdlib/bigint.fut:36:7-26
                    
                    int64_t sub_abs_res_19016;
                    bool sub_abs_res_19017;
                    
                    if (futrts_normalise_10583(ctx, &ext_mem_19246, &sub_abs_res_19016, &sub_abs_res_19017, mem_19237, quotient_res_17726, 0) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &mem_19237, "mem_19237") != 0)
                        return 1;
                    // stdlib/bigint.fut:39:41-71
                    
                    int64_t add_res_f_res_t_res_19019;
                    bool add_res_f_res_t_res_19020;
                    
                    if (futrts_normalise_10583(ctx, &ext_mem_19247, &add_res_f_res_t_res_19019, &add_res_f_res_t_res_19020, ext_mem_19246, sub_abs_res_19016, quotient_res_17727) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &ext_mem_19246, "ext_mem_19246") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_19248, &ext_mem_19247, "ext_mem_19247") != 0)
                        return 1;
                    add_res_f_res_17843 = add_res_f_res_t_res_19019;
                    add_res_f_res_17844 = add_res_f_res_t_res_19020;
                } else {
                    if (memblock_alloc(ctx, &mem_19225, bytes_19224, "mem_19225")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/bigint.fut:31:44-69
                    for (int64_t nest_i_19360 = 0; nest_i_19360 < integer_res_17755; nest_i_19360++) {
                        ((int32_t *) mem_19225.mem)[nest_i_19360] = 0;
                    }
                    // stdlib/bigint.fut:31:23-35:39
                    
                    int64_t sub_abs_res_17873;
                    int64_t borrow_17876 = (int64_t) 0;
                    
                    for (int64_t i_17874 = 0; i_17874 < integer_res_17755; i_17874++) {
                        // stdlib/bigint.fut:32:21-25
                        
                        int32_t u32_arg0_17877 = ((int32_t *) ext_mem_19223.mem)[i_17874];
                        
                        // stdlib/bigint.fut:32:13-25
                        
                        int64_t u32_res_17878 = zext_i32_i64(u32_arg0_17877);
                        
                        // stdlib/bigint.fut:33:14-54
                        
                        bool cond_17879 = slt64(i_17874, quotient_res_17726);
                        
                        // stdlib/bigint.fut:33:14-54
                        
                        int64_t zp_lhs_17880;
                        
                        if (cond_17879) {
                            // stdlib/bigint.fut:33:43-47
                            
                            bool index_certs_19022;
                            
                            if (!cond_17879) {
                                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17874, "] out of bounds for array of shape [", (long long) quotient_res_17726, "].", "-> #0  stdlib/bigint.fut:33:43-47\n   #1  stdlib/bigint.fut:40:23-38\n   #2  stdlib/bigint.fut:70:7-74:40\n   #3  probes/stdlib-tests.fut:5:7-7:60\n"));
                                err = FUTHARK_PROGRAM_ERROR;
                                goto cleanup;
                            }
                            // stdlib/bigint.fut:33:43-47
                            
                            int32_t u32_arg0_19023 = ((int32_t *) ext_mem_19221.mem)[i_17874];
                            
                            // stdlib/bigint.fut:33:35-47
                            
                            int64_t u32_res_19024 = zext_i32_i64(u32_arg0_19023);
                            
                            zp_lhs_17880 = u32_res_19024;
                        } else {
                            zp_lhs_17880 = (int64_t) 0;
                        }
                        // stdlib/bigint.fut:33:55-62
                        
                        int64_t y_17884 = add64(borrow_17876, zp_lhs_17880);
                        
                        // stdlib/bigint.fut:34:28-30
                        
                        int64_t u64_arg0_17885 = sub64(u32_res_17878, y_17884);
                        
                        // stdlib/bigint.fut:34:18-30
                        
                        int32_t u64_res_17886 = zext_i64_i32(u64_arg0_17885);
                        
                        // stdlib/bigint.fut:34:9-31
                        ((int32_t *) mem_19225.mem)[i_17874] = u64_res_17886;
                        // stdlib/bigint.fut:35:13-38
                        
                        bool cond_17888 = ult64(u32_res_17878, y_17884);
                        
                        // stdlib/bigint.fut:40:23-38
                        
                        int64_t tmp_17889 = btoi_bool_i64(cond_17888);
                        int64_t borrow_tmp_19362 = tmp_17889;
                        
                        borrow_17876 = borrow_tmp_19362;
                    }
                    sub_abs_res_17873 = borrow_17876;
                    // stdlib/bigint.fut:36:7-26
                    
                    int64_t sub_abs_res_17890;
                    bool sub_abs_res_17891;
                    
                    if (futrts_normalise_10583(ctx, &ext_mem_19234, &sub_abs_res_17890, &sub_abs_res_17891, mem_19225, integer_res_17755, 0) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &mem_19225, "mem_19225") != 0)
                        return 1;
                    // stdlib/bigint.fut:40:8-38
                    
                    int64_t add_res_f_res_f_res_17893;
                    bool add_res_f_res_f_res_17894;
                    
                    if (futrts_normalise_10583(ctx, &ext_mem_19235, &add_res_f_res_f_res_17893, &add_res_f_res_f_res_17894, ext_mem_19234, sub_abs_res_17890, integer_res_17756) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &ext_mem_19234, "ext_mem_19234") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_19248, &ext_mem_19235, "ext_mem_19235") != 0)
                        return 1;
                    add_res_f_res_17843 = add_res_f_res_f_res_17893;
                    add_res_f_res_17844 = add_res_f_res_f_res_17894;
                }
                if (memblock_set(ctx, &ext_mem_19261, &ext_mem_19248, "ext_mem_19248") != 0)
                    return 1;
                add_res_17803 = add_res_f_res_17843;
                add_res_17804 = add_res_f_res_17844;
            }
            if (memblock_unref(ctx, &ext_mem_19223, "ext_mem_19223") != 0)
                return 1;
            // stdlib/bigint.fut:41:34-54
            
            bool normalise_arg_17896 = !add_res_17804;
            
            // stdlib/bigint.fut:41:34-54
            
            int64_t negate_res_17897;
            bool negate_res_17898;
            
            if (futrts_normalise_10583(ctx, &ext_mem_19262, &negate_res_17897, &negate_res_17898, ext_mem_19261, add_res_17803, normalise_arg_17896) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_unref(ctx, &ext_mem_19261, "ext_mem_19261") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19264, &ext_mem_19262, "ext_mem_19262") != 0)
                return 1;
            quotient_res_f_res_17749 = negate_res_17897;
            quotient_res_f_res_17750 = negate_res_17898;
        }
        if (memblock_set(ctx, &ext_mem_19265, &ext_mem_19264, "ext_mem_19264") != 0)
            return 1;
        quotient_res_17746 = quotient_res_f_res_17749;
        quotient_res_17747 = quotient_res_f_res_17750;
    }
    if (memblock_unref(ctx, &ext_mem_19221, "ext_mem_19221") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_19222, "mem_19222") != 0)
        return 1;
    // stdlib/bigint.fut:18:3-19:51
    
    int64_t compare_res_17902;
    
    if (x_17237) {
        // stdlib/bigint.fut:18:22-43
        
        int64_t compare_res_t_res_19025;
        
        if (arithmetic_res_17231) {
            compare_res_t_res_19025 = (int64_t) -1;
        } else {
            compare_res_t_res_19025 = (int64_t) 1;
        }
        compare_res_17902 = compare_res_t_res_19025;
    } else {
        // stdlib/bigint.fut:19:9-30
        
        int64_t zt_lhs_17904;
        
        if (arithmetic_res_17231) {
            zt_lhs_17904 = (int64_t) -1;
        } else {
            zt_lhs_17904 = (int64_t) 1;
        }
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_17905 = arithmetic_res_17230 == arithmetic_res_17233;
        
        // stdlib/bigint.fut:12:3-16:17
        
        bool cond_17906 = !cond_17905;
        
        // stdlib/bigint.fut:12:3-16:17
        
        int64_t compare_abs_res_17907;
        
        if (cond_17906) {
            // stdlib/bigint.fut:12:32-69
            
            bool cond_19026 = slt64(arithmetic_res_17230, arithmetic_res_17233);
            
            // stdlib/bigint.fut:12:32-69
            
            int64_t compare_abs_res_t_res_19027;
            
            if (cond_19026) {
                compare_abs_res_t_res_19027 = (int64_t) -1;
            } else {
                compare_abs_res_t_res_19027 = (int64_t) 1;
            }
            compare_abs_res_17907 = compare_abs_res_t_res_19027;
        } else {
            // stdlib/bigint.fut:13:53-55
            
            int64_t tmp_17910 = sub64(arithmetic_res_17230, (int64_t) 1);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool cond_17911 = sle64((int64_t) 0, tmp_17910);
            
            // stdlib/bigint.fut:13:26-15:74
            
            bool compare_abs_res_f_res_17912;
            int64_t compare_abs_res_f_res_17913;
            int64_t compare_abs_res_f_res_17914;
            bool loop_while_17915;
            int64_t i_17916;
            int64_t result_17917;
            
            loop_while_17915 = cond_17911;
            i_17916 = tmp_17910;
            result_17917 = (int64_t) 0;
            while (loop_while_17915) {
                // stdlib/bigint.fut:15:20-24
                
                bool x_17919 = sle64((int64_t) 0, i_17916);
                
                // stdlib/bigint.fut:15:20-24
                
                bool y_17920 = slt64(i_17916, arithmetic_res_17230);
                
                // stdlib/bigint.fut:15:20-24
                
                bool bounds_check_17921 = x_17919 && y_17920;
                
                // stdlib/bigint.fut:15:20-24
                
                bool index_certs_17922;
                
                if (!bounds_check_17921) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17916, "] out of bounds for array of shape [", (long long) arithmetic_res_17230, "].", "-> #0  stdlib/bigint.fut:15:20-24\n   #1  stdlib/bigint.fut:19:32-51\n   #2  probes/stdlib-tests.fut:5:7-7:72\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:27-31
                
                bool y_17925 = slt64(i_17916, arithmetic_res_17233);
                
                // stdlib/bigint.fut:15:27-31
                
                bool bounds_check_17926 = x_17919 && y_17925;
                
                // stdlib/bigint.fut:15:27-31
                
                bool index_certs_17927;
                
                if (!bounds_check_17926) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_17916, "] out of bounds for array of shape [", (long long) arithmetic_res_17233, "].", "-> #0  stdlib/bigint.fut:15:27-31\n   #1  stdlib/bigint.fut:19:32-51\n   #2  probes/stdlib-tests.fut:5:7-7:72\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/bigint.fut:15:14-16
                
                int64_t tmp_17918 = sub64(i_17916, (int64_t) 1);
                
                // stdlib/bigint.fut:15:20-24
                
                int32_t zl_lhs_17923 = ((int32_t *) ext_mem_19088.mem)[i_17916];
                
                // stdlib/bigint.fut:15:27-31
                
                int32_t zl_rhs_17928 = ((int32_t *) ext_mem_19089.mem)[i_17916];
                
                // stdlib/bigint.fut:15:17-73
                
                bool cond_17929 = ult32(zl_lhs_17923, zl_rhs_17928);
                
                // stdlib/bigint.fut:15:17-73
                
                int64_t tmp_17930;
                
                if (cond_17929) {
                    tmp_17930 = (int64_t) -1;
                } else {
                    // stdlib/bigint.fut:15:45-73
                    
                    bool cond_17941 = ult32(zl_rhs_17928, zl_lhs_17923);
                    
                    // stdlib/bigint.fut:19:32-51
                    
                    int64_t tmp_f_res_17942 = btoi_bool_i64(cond_17941);
                    
                    tmp_17930 = tmp_f_res_17942;
                }
                // stdlib/bigint.fut:14:16-37
                
                bool cond_17943 = sle64((int64_t) 0, tmp_17918);
                
                // stdlib/bigint.fut:14:33-37
                
                bool loop_cond_t_res_17944 = tmp_17930 == (int64_t) 0;
                
                // stdlib/bigint.fut:19:32-51
                
                bool x_17945 = cond_17943 && loop_cond_t_res_17944;
                bool loop_while_tmp_19363 = x_17945;
                int64_t i_tmp_19364 = tmp_17918;
                int64_t result_tmp_19365 = tmp_17930;
                
                loop_while_17915 = loop_while_tmp_19363;
                i_17916 = i_tmp_19364;
                result_17917 = result_tmp_19365;
            }
            compare_abs_res_f_res_17912 = loop_while_17915;
            compare_abs_res_f_res_17913 = i_17916;
            compare_abs_res_f_res_17914 = result_17917;
            compare_abs_res_17907 = compare_abs_res_f_res_17914;
        }
        // stdlib/bigint.fut:19:31-51
        
        int64_t compare_res_f_res_17946 = mul64(zt_lhs_17904, compare_abs_res_17907);
        
        compare_res_17902 = compare_res_f_res_17946;
    }
    if (memblock_unref(ctx, &ext_mem_19088, "ext_mem_19088") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_19089, "ext_mem_19089") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19289, &ext_mem_19127, "ext_mem_19127") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19290, &ext_mem_19166, "ext_mem_19166") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19291, &ext_mem_19168, "ext_mem_19168") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19292, &ext_mem_19265, "ext_mem_19265") != 0)
        return 1;
    prim_out_19293 = add_res_17281;
    prim_out_19294 = add_res_17423;
    prim_out_19295 = multiply_small_res_17518;
    prim_out_19296 = quotient_res_17746;
    prim_out_19297 = add_res_17282;
    prim_out_19298 = add_res_17424;
    prim_out_19299 = multiply_small_res_17519;
    prim_out_19300 = quotient_res_17747;
    prim_out_19301 = compare_res_17902;
    if (memblock_set(ctx, &*mem_out_p_19366, &mem_out_19289, "mem_out_19289") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19367, &mem_out_19290, "mem_out_19290") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19368, &mem_out_19291, "mem_out_19291") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19369, &mem_out_19292, "mem_out_19292") != 0)
        return 1;
    *out_prim_out_19370 = prim_out_19293;
    *out_prim_out_19371 = prim_out_19294;
    *out_prim_out_19372 = prim_out_19295;
    *out_prim_out_19373 = prim_out_19296;
    *out_prim_out_19374 = prim_out_19297;
    *out_prim_out_19375 = prim_out_19298;
    *out_prim_out_19376 = prim_out_19299;
    *out_prim_out_19377 = prim_out_19300;
    *out_prim_out_19378 = prim_out_19301;
    
  cleanup:
    {
        if (memblock_unref(ctx, &ext_mem_19262, "ext_mem_19262") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19235, "ext_mem_19235") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19234, "ext_mem_19234") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19225, "mem_19225") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19247, "ext_mem_19247") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19246, "ext_mem_19246") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19237, "mem_19237") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19248, "ext_mem_19248") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19260, "ext_mem_19260") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19259, "ext_mem_19259") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19250, "mem_19250") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19261, "ext_mem_19261") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19223, "ext_mem_19223") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19263, "ext_mem_19263") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19264, "ext_mem_19264") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19265, "ext_mem_19265") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19222, "mem_19222") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19221, "ext_mem_19221") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19168, "ext_mem_19168") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19167, "ext_mem_19167") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19140, "ext_mem_19140") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19139, "ext_mem_19139") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19130, "mem_19130") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19152, "ext_mem_19152") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19151, "ext_mem_19151") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19142, "mem_19142") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19153, "ext_mem_19153") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19165, "ext_mem_19165") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19164, "ext_mem_19164") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19155, "mem_19155") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19166, "ext_mem_19166") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19101, "ext_mem_19101") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19100, "ext_mem_19100") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19091, "mem_19091") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19113, "ext_mem_19113") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19112, "ext_mem_19112") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19103, "mem_19103") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19114, "ext_mem_19114") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19126, "ext_mem_19126") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19125, "ext_mem_19125") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19116, "mem_19116") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19127, "ext_mem_19127") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19303, "mem_param_tmp_19303") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19212, "ext_mem_19212") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19203, "mem_19203") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19312, "mem_param_tmp_19312") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19198, "ext_mem_19198") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19189, "mem_19189") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19187, "mem_param_19187") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19201, "ext_mem_19201") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19184, "ext_mem_19184") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19183, "mem_19183") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19181, "ext_mem_19181") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19178, "mem_param_19178") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19217, "ext_mem_19217") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19172, "mem_19172") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19170, "ext_mem_19170") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19169, "ext_mem_19169") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19128, "ext_mem_19128") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19089, "ext_mem_19089") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19088, "ext_mem_19088") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19292, "mem_out_19292") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19291, "mem_out_19291") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19290, "mem_out_19290") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19289, "mem_out_19289") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_hashmap_test(struct futhark_context *ctx, struct memblock *mem_out_p_19380, struct memblock keys_mem_19086, int64_t dz2080U_16031)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_19136;
    
    mem_19136.references = NULL;
    
    struct memblock mem_param_tmp_19295;
    
    mem_param_tmp_19295.references = NULL;
    
    struct memblock mem_param_tmp_19294;
    
    mem_param_tmp_19294.references = NULL;
    
    struct memblock mem_param_tmp_19293;
    
    mem_param_tmp_19293.references = NULL;
    
    struct memblock ext_mem_19123;
    
    ext_mem_19123.references = NULL;
    
    struct memblock ext_mem_19124;
    
    ext_mem_19124.references = NULL;
    
    struct memblock ext_mem_19125;
    
    ext_mem_19125.references = NULL;
    
    struct memblock ext_mem_19105;
    
    ext_mem_19105.references = NULL;
    
    struct memblock ext_mem_19106;
    
    ext_mem_19106.references = NULL;
    
    struct memblock ext_mem_19107;
    
    ext_mem_19107.references = NULL;
    
    struct memblock ext_mem_19110;
    
    ext_mem_19110.references = NULL;
    
    struct memblock ext_mem_19113;
    
    ext_mem_19113.references = NULL;
    
    struct memblock ext_mem_19116;
    
    ext_mem_19116.references = NULL;
    
    struct memblock mem_param_19098;
    
    mem_param_19098.references = NULL;
    
    struct memblock mem_param_19095;
    
    mem_param_19095.references = NULL;
    
    struct memblock mem_param_19092;
    
    mem_param_19092.references = NULL;
    
    struct memblock ext_mem_19132;
    
    ext_mem_19132.references = NULL;
    
    struct memblock ext_mem_19133;
    
    ext_mem_19133.references = NULL;
    
    struct memblock ext_mem_19134;
    
    ext_mem_19134.references = NULL;
    
    struct memblock mem_19089;
    
    mem_19089.references = NULL;
    
    struct memblock mem_19088;
    
    mem_19088.references = NULL;
    
    struct memblock mem_19087;
    
    mem_19087.references = NULL;
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    int64_t bytes_19135 = (int64_t) 8 * dz2080U_16031;
    
    if (memblock_alloc(ctx, &mem_19087, (int64_t) 16, "mem_19087")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:4:4-40
    for (int64_t nest_i_19290 = 0; nest_i_19290 < (int64_t) 2; nest_i_19290++) {
        ((int64_t *) mem_19087.mem)[nest_i_19290] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_19088, (int64_t) 16, "mem_19088")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:4:4-40
    for (int64_t nest_i_19291 = 0; nest_i_19291 < (int64_t) 2; nest_i_19291++) {
        ((int64_t *) mem_19088.mem)[nest_i_19291] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_19089, (int64_t) 2, "mem_19089")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:4:4-40
    for (int64_t nest_i_19292 = 0; nest_i_19292 < (int64_t) 2; nest_i_19292++) {
        ((bool *) mem_19089.mem)[nest_i_19292] = 0;
    }
    // probes/stdlib-tests.fut:12:15-14:31
    
    int64_t hashmap_test_res_17228;
    int64_t hashmap_test_res_17232;
    int64_t loop_dz2081U_17234;
    int64_t table_17238;
    
    if (memblock_set(ctx, &mem_param_19092, &mem_19087, "mem_19087") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_19095, &mem_19088, "mem_19088") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_19098, &mem_19089, "mem_19089") != 0)
        return 1;
    loop_dz2081U_17234 = (int64_t) 2;
    table_17238 = (int64_t) 0;
    for (int64_t i_17233 = 0; i_17233 < dz2080U_16031; i_17233++) {
        int64_t key_17239 = ((int64_t *) keys_mem_19086.mem)[i_17233];
        
        // stdlib/hashmap.fut:10:26-28
        
        int64_t mask_18588 = sub64(loop_dz2081U_17234, (int64_t) 1);
        
        // stdlib/hashmap.fut:6:19-24
        
        int64_t zc_rhs_18589 = lshr64(key_17239, (int64_t) 30);
        
        // stdlib/hashmap.fut:6:14-24
        
        int64_t zt_lhs_18590 = key_17239 ^ zc_rhs_18589;
        
        // stdlib/hashmap.fut:6:26-48
        
        int64_t x_18591 = mul64((int64_t) -4658895280553007687, zt_lhs_18590);
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_18592 = lshr64(x_18591, (int64_t) 27);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_18593 = x_18591 ^ zc_rhs_18592;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_18594 = mul64((int64_t) -7723592293110705685, zt_lhs_18593);
        
        // stdlib/hashmap.fut:8:13-18
        
        int64_t zc_rhs_18595 = lshr64(x_18594, (int64_t) 31);
        
        // stdlib/hashmap.fut:8:8-18
        
        int64_t hash_res_18596 = x_18594 ^ zc_rhs_18595;
        
        // stdlib/hashmap.fut:11:33-47
        
        int64_t u64_arg0_18597 = mask_18588 & hash_res_18596;
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool x_18598 = sle64((int64_t) 0, u64_arg0_18597);
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool y_18599 = slt64(u64_arg0_18597, loop_dz2081U_17234);
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool bounds_check_18600 = x_18598 && y_18599;
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool index_certs_18601;
        
        if (!bounds_check_18600) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u64_arg0_18597, "] out of bounds for array of shape [", (long long) loop_dz2081U_17234, "].", "-> #0  stdlib/hashmap.fut:11:6-12:57\n   #1  stdlib/hashmap.fut:15:11-25\n   #2  probes/stdlib-tests.fut:13:17-30\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:11:6-12:57
        
        bool loop_cond_18602 = ((bool *) mem_param_19098.mem)[u64_arg0_18597];
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool loop_cond_18603;
        
        if (loop_cond_18602) {
            // stdlib/hashmap.fut:12:26-34
            
            int64_t loop_cond_t_res_18677 = ((int64_t *) mem_param_19092.mem)[u64_arg0_18597];
            
            // stdlib/hashmap.fut:12:37-43
            
            bool loop_cond_t_res_18678 = loop_cond_t_res_18677 == key_17239;
            
            // stdlib/hashmap.fut:12:37-43
            
            bool loop_cond_t_res_18679 = !loop_cond_t_res_18678;
            
            loop_cond_18603 = loop_cond_t_res_18679;
        } else {
            loop_cond_18603 = 0;
        }
        // stdlib/hashmap.fut:11:6-12:57
        
        bool slot_res_18611;
        int64_t slot_res_18612;
        bool loop_while_18613;
        int64_t i_18614;
        
        loop_while_18613 = loop_cond_18603;
        i_18614 = u64_arg0_18597;
        while (loop_while_18613) {
            // stdlib/hashmap.fut:12:49-51
            
            int64_t za_lhs_18615 = add64((int64_t) 1, i_18614);
            
            // stdlib/hashmap.fut:12:52-57
            
            int64_t loopres_18616 = mask_18588 & za_lhs_18615;
            
            // stdlib/hashmap.fut:12:12-20
            
            bool x_18617 = sle64((int64_t) 0, loopres_18616);
            
            // stdlib/hashmap.fut:12:12-20
            
            bool y_18618 = slt64(loopres_18616, loop_dz2081U_17234);
            
            // stdlib/hashmap.fut:12:12-20
            
            bool bounds_check_18619 = x_18617 && y_18618;
            
            // stdlib/hashmap.fut:12:12-20
            
            bool index_certs_18620;
            
            if (!bounds_check_18619) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_18616, "] out of bounds for array of shape [", (long long) loop_dz2081U_17234, "].", "-> #0  stdlib/hashmap.fut:12:12-20\n   #1  stdlib/hashmap.fut:15:11-25\n   #2  probes/stdlib-tests.fut:13:17-30\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/hashmap.fut:12:12-20
            
            bool loop_cond_18621 = ((bool *) mem_param_19098.mem)[loopres_18616];
            
            // stdlib/hashmap.fut:12:12-43
            
            bool loop_cond_18622;
            
            if (loop_cond_18621) {
                // stdlib/hashmap.fut:12:26-34
                
                int64_t loop_cond_t_res_18680 = ((int64_t *) mem_param_19092.mem)[loopres_18616];
                
                // stdlib/hashmap.fut:12:37-43
                
                bool loop_cond_t_res_18681 = loop_cond_t_res_18680 == key_17239;
                
                // stdlib/hashmap.fut:12:37-43
                
                bool loop_cond_t_res_18682 = !loop_cond_t_res_18681;
                
                loop_cond_18622 = loop_cond_t_res_18682;
            } else {
                loop_cond_18622 = 0;
            }
            
            bool loop_while_tmp_19301 = loop_cond_18622;
            int64_t i_tmp_19302 = loopres_18616;
            
            loop_while_18613 = loop_while_tmp_19301;
            i_18614 = i_tmp_19302;
        }
        slot_res_18611 = loop_while_18613;
        slot_res_18612 = i_18614;
        // stdlib/hashmap.fut:16:9-17
        
        bool x_17241 = sle64((int64_t) 0, slot_res_18612);
        
        // stdlib/hashmap.fut:16:9-17
        
        bool y_17242 = slt64(slot_res_18612, loop_dz2081U_17234);
        
        // stdlib/hashmap.fut:16:9-17
        
        bool bounds_check_17243 = x_17241 && y_17242;
        
        // stdlib/hashmap.fut:16:9-17
        
        bool index_certs_17244;
        
        if (!bounds_check_17243) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_18612, "] out of bounds for array of shape [", (long long) loop_dz2081U_17234, "].", "-> #0  stdlib/hashmap.fut:16:9-17\n   #1  probes/stdlib-tests.fut:13:17-30\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:16:9-17
        
        bool get_res_17245 = ((bool *) mem_param_19098.mem)[slot_res_18612];
        
        // stdlib/hashmap.fut:16:6-42
        
        int64_t get_res_17246;
        
        if (get_res_17245) {
            // stdlib/hashmap.fut:16:25-33
            
            int64_t get_res_t_res_18683 = ((int64_t *) mem_param_19095.mem)[slot_res_18612];
            
            get_res_17246 = get_res_t_res_18683;
        } else {
            get_res_17246 = (int64_t) 0;
        }
        // probes/stdlib-tests.fut:14:28-30
        
        int64_t put_arg2_17252 = add64((int64_t) 1, get_res_17246);
        
        // stdlib/hashmap.fut:28:18-20
        
        int64_t zt_lhs_17253 = add64((int64_t) 1, table_17238);
        
        // stdlib/hashmap.fut:28:21-24
        
        int64_t zg_lhs_17254 = mul64((int64_t) 10, zt_lhs_17253);
        
        // stdlib/hashmap.fut:28:37-39
        
        int64_t zg_rhs_17255 = mul64((int64_t) 7, loop_dz2081U_17234);
        
        // stdlib/hashmap.fut:28:11-70
        
        bool cond_17256 = slt64(zg_rhs_17255, zg_lhs_17254);
        int64_t put_res_17257;
        int64_t put_res_17261;
        
        if (cond_17256) {
            // stdlib/hashmap.fut:27:10-28:61
            
            int64_t put_res_t_res_18684;
            int64_t put_res_t_res_18688;
            
            if (futrts_grow_10668(ctx, &ext_mem_19107, &ext_mem_19106, &ext_mem_19105, &put_res_t_res_18684, &put_res_t_res_18688, mem_param_19092, mem_param_19095, mem_param_19098, loop_dz2081U_17234, table_17238) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_19116, &ext_mem_19107, "ext_mem_19107") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19113, &ext_mem_19106, "ext_mem_19106") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19110, &ext_mem_19105, "ext_mem_19105") != 0)
                return 1;
            put_res_17257 = put_res_t_res_18684;
            put_res_17261 = put_res_t_res_18688;
        } else {
            if (memblock_set(ctx, &ext_mem_19116, &mem_param_19092, "mem_param_19092") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19113, &mem_param_19095, "mem_param_19095") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19110, &mem_param_19098, "mem_param_19098") != 0)
                return 1;
            put_res_17257 = loop_dz2081U_17234;
            put_res_17261 = table_17238;
        }
        // stdlib/hashmap.fut:28:7-29:25
        
        int64_t put_res_17270;
        
        if (futrts_put_raw_10670(ctx, &ext_mem_19125, &ext_mem_19124, &ext_mem_19123, &put_res_17270, ext_mem_19116, ext_mem_19113, ext_mem_19110, put_res_17257, put_res_17261, key_17239, put_arg2_17252) != 0) {
            err = 1;
            goto cleanup;
        }
        if (memblock_unref(ctx, &ext_mem_19110, "ext_mem_19110") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19113, "ext_mem_19113") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19116, "ext_mem_19116") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_19293, &ext_mem_19125, "ext_mem_19125") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_19294, &ext_mem_19124, "ext_mem_19124") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_19295, &ext_mem_19123, "ext_mem_19123") != 0)
            return 1;
        
        int64_t loop_dz2081U_tmp_19296 = put_res_17257;
        int64_t table_tmp_19300 = put_res_17270;
        
        if (memblock_set(ctx, &mem_param_19092, &mem_param_tmp_19293, "mem_param_tmp_19293") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_19095, &mem_param_tmp_19294, "mem_param_tmp_19294") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_19098, &mem_param_tmp_19295, "mem_param_tmp_19295") != 0)
            return 1;
        loop_dz2081U_17234 = loop_dz2081U_tmp_19296;
        table_17238 = table_tmp_19300;
    }
    if (memblock_set(ctx, &ext_mem_19134, &mem_param_19092, "mem_param_19092") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_19133, &mem_param_19095, "mem_param_19095") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_19132, &mem_param_19098, "mem_param_19098") != 0)
        return 1;
    hashmap_test_res_17228 = loop_dz2081U_17234;
    hashmap_test_res_17232 = table_17238;
    if (memblock_unref(ctx, &mem_19087, "mem_19087") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_19088, "mem_19088") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_19089, "mem_19089") != 0)
        return 1;
    // stdlib/hashmap.fut:10:26-28
    
    int64_t mask_18635 = sub64(hashmap_test_res_17228, (int64_t) 1);
    
    if (memblock_alloc(ctx, &mem_19136, bytes_19135, "mem_19136")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:15:6-26
    for (int64_t i_19072 = 0; i_19072 < dz2080U_16031; i_19072++) {
        int64_t eta_p_17272 = ((int64_t *) keys_mem_19086.mem)[i_19072];
        
        // stdlib/hashmap.fut:6:19-24
        
        int64_t zc_rhs_18636 = lshr64(eta_p_17272, (int64_t) 30);
        
        // stdlib/hashmap.fut:6:14-24
        
        int64_t zt_lhs_18637 = eta_p_17272 ^ zc_rhs_18636;
        
        // stdlib/hashmap.fut:6:26-48
        
        int64_t x_18638 = mul64((int64_t) -4658895280553007687, zt_lhs_18637);
        
        // stdlib/hashmap.fut:7:19-24
        
        int64_t zc_rhs_18639 = lshr64(x_18638, (int64_t) 27);
        
        // stdlib/hashmap.fut:7:14-24
        
        int64_t zt_lhs_18640 = x_18638 ^ zc_rhs_18639;
        
        // stdlib/hashmap.fut:7:26-48
        
        int64_t x_18641 = mul64((int64_t) -7723592293110705685, zt_lhs_18640);
        
        // stdlib/hashmap.fut:8:13-18
        
        int64_t zc_rhs_18642 = lshr64(x_18641, (int64_t) 31);
        
        // stdlib/hashmap.fut:8:8-18
        
        int64_t hash_res_18643 = x_18641 ^ zc_rhs_18642;
        
        // stdlib/hashmap.fut:11:33-47
        
        int64_t u64_arg0_18644 = mask_18635 & hash_res_18643;
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool x_18645 = sle64((int64_t) 0, u64_arg0_18644);
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool y_18646 = slt64(u64_arg0_18644, hashmap_test_res_17228);
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool bounds_check_18647 = x_18645 && y_18646;
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool index_certs_18648;
        
        if (!bounds_check_18647) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u64_arg0_18644, "] out of bounds for array of shape [", (long long) hashmap_test_res_17228, "].", "-> #0  stdlib/hashmap.fut:11:6-12:57\n   #1  stdlib/hashmap.fut:15:11-25\n   #2  probes/stdlib-tests.fut:15:11-14\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:11:6-12:57
        
        bool loop_cond_18649 = ((bool *) ext_mem_19132.mem)[u64_arg0_18644];
        
        // stdlib/hashmap.fut:11:6-12:57
        
        bool loop_cond_18650;
        
        if (loop_cond_18649) {
            // stdlib/hashmap.fut:12:26-34
            
            int64_t loop_cond_t_res_18689 = ((int64_t *) ext_mem_19134.mem)[u64_arg0_18644];
            
            // stdlib/hashmap.fut:12:37-43
            
            bool loop_cond_t_res_18690 = loop_cond_t_res_18689 == eta_p_17272;
            
            // stdlib/hashmap.fut:12:37-43
            
            bool loop_cond_t_res_18691 = !loop_cond_t_res_18690;
            
            loop_cond_18650 = loop_cond_t_res_18691;
        } else {
            loop_cond_18650 = 0;
        }
        // stdlib/hashmap.fut:11:6-12:57
        
        bool slot_res_18658;
        int64_t slot_res_18659;
        bool loop_while_18660;
        int64_t i_18661;
        
        loop_while_18660 = loop_cond_18650;
        i_18661 = u64_arg0_18644;
        while (loop_while_18660) {
            // stdlib/hashmap.fut:12:49-51
            
            int64_t za_lhs_18662 = add64((int64_t) 1, i_18661);
            
            // stdlib/hashmap.fut:12:52-57
            
            int64_t loopres_18663 = mask_18635 & za_lhs_18662;
            
            // stdlib/hashmap.fut:12:12-20
            
            bool x_18664 = sle64((int64_t) 0, loopres_18663);
            
            // stdlib/hashmap.fut:12:12-20
            
            bool y_18665 = slt64(loopres_18663, hashmap_test_res_17228);
            
            // stdlib/hashmap.fut:12:12-20
            
            bool bounds_check_18666 = x_18664 && y_18665;
            
            // stdlib/hashmap.fut:12:12-20
            
            bool index_certs_18667;
            
            if (!bounds_check_18666) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_18663, "] out of bounds for array of shape [", (long long) hashmap_test_res_17228, "].", "-> #0  stdlib/hashmap.fut:12:12-20\n   #1  stdlib/hashmap.fut:15:11-25\n   #2  probes/stdlib-tests.fut:15:11-14\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/hashmap.fut:12:12-20
            
            bool loop_cond_18668 = ((bool *) ext_mem_19132.mem)[loopres_18663];
            
            // stdlib/hashmap.fut:12:12-43
            
            bool loop_cond_18669;
            
            if (loop_cond_18668) {
                // stdlib/hashmap.fut:12:26-34
                
                int64_t loop_cond_t_res_18692 = ((int64_t *) ext_mem_19134.mem)[loopres_18663];
                
                // stdlib/hashmap.fut:12:37-43
                
                bool loop_cond_t_res_18693 = loop_cond_t_res_18692 == eta_p_17272;
                
                // stdlib/hashmap.fut:12:37-43
                
                bool loop_cond_t_res_18694 = !loop_cond_t_res_18693;
                
                loop_cond_18669 = loop_cond_t_res_18694;
            } else {
                loop_cond_18669 = 0;
            }
            
            bool loop_while_tmp_19304 = loop_cond_18669;
            int64_t i_tmp_19305 = loopres_18663;
            
            loop_while_18660 = loop_while_tmp_19304;
            i_18661 = i_tmp_19305;
        }
        slot_res_18658 = loop_while_18660;
        slot_res_18659 = i_18661;
        // stdlib/hashmap.fut:16:9-17
        
        bool x_17274 = sle64((int64_t) 0, slot_res_18659);
        
        // stdlib/hashmap.fut:16:9-17
        
        bool y_17275 = slt64(slot_res_18659, hashmap_test_res_17228);
        
        // stdlib/hashmap.fut:16:9-17
        
        bool bounds_check_17276 = x_17274 && y_17275;
        
        // stdlib/hashmap.fut:16:9-17
        
        bool index_certs_17277;
        
        if (!bounds_check_17276) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_18659, "] out of bounds for array of shape [", (long long) hashmap_test_res_17228, "].", "-> #0  stdlib/hashmap.fut:16:9-17\n   #1  probes/stdlib-tests.fut:15:11-14\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:16:9-17
        
        bool get_res_17278 = ((bool *) ext_mem_19132.mem)[slot_res_18659];
        
        // stdlib/hashmap.fut:16:6-42
        
        int64_t get_res_17279;
        
        if (get_res_17278) {
            // stdlib/hashmap.fut:16:25-33
            
            int64_t get_res_t_res_18695 = ((int64_t *) ext_mem_19133.mem)[slot_res_18659];
            
            get_res_17279 = get_res_t_res_18695;
        } else {
            get_res_17279 = (int64_t) 0;
        }
        ((int64_t *) mem_19136.mem)[i_19072] = get_res_17279;
    }
    if (memblock_unref(ctx, &ext_mem_19132, "ext_mem_19132") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_19133, "ext_mem_19133") != 0)
        return 1;
    if (memblock_unref(ctx, &ext_mem_19134, "ext_mem_19134") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19289, &mem_19136, "mem_19136") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19380, &mem_out_19289, "mem_out_19289") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_19136, "mem_19136") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19295, "mem_param_tmp_19295") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19294, "mem_param_tmp_19294") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19293, "mem_param_tmp_19293") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19123, "ext_mem_19123") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19124, "ext_mem_19124") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19125, "ext_mem_19125") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19105, "ext_mem_19105") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19106, "ext_mem_19106") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19107, "ext_mem_19107") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19110, "ext_mem_19110") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19113, "ext_mem_19113") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19116, "ext_mem_19116") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19098, "mem_param_19098") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19095, "mem_param_19095") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19092, "mem_param_19092") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19132, "ext_mem_19132") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19133, "ext_mem_19133") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19134, "ext_mem_19134") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19089, "mem_19089") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19088, "mem_19088") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19087, "mem_19087") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19289, "mem_out_19289") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_entry_regex_test(struct futhark_context *ctx, struct memblock *mem_out_p_19381, int64_t *out_prim_out_19382, int64_t *out_prim_out_19383, struct memblock pattern_mem_19086, struct memblock text_mem_19087, struct memblock replacement_mem_19088, int64_t dz2080U_15585, int64_t dz2081U_15586, int64_t dz2082U_15587)
{
    (void) ctx;
    
    int err = 0;
    int64_t mem_19157_cached_sizze_19384 = 0;
    unsigned char *mem_19157 = NULL;
    struct memblock mem_19172;
    
    mem_19172.references = NULL;
    
    struct memblock ext_mem_19112;
    
    ext_mem_19112.references = NULL;
    
    struct memblock mem_19111;
    
    mem_19111.references = NULL;
    
    struct memblock mem_19113;
    
    mem_19113.references = NULL;
    
    struct memblock ext_mem_19114;
    
    ext_mem_19114.references = NULL;
    
    struct memblock mem_19131;
    
    mem_19131.references = NULL;
    
    struct memblock ext_mem_19140;
    
    ext_mem_19140.references = NULL;
    
    struct memblock mem_param_tmp_19306;
    
    mem_param_tmp_19306.references = NULL;
    
    struct memblock mem_param_tmp_19310;
    
    mem_param_tmp_19310.references = NULL;
    
    struct memblock ext_mem_19124;
    
    ext_mem_19124.references = NULL;
    
    struct memblock mem_param_19121;
    
    mem_param_19121.references = NULL;
    
    struct memblock ext_mem_19127;
    
    ext_mem_19127.references = NULL;
    
    struct memblock mem_param_19118;
    
    mem_param_19118.references = NULL;
    
    struct memblock ext_mem_19130;
    
    ext_mem_19130.references = NULL;
    
    struct memblock mem_19115;
    
    mem_19115.references = NULL;
    
    struct memblock ext_mem_19143;
    
    ext_mem_19143.references = NULL;
    
    struct memblock mem_19097;
    
    mem_19097.references = NULL;
    
    struct memblock mem_19095;
    
    mem_19095.references = NULL;
    
    struct memblock mem_19093;
    
    mem_19093.references = NULL;
    
    struct memblock mem_19091;
    
    mem_19091.references = NULL;
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    int64_t prim_out_19290;
    int64_t prim_out_19291;
    
    // stdlib/regex.fut:26:28-30
    
    int64_t sizze_18199 = add64((int64_t) 1, dz2080U_15585);
    int64_t binop_x_19089 = (int64_t) 8 * sizze_18199;
    int64_t bytes_19090 = (int64_t) 32 * sizze_18199;
    
    if (memblock_alloc(ctx, &mem_19091, bytes_19090, "mem_19091")) {
        err = 1;
        goto cleanup;
    }
    // probes/stdlib-tests.fut:9:12-27
    for (int64_t nest_i_19292 = 0; nest_i_19292 < sizze_18199; nest_i_19292++) {
        for (int64_t nest_i_19293 = 0; nest_i_19293 < (int64_t) 4; nest_i_19293++) {
            ((int64_t *) mem_19091.mem)[nest_i_19292 * (int64_t) 4 + nest_i_19293] = (int64_t) 0;
        }
    }
    if (memblock_alloc(ctx, &mem_19093, binop_x_19089, "mem_19093")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:47-66
    for (int64_t nest_i_19294 = 0; nest_i_19294 < sizze_18199; nest_i_19294++) {
        ((int64_t *) mem_19093.mem)[nest_i_19294] = (int64_t) 1;
    }
    if (memblock_alloc(ctx, &mem_19095, binop_x_19089, "mem_19095")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:67-86
    for (int64_t nest_i_19295 = 0; nest_i_19295 < sizze_18199; nest_i_19295++) {
        ((int64_t *) mem_19095.mem)[nest_i_19295] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_19097, binop_x_19089, "mem_19097")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:29:87-106
    for (int64_t nest_i_19296 = 0; nest_i_19296 < sizze_18199; nest_i_19296++) {
        ((int64_t *) mem_19097.mem)[nest_i_19296] = (int64_t) 0;
    }
    // stdlib/regex.fut:28:5-49:95
    
    bool loop_cond_18204 = slt64((int64_t) 0, dz2080U_15585);
    
    // stdlib/regex.fut:28:5-49:95
    
    bool compile_res_18205;
    int64_t compile_res_18206;
    int64_t compile_res_18211;
    int64_t compile_res_18212;
    bool loop_while_18213;
    int64_t p_18214;
    int64_t used_18219;
    int64_t branch_18220;
    
    loop_while_18213 = loop_cond_18204;
    p_18214 = (int64_t) 0;
    used_18219 = (int64_t) 0;
    branch_18220 = (int64_t) 0;
    while (loop_while_18213) {
        // stdlib/regex.fut:31:12-22
        
        bool x_18221 = sle64((int64_t) 0, p_18214);
        
        // stdlib/regex.fut:31:12-22
        
        bool y_18222 = slt64(p_18214, dz2080U_15585);
        
        // stdlib/regex.fut:31:12-22
        
        bool bounds_check_18223 = x_18221 && y_18222;
        
        // stdlib/regex.fut:31:12-22
        
        bool index_certs_18224;
        
        if (!bounds_check_18223) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) p_18214, "] out of bounds for array of shape [", (long long) dz2080U_15585, "].", "-> #0  stdlib/regex.fut:31:12-22\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:31:12-22
        
        int8_t zeze_lhs_18225 = ((int8_t *) pattern_mem_19086.mem)[p_18214];
        
        // stdlib/regex.fut:31:9-49:95
        
        bool cond_18226 = zeze_lhs_18225 == (int8_t) 124;
        
        // stdlib/regex.fut:36:29-42:57
        
        bool cond_18246 = zeze_lhs_18225 == (int8_t) 91;
        
        // stdlib/regex.fut:31:9-49:95
        
        int64_t loopres_18227;
        int64_t loopres_18232;
        int64_t loopres_18233;
        
        if (cond_18226) {
            // stdlib/regex.fut:32:15-34
            
            bool x_18888 = sle64((int64_t) 0, branch_18220);
            
            // stdlib/regex.fut:32:15-34
            
            bool y_18889 = slt64(branch_18220, sizze_18199);
            
            // stdlib/regex.fut:32:15-34
            
            bool bounds_check_18890 = x_18888 && y_18889;
            
            // stdlib/regex.fut:32:15-34
            
            bool index_certs_18891;
            
            if (!bounds_check_18890) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) branch_18220, "] out of bounds for array of shape [", (long long) sizze_18199, "].", "-> #0  stdlib/regex.fut:32:15-34\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:33:28-30
            
            int64_t tmp_18893 = add64((int64_t) 1, branch_18220);
            
            // stdlib/regex.fut:33:15-38
            
            bool x_18894 = sle64((int64_t) 0, tmp_18893);
            
            // stdlib/regex.fut:33:15-38
            
            bool y_18895 = slt64(tmp_18893, sizze_18199);
            
            // stdlib/regex.fut:33:15-38
            
            bool bounds_check_18896 = x_18894 && y_18895;
            
            // stdlib/regex.fut:33:15-38
            
            bool index_certs_18897;
            
            if (!bounds_check_18896) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_18893, "] out of bounds for array of shape [", (long long) sizze_18199, "].", "-> #0  stdlib/regex.fut:33:15-38\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:32:15-34
            ((int64_t *) mem_19097.mem)[branch_18220] = used_18219;
            // stdlib/regex.fut:33:15-38
            ((int64_t *) mem_19095.mem)[tmp_18893] = used_18219;
            // stdlib/regex.fut:34:16-18
            
            int64_t tmp_18899 = add64((int64_t) 1, p_18214);
            
            loopres_18227 = tmp_18899;
            loopres_18232 = used_18219;
            loopres_18233 = tmp_18893;
        } else {
            // stdlib/regex.fut:15:23-25
            
            int64_t zl_lhs_18935 = add64((int64_t) 1, p_18214);
            
            // stdlib/regex.fut:15:18-68
            
            bool cond_18936 = slt64(zl_lhs_18935, dz2080U_15585);
            
            // stdlib/regex.fut:15:46-62
            
            bool x_18938 = sle64((int64_t) 0, zl_lhs_18935);
            
            // stdlib/regex.fut:15:46-62
            
            bool bounds_check_18939 = cond_18936 && x_18938;
            bool loop_not_taken_19268 = !cond_18936;
            bool protect_assert_disj_19269 = bounds_check_18939 || loop_not_taken_19268;
            bool loop_not_taken_19274 = !cond_18246;
            bool protect_assert_disj_19275 = protect_assert_disj_19269 || loop_not_taken_19274;
            
            // stdlib/regex.fut:15:46-62
            
            bool index_certs_18940;
            
            if (!protect_assert_disj_19275) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_18935, "] out of bounds for array of shape [", (long long) dz2080U_15585, "].", "-> #0  stdlib/regex.fut:15:46-62\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:9:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:47:15-33
            
            bool x_18342 = sle64((int64_t) 0, used_18219);
            
            // stdlib/regex.fut:47:15-33
            
            bool y_18343 = slt64(used_18219, sizze_18199);
            
            // stdlib/regex.fut:47:15-33
            
            bool bounds_check_18344 = x_18342 && y_18343;
            
            // stdlib/regex.fut:47:15-33
            
            bool index_certs_18345;
            
            if (!bounds_check_18344) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) used_18219, "] out of bounds for array of shape [", (long long) sizze_18199, "].", "-> #0  stdlib/regex.fut:47:15-33\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:37:18-42:57
            
            bool cond_18319 = zeze_lhs_18225 == (int8_t) 46;
            bool x_19277 = !cond_18319;
            bool protect_cond_conj_19285 = loop_not_taken_19274 && x_19277;
            
            // stdlib/regex.fut:41:33-50
            
            int8_t loopres_f_res_f_res_f_res_18325;
            int64_t loopres_f_res_f_res_f_res_18326;
            
            if (protect_cond_conj_19285) {
                int8_t x_19279;
                int64_t x_19280;
                
                if (futrts_literal_10630(ctx, &x_19279, &x_19280, pattern_mem_19086, dz2080U_15585, p_18214) != 0) {
                    err = 1;
                    goto cleanup;
                }
                loopres_f_res_f_res_f_res_18325 = x_19279;
                loopres_f_res_f_res_f_res_18326 = x_19280;
            } else {
                loopres_f_res_f_res_f_res_18325 = (int8_t) 0;
                loopres_f_res_f_res_f_res_18326 = (int64_t) 0;
            }
            
            bool protect_cond_conj_19276 = cond_18246 && cond_18936;
            
            // stdlib/regex.fut:15:46-62
            
            int8_t zeze_lhs_18941;
            
            if (protect_cond_conj_19276) {
                int8_t x_19270 = ((int8_t *) pattern_mem_19086.mem)[zl_lhs_18935];
                
                zeze_lhs_18941 = x_19270;
            } else {
                zeze_lhs_18941 = (int8_t) 0;
            }
            // stdlib/regex.fut:15:63-68
            
            bool negative_t_res_18942 = zeze_lhs_18941 == (int8_t) 94;
            bool x_19286 = cond_18936 && negative_t_res_18942;
            
            // stdlib/regex.fut:36:54-74
            
            int64_t zp_rhs_18943 = btoi_bool_i64(x_19286);
            
            // stdlib/regex.fut:16:22-49
            
            int64_t start_18944 = add64(zl_lhs_18935, zp_rhs_18943);
            
            // stdlib/regex.fut:17:18-23:21
            
            bool cond_18946 = slt64(start_18944, dz2080U_15585);
            int64_t loopres_f_res_18248;
            
            if (cond_18246) {
                if (memblock_alloc(ctx, &mem_19115, (int64_t) 32, "mem_19115")) {
                    err = 1;
                    goto cleanup;
                }
                // stdlib/regex.fut:17:35-51
                for (int64_t nest_i_19305 = 0; nest_i_19305 < (int64_t) 4; nest_i_19305++) {
                    ((int64_t *) mem_19115.mem)[nest_i_19305] = (int64_t) 0;
                }
                // stdlib/regex.fut:17:18-23:21
                
                bool class_mask_res_18947;
                int64_t class_mask_res_18949;
                bool loop_while_18950;
                int64_t p_18952;
                
                if (memblock_set(ctx, &mem_param_19118, &mem_19115, "mem_19115") != 0)
                    return 1;
                loop_while_18950 = cond_18946;
                p_18952 = start_18944;
                while (loop_while_18950) {
                    // stdlib/regex.fut:19:23-40
                    
                    int8_t loopres_18953;
                    int64_t loopres_18954;
                    
                    if (futrts_literal_10630(ctx, &loopres_18953, &loopres_18954, pattern_mem_19086, dz2080U_15585, p_18952) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:20:23-25
                    
                    int64_t zl_lhs_18955 = add64((int64_t) 1, loopres_18954);
                    
                    // stdlib/regex.fut:20:19-90
                    
                    bool cond_18956 = slt64(zl_lhs_18955, dz2080U_15585);
                    
                    // stdlib/regex.fut:20:19-90
                    
                    bool cond_18957;
                    
                    if (cond_18956) {
                        // stdlib/regex.fut:20:46-59
                        
                        bool x_18958 = sle64((int64_t) 0, loopres_18954);
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool y_18959 = slt64(loopres_18954, dz2080U_15585);
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool bounds_check_18960 = x_18958 && y_18959;
                        
                        // stdlib/regex.fut:20:46-59
                        
                        bool index_certs_18961;
                        
                        if (!bounds_check_18960) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_18954, "] out of bounds for array of shape [", (long long) dz2080U_15585, "].", "-> #0  stdlib/regex.fut:20:46-59\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:9:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:20:46-59
                        
                        int8_t zeze_lhs_18962 = ((int8_t *) pattern_mem_19086.mem)[loopres_18954];
                        
                        // stdlib/regex.fut:20:60-65
                        
                        bool cond_t_res_18963 = zeze_lhs_18962 == (int8_t) 45;
                        
                        cond_18957 = cond_t_res_18963;
                    } else {
                        cond_18957 = 0;
                    }
                    // stdlib/regex.fut:20:19-90
                    
                    bool range_18964;
                    
                    if (cond_18957) {
                        // stdlib/regex.fut:20:69-84
                        
                        bool x_18965 = sle64((int64_t) 0, zl_lhs_18955);
                        
                        // stdlib/regex.fut:20:69-84
                        
                        bool bounds_check_18966 = cond_18956 && x_18965;
                        
                        // stdlib/regex.fut:20:69-84
                        
                        bool index_certs_18967;
                        
                        if (!bounds_check_18966) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_18955, "] out of bounds for array of shape [", (long long) dz2080U_15585, "].", "-> #0  stdlib/regex.fut:20:69-84\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:9:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:20:69-84
                        
                        int8_t znze_lhs_18968 = ((int8_t *) pattern_mem_19086.mem)[zl_lhs_18955];
                        
                        // stdlib/regex.fut:20:85-90
                        
                        bool range_t_res_18969 = znze_lhs_18968 == (int8_t) 93;
                        
                        // stdlib/regex.fut:20:85-90
                        
                        bool range_t_res_18970 = !range_t_res_18969;
                        
                        range_18964 = range_t_res_18970;
                    } else {
                        range_18964 = 0;
                    }
                    // stdlib/regex.fut:21:23-76
                    
                    int8_t loopres_18971;
                    int64_t loopres_18972;
                    
                    if (range_18964) {
                        // stdlib/regex.fut:21:37-60
                        
                        int8_t loopres_t_res_18973;
                        int64_t loopres_t_res_18974;
                        
                        if (futrts_literal_10630(ctx, &loopres_t_res_18973, &loopres_t_res_18974, pattern_mem_19086, dz2080U_15585, zl_lhs_18955) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        loopres_18971 = loopres_t_res_18973;
                        loopres_18972 = loopres_t_res_18974;
                    } else {
                        loopres_18971 = loopres_18953;
                        loopres_18972 = loopres_18954;
                    }
                    // stdlib/regex.fut:22:48-57
                    
                    int64_t u8_res_18975 = zext_i8_i64(loopres_18953);
                    
                    // stdlib/regex.fut:22:60-69
                    
                    int64_t u8_res_18976 = zext_i8_i64(loopres_18971);
                    
                    // stdlib/regex.fut:22:69-71
                    
                    int64_t mask_18977 = add64((int64_t) 1, u8_res_18976);
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool bounds_invalid_upwards_18978 = slt64(mask_18977, u8_res_18975);
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool valid_18980 = !bounds_invalid_upwards_18978;
                    
                    // stdlib/regex.fut:22:18-103
                    
                    bool range_valid_c_18981;
                    
                    if (!valid_18980) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) u8_res_18975, "..<", (long long) mask_18977, " is invalid.", "-> #0  stdlib/regex.fut:22:18-103\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:9:12-27\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:22:18-103
                    
                    int64_t distance_18979 = sub64(mask_18977, u8_res_18975);
                    
                    // stdlib/regex.fut:22:18-103
                    if (memblock_set(ctx, &mem_param_19121, &mem_param_19118, "mem_param_19118") != 0)
                        return 1;
                    for (int64_t i_18983 = 0; i_18983 < distance_18979; i_18983++) {
                        // stdlib/regex.fut:36:54-74
                        
                        int64_t index_primexp_18985 = add64(u8_res_18975, i_18983);
                        
                        // stdlib/regex.fut:22:90-102
                        
                        int8_t unsign_arg0_18986 = zext_i64_i8(index_primexp_18985);
                        
                        // stdlib/regex.fut:22:75-102
                        if (futrts_mask_set_8060(ctx, &ext_mem_19124, mem_param_19121, unsign_arg0_18986) != 0) {
                            err = 1;
                            goto cleanup;
                        }
                        if (memblock_set(ctx, &mem_param_tmp_19310, &ext_mem_19124, "ext_mem_19124") != 0)
                            return 1;
                        if (memblock_set(ctx, &mem_param_19121, &mem_param_tmp_19310, "mem_param_tmp_19310") != 0)
                            return 1;
                    }
                    if (memblock_set(ctx, &ext_mem_19127, &mem_param_19121, "mem_param_19121") != 0)
                        return 1;
                    // stdlib/regex.fut:18:11-65
                    
                    bool cond_18988 = slt64(loopres_18972, dz2080U_15585);
                    
                    // stdlib/regex.fut:18:11-65
                    
                    bool loop_cond_18989;
                    
                    if (cond_18988) {
                        // stdlib/regex.fut:18:34-44
                        
                        bool x_18990 = sle64((int64_t) 0, loopres_18972);
                        
                        // stdlib/regex.fut:18:34-44
                        
                        bool bounds_check_18991 = cond_18988 && x_18990;
                        
                        // stdlib/regex.fut:18:34-44
                        
                        bool index_certs_18992;
                        
                        if (!bounds_check_18991) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_18972, "] out of bounds for array of shape [", (long long) dz2080U_15585, "].", "-> #0  stdlib/regex.fut:18:34-44\n   #1  stdlib/regex.fut:36:54-74\n   #2  probes/stdlib-tests.fut:9:12-27\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:18:34-44
                        
                        int8_t znze_lhs_18993 = ((int8_t *) pattern_mem_19086.mem)[loopres_18972];
                        
                        // stdlib/regex.fut:18:34-64
                        
                        bool cond_18994 = znze_lhs_18993 == (int8_t) 93;
                        
                        // stdlib/regex.fut:18:34-64
                        
                        bool cond_18995 = !cond_18994;
                        
                        // stdlib/regex.fut:18:56-64
                        
                        bool loop_cond_t_res_f_res_18996 = loopres_18972 == start_18944;
                        
                        // stdlib/regex.fut:36:54-74
                        
                        bool y_18997 = cond_18994 && loop_cond_t_res_f_res_18996;
                        
                        // stdlib/regex.fut:36:54-74
                        
                        bool loop_cond_t_res_18998 = cond_18995 || y_18997;
                        
                        loop_cond_18989 = loop_cond_t_res_18998;
                    } else {
                        loop_cond_18989 = 0;
                    }
                    if (memblock_set(ctx, &mem_param_tmp_19306, &ext_mem_19127, "ext_mem_19127") != 0)
                        return 1;
                    
                    bool loop_while_tmp_19307 = loop_cond_18989;
                    int64_t p_tmp_19309 = loopres_18972;
                    
                    if (memblock_set(ctx, &mem_param_19118, &mem_param_tmp_19306, "mem_param_tmp_19306") != 0)
                        return 1;
                    loop_while_18950 = loop_while_tmp_19307;
                    p_18952 = p_tmp_19309;
                }
                if (memblock_set(ctx, &ext_mem_19130, &mem_param_19118, "mem_param_19118") != 0)
                    return 1;
                class_mask_res_18947 = loop_while_18950;
                class_mask_res_18949 = p_18952;
                if (memblock_unref(ctx, &mem_19115, "mem_19115") != 0)
                    return 1;
                if (x_19286) {
                    if (memblock_alloc(ctx, &mem_19131, (int64_t) 32, "mem_19131")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:24:24-56
                    for (int64_t i_19072 = 0; i_19072 < (int64_t) 4; i_19072++) {
                        int64_t eta_p_19001 = ((int64_t *) ext_mem_19130.mem)[i_19072];
                        
                        // stdlib/regex.fut:24:37-50
                        
                        int64_t lifted_lambda_res_19002 = (int64_t) -1 ^ eta_p_19001;
                        
                        ((int64_t *) mem_19131.mem)[i_19072] = lifted_lambda_res_19002;
                    }
                    if (memblock_set(ctx, &ext_mem_19140, &mem_19131, "mem_19131") != 0)
                        return 1;
                } else if (memblock_set(ctx, &ext_mem_19140, &ext_mem_19130, "ext_mem_19130") != 0)
                    return 1;
                if (memblock_unref(ctx, &ext_mem_19130, "ext_mem_19130") != 0)
                    return 1;
                // stdlib/regex.fut:24:68-70
                
                int64_t tmp_19006 = add64((int64_t) 1, class_mask_res_18949);
                
                if (memblock_set(ctx, &ext_mem_19143, &ext_mem_19140, "ext_mem_19140") != 0)
                    return 1;
                loopres_f_res_18248 = tmp_19006;
            } else {
                int64_t loopres_f_res_f_res_18321;
                
                if (cond_18319) {
                    loopres_f_res_f_res_18321 = zl_lhs_18935;
                } else {
                    loopres_f_res_f_res_18321 = loopres_f_res_f_res_f_res_18326;
                }
                if (cond_18319) {
                    if (memblock_alloc(ctx, &mem_19113, (int64_t) 32, "mem_19113")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:38:26-49
                    for (int64_t nest_i_19313 = 0; nest_i_19313 < (int64_t) 4; nest_i_19313++) {
                        ((int64_t *) mem_19113.mem)[nest_i_19313] = (int64_t) -1;
                    }
                    // stdlib/regex.fut:39:19-51
                    ((int64_t *) mem_19113.mem)[(int64_t) 0] = (int64_t) -1025;
                    if (memblock_set(ctx, &ext_mem_19114, &mem_19113, "mem_19113") != 0)
                        return 1;
                } else {
                    if (memblock_alloc(ctx, &mem_19111, (int64_t) 32, "mem_19111")) {
                        err = 1;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:42:32-48
                    for (int64_t nest_i_19314 = 0; nest_i_19314 < (int64_t) 4; nest_i_19314++) {
                        ((int64_t *) mem_19111.mem)[nest_i_19314] = (int64_t) 0;
                    }
                    // stdlib/regex.fut:42:22-51
                    if (futrts_mask_set_8060(ctx, &ext_mem_19112, mem_19111, loopres_f_res_f_res_f_res_18325) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    if (memblock_unref(ctx, &mem_19111, "mem_19111") != 0)
                        return 1;
                    if (memblock_set(ctx, &ext_mem_19114, &ext_mem_19112, "ext_mem_19112") != 0)
                        return 1;
                }
                if (memblock_set(ctx, &ext_mem_19143, &ext_mem_19114, "ext_mem_19114") != 0)
                    return 1;
                loopres_f_res_18248 = loopres_f_res_f_res_18321;
            }
            // stdlib/regex.fut:43:28-46:54
            
            bool cond_18329 = sle64(dz2080U_15585, loopres_f_res_18248);
            
            // stdlib/regex.fut:43:28-46:54
            
            int64_t quantifier_18330;
            
            if (cond_18329) {
                quantifier_18330 = (int64_t) 1;
            } else {
                // stdlib/regex.fut:44:21-34
                
                bool x_18331 = sle64((int64_t) 0, loopres_f_res_18248);
                
                // stdlib/regex.fut:44:21-34
                
                bool y_18332 = slt64(loopres_f_res_18248, dz2080U_15585);
                
                // stdlib/regex.fut:44:21-34
                
                bool bounds_check_18333 = x_18331 && y_18332;
                
                // stdlib/regex.fut:44:21-34
                
                bool index_certs_18334;
                
                if (!bounds_check_18333) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_f_res_18248, "] out of bounds for array of shape [", (long long) dz2080U_15585, "].", "-> #0  stdlib/regex.fut:44:21-34\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:44:21-34
                
                int8_t zeze_lhs_18335 = ((int8_t *) pattern_mem_19086.mem)[loopres_f_res_18248];
                
                // stdlib/regex.fut:44:18-46:54
                
                bool cond_18336 = zeze_lhs_18335 == (int8_t) 42;
                
                // stdlib/regex.fut:44:18-46:54
                
                int64_t quantifier_f_res_18337;
                
                if (cond_18336) {
                    quantifier_f_res_18337 = (int64_t) 0;
                } else {
                    // stdlib/regex.fut:45:18-46:54
                    
                    bool cond_18338 = zeze_lhs_18335 == (int8_t) 43;
                    
                    // stdlib/regex.fut:45:18-46:54
                    
                    int64_t quantifier_f_res_f_res_18339;
                    
                    if (cond_18338) {
                        quantifier_f_res_f_res_18339 = (int64_t) 2;
                    } else {
                        // stdlib/regex.fut:46:18-54
                        
                        bool cond_18340 = zeze_lhs_18335 == (int8_t) 63;
                        
                        // stdlib/regex.fut:46:18-54
                        
                        int64_t quantifier_f_res_f_res_f_res_18341;
                        
                        if (cond_18340) {
                            quantifier_f_res_f_res_f_res_18341 = (int64_t) 3;
                        } else {
                            quantifier_f_res_f_res_f_res_18341 = (int64_t) 1;
                        }
                        quantifier_f_res_f_res_18339 = quantifier_f_res_f_res_f_res_18341;
                    }
                    quantifier_f_res_18337 = quantifier_f_res_f_res_18339;
                }
                quantifier_18330 = quantifier_f_res_18337;
            }
            // stdlib/regex.fut:47:15-33
            // stdlib/regex.fut:47:15-33
            lmad_copy_8b(ctx, 1, (uint64_t *) mem_19091.mem, used_18219 * (int64_t) 4, (int64_t []) {(int64_t) 1}, (uint64_t *) ext_mem_19143.mem, (int64_t) 0, (int64_t []) {(int64_t) 1}, (int64_t []) {(int64_t) 4});
            if (memblock_unref(ctx, &ext_mem_19143, "ext_mem_19143") != 0)
                return 1;
            // stdlib/regex.fut:48:15-41
            ((int64_t *) mem_19093.mem)[used_18219] = quantifier_18330;
            // stdlib/regex.fut:49:21-53
            
            bool cond_18348 = quantifier_18330 == (int64_t) 1;
            
            // probes/stdlib-tests.fut:9:12-27
            
            bool cond_neg_18349 = !cond_18348;
            
            // probes/stdlib-tests.fut:9:12-27
            
            int64_t zp_rhs_18350 = btoi_bool_i64(cond_neg_18349);
            
            // stdlib/regex.fut:49:19-53
            
            int64_t tmp_18351 = add64(loopres_f_res_18248, zp_rhs_18350);
            
            // stdlib/regex.fut:49:85-87
            
            int64_t tmp_18352 = add64((int64_t) 1, used_18219);
            
            loopres_18227 = tmp_18351;
            loopres_18232 = tmp_18352;
            loopres_18233 = branch_18220;
        }
        // stdlib/regex.fut:30:15-31
        
        bool loop_cond_18353 = slt64(loopres_18227, dz2080U_15585);
        bool loop_while_tmp_19297 = loop_cond_18353;
        int64_t p_tmp_19298 = loopres_18227;
        int64_t used_tmp_19303 = loopres_18232;
        int64_t branch_tmp_19304 = loopres_18233;
        
        loop_while_18213 = loop_while_tmp_19297;
        p_18214 = p_tmp_19298;
        used_18219 = used_tmp_19303;
        branch_18220 = branch_tmp_19304;
    }
    compile_res_18205 = loop_while_18213;
    compile_res_18206 = p_18214;
    compile_res_18211 = used_18219;
    compile_res_18212 = branch_18220;
    // stdlib/regex.fut:51:47-57
    
    int64_t dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354 = add64((int64_t) 1, compile_res_18212);
    
    // stdlib/regex.fut:50:7-28
    
    bool x_18355 = sle64((int64_t) 0, compile_res_18212);
    
    // stdlib/regex.fut:50:7-28
    
    bool y_18356 = slt64(compile_res_18212, sizze_18199);
    
    // stdlib/regex.fut:50:7-28
    
    bool bounds_check_18357 = x_18355 && y_18356;
    
    // stdlib/regex.fut:50:7-28
    
    bool index_certs_18358;
    
    if (!bounds_check_18357) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) compile_res_18212, "] out of bounds for array of shape [", (long long) sizze_18199, "].", "-> #0  stdlib/regex.fut:50:7-28\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:50:7-28
    ((int64_t *) mem_19097.mem)[compile_res_18212] = compile_res_18211;
    // stdlib/regex.fut:51:7-22
    
    bool empty_slice_18360 = compile_res_18211 == (int64_t) 0;
    
    // stdlib/regex.fut:51:7-22
    
    int64_t m_18361 = sub64(compile_res_18211, (int64_t) 1);
    
    // stdlib/regex.fut:51:7-22
    
    bool zzero_leq_i_p_m_t_s_18362 = sle64((int64_t) 0, m_18361);
    
    // stdlib/regex.fut:51:7-22
    
    bool i_p_m_t_s_leq_w_18363 = slt64(m_18361, sizze_18199);
    
    // stdlib/regex.fut:51:7-22
    
    bool i_lte_j_18364 = sle64((int64_t) 0, compile_res_18211);
    
    // stdlib/regex.fut:51:7-22
    
    bool y_18365 = zzero_leq_i_p_m_t_s_18362 && i_p_m_t_s_leq_w_18363;
    
    // stdlib/regex.fut:51:7-22
    
    bool forwards_ok_18366 = i_lte_j_18364 && y_18365;
    
    // stdlib/regex.fut:51:7-22
    
    bool ok_or_empty_18367 = empty_slice_18360 || forwards_ok_18366;
    
    // stdlib/regex.fut:51:7-22
    
    bool index_certs_18368;
    
    if (!ok_or_empty_18367) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) compile_res_18211, "] out of bounds for array of shape [", (long long) sizze_18199, "].", "-> #0  stdlib/regex.fut:51:7-22\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:23-40
    
    bool ok_or_empty_18370 = empty_slice_18360 || y_18365;
    
    // stdlib/regex.fut:51:23-40
    
    bool index_certs_18371;
    
    if (!ok_or_empty_18370) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) compile_res_18211, "] out of bounds for array of shape [", (long long) sizze_18199, "].", "-> #0  stdlib/regex.fut:51:23-40\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:41-65
    
    bool empty_slice_18373 = dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354 == (int64_t) 0;
    
    // stdlib/regex.fut:51:41-65
    
    bool i_lte_j_18374 = sle64((int64_t) 0, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354);
    
    // stdlib/regex.fut:51:41-65
    
    bool forwards_ok_18375 = bounds_check_18357 && i_lte_j_18374;
    
    // stdlib/regex.fut:51:41-65
    
    bool ok_or_empty_18376 = empty_slice_18373 || forwards_ok_18375;
    
    // stdlib/regex.fut:51:41-65
    
    bool index_certs_18377;
    
    if (!ok_or_empty_18376) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354, "] out of bounds for array of shape [", (long long) sizze_18199, "].", "-> #0  stdlib/regex.fut:51:41-65\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:51:66-88
    
    bool ok_or_empty_18379 = bounds_check_18357 || empty_slice_18373;
    
    // stdlib/regex.fut:51:66-88
    
    bool index_certs_18380;
    
    if (!ok_or_empty_18379) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354, "] out of bounds for array of shape [", (long long) sizze_18199, "].", "-> #0  stdlib/regex.fut:51:66-88\n   #1  probes/stdlib-tests.fut:9:12-27\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:92:15-22
    
    bool y_18382 = slt64((int64_t) 0, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354);
    
    // stdlib/regex.fut:92:15-22
    
    bool index_certs_18383;
    
    if (!y_18382) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354, "].", "-> #0  stdlib/regex.fut:92:15-22\n   #1  probes/stdlib-tests.fut:9:7-10:20\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:100:42-44
    
    int64_t zt_lhs_18498 = add64((int64_t) 1, dz2081U_15586);
    
    // stdlib/regex.fut:100:45-64
    
    int64_t zp_rhs_18499 = mul64(dz2082U_15587, zt_lhs_18498);
    
    // stdlib/regex.fut:100:29-64
    
    int64_t capacity_18500 = add64(dz2081U_15586, zp_rhs_18499);
    
    if (memblock_alloc(ctx, &mem_19172, capacity_18500, "mem_19172")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/regex.fut:101:63-85
    for (int64_t nest_i_19315 = 0; nest_i_19315 < capacity_18500; nest_i_19315++) {
        ((int8_t *) mem_19172.mem)[nest_i_19315] = (int8_t) 0;
    }
    // stdlib/regex.fut:101:30-117:72
    
    bool replace_res_18502;
    int64_t replace_res_18503;
    int64_t replace_res_18505;
    bool loop_while_18506;
    int64_t position_18507;
    int64_t used_18509;
    
    loop_while_18506 = 1;
    position_18507 = (int64_t) 0;
    used_18509 = (int64_t) 0;
    while (loop_while_18506) {
        // stdlib/regex.fut:99:14-103:47
        
        int64_t loopres_18510;
        int64_t loopres_18511;
        
        if (futrts_find_10646(ctx, &loopres_18510, &loopres_18511, mem_19091, mem_19093, mem_19095, mem_19097, text_mem_19087, compile_res_18211, compile_res_18211, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354, dz2081U_15586, position_18507) != 0) {
            err = 1;
            goto cleanup;
        }
        // stdlib/regex.fut:104:20-59
        
        bool cond_18512 = slt64(loopres_18511, (int64_t) 0);
        
        // stdlib/regex.fut:104:20-59
        
        int64_t finish_18513;
        
        if (cond_18512) {
            finish_18513 = dz2081U_15586;
        } else {
            finish_18513 = loopres_18510;
        }
        // stdlib/regex.fut:105:17-107:15
        
        bool bounds_invalid_upwards_18514 = slt64(finish_18513, position_18507);
        
        // stdlib/regex.fut:105:17-107:15
        
        bool valid_18516 = !bounds_invalid_upwards_18514;
        
        // stdlib/regex.fut:105:17-107:15
        
        bool range_valid_c_18517;
        
        if (!valid_18516) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) position_18507, "..<", (long long) finish_18513, " is invalid.", "-> #0  stdlib/regex.fut:105:17-107:15\n   #1  probes/stdlib-tests.fut:9:7-10:48\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:105:17-107:15
        
        int64_t distance_18515 = sub64(finish_18513, position_18507);
        
        // stdlib/regex.fut:105:17-107:15
        for (int64_t i_18519 = 0; i_18519 < distance_18515; i_18519++) {
            // probes/stdlib-tests.fut:9:7-10:48
            
            int64_t index_primexp_18521 = add64(position_18507, i_18519);
            
            // stdlib/regex.fut:106:21-23
            
            int64_t zm_lhs_18522 = add64(used_18509, index_primexp_18521);
            
            // stdlib/regex.fut:106:23-32
            
            int64_t tmp_18523 = sub64(zm_lhs_18522, position_18507);
            
            // stdlib/regex.fut:106:36-43
            
            bool x_18524 = sle64((int64_t) 0, index_primexp_18521);
            
            // stdlib/regex.fut:106:36-43
            
            bool y_18525 = slt64(index_primexp_18521, dz2081U_15586);
            
            // stdlib/regex.fut:106:36-43
            
            bool bounds_check_18526 = x_18524 && y_18525;
            
            // stdlib/regex.fut:106:36-43
            
            bool index_certs_18527;
            
            if (!bounds_check_18526) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) index_primexp_18521, "] out of bounds for array of shape [", (long long) dz2081U_15586, "].", "-> #0  stdlib/regex.fut:106:36-43\n   #1  probes/stdlib-tests.fut:9:7-10:48\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:106:13-43
            
            bool x_18529 = sle64((int64_t) 0, tmp_18523);
            
            // stdlib/regex.fut:106:13-43
            
            bool y_18530 = slt64(tmp_18523, capacity_18500);
            
            // stdlib/regex.fut:106:13-43
            
            bool bounds_check_18531 = x_18529 && y_18530;
            
            // stdlib/regex.fut:106:13-43
            
            bool index_certs_18532;
            
            if (!bounds_check_18531) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_18523, "] out of bounds for array of shape [", (long long) capacity_18500, "].", "-> #0  stdlib/regex.fut:106:13-43\n   #1  probes/stdlib-tests.fut:9:7-10:48\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:106:36-43
            
            int8_t tmp_18528 = ((int8_t *) text_mem_19087.mem)[index_primexp_18521];
            
            // stdlib/regex.fut:106:13-43
            ((int8_t *) mem_19172.mem)[tmp_18523] = tmp_18528;
        }
        // stdlib/regex.fut:108:22-29
        
        int64_t zm_lhs_18534 = add64(used_18509, finish_18513);
        
        // stdlib/regex.fut:108:29-38
        
        int64_t used_18535 = sub64(zm_lhs_18534, position_18507);
        int64_t loopres_18536;
        int64_t loopres_18538;
        
        if (cond_18512) {
            loopres_18536 = zt_lhs_18498;
            loopres_18538 = used_18535;
        } else {
            // stdlib/regex.fut:110:25-112:23
            for (int64_t i_18540 = 0; i_18540 < dz2082U_15587; i_18540++) {
                // stdlib/regex.fut:111:29-31
                
                int64_t tmp_18542 = add64(used_18535, i_18540);
                
                // stdlib/regex.fut:111:21-49
                
                bool x_18544 = sle64((int64_t) 0, tmp_18542);
                
                // stdlib/regex.fut:111:21-49
                
                bool y_18545 = slt64(tmp_18542, capacity_18500);
                
                // stdlib/regex.fut:111:21-49
                
                bool bounds_check_18546 = x_18544 && y_18545;
                
                // stdlib/regex.fut:111:21-49
                
                bool index_certs_18547;
                
                if (!bounds_check_18546) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) tmp_18542, "] out of bounds for array of shape [", (long long) capacity_18500, "].", "-> #0  stdlib/regex.fut:111:21-49\n   #1  probes/stdlib-tests.fut:9:7-10:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:111:35-49
                
                int8_t tmp_18543 = ((int8_t *) replacement_mem_19088.mem)[i_18540];
                
                // stdlib/regex.fut:111:21-49
                ((int8_t *) mem_19172.mem)[tmp_18542] = tmp_18543;
            }
            // stdlib/regex.fut:113:30-49
            
            int64_t used_18549 = add64(dz2082U_15587, used_18535);
            
            // stdlib/regex.fut:114:18-117:72
            
            bool cond_18550 = loopres_18510 == loopres_18511;
            
            // stdlib/regex.fut:114:43-56
            
            bool cond_t_res_18551 = slt64(loopres_18511, dz2081U_15586);
            
            // probes/stdlib-tests.fut:9:7-10:48
            
            bool x_18552 = cond_18550 && cond_t_res_18551;
            
            // stdlib/regex.fut:114:18-117:72
            
            int64_t loopres_f_res_18553;
            int64_t loopres_f_res_18555;
            
            if (x_18552) {
                // stdlib/regex.fut:115:36-46
                
                bool x_19058 = sle64((int64_t) 0, loopres_18511);
                
                // stdlib/regex.fut:115:36-46
                
                bool bounds_check_19059 = cond_t_res_18551 && x_19058;
                
                // stdlib/regex.fut:115:36-46
                
                bool index_certs_19060;
                
                if (!bounds_check_19059) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_18511, "] out of bounds for array of shape [", (long long) dz2081U_15586, "].", "-> #0  stdlib/regex.fut:115:36-46\n   #1  probes/stdlib-tests.fut:9:7-10:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:115:24-46
                
                bool x_19062 = sle64((int64_t) 0, used_18549);
                
                // stdlib/regex.fut:115:24-46
                
                bool y_19063 = slt64(used_18549, capacity_18500);
                
                // stdlib/regex.fut:115:24-46
                
                bool bounds_check_19064 = x_19062 && y_19063;
                
                // stdlib/regex.fut:115:24-46
                
                bool index_certs_19065;
                
                if (!bounds_check_19064) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) used_18549, "] out of bounds for array of shape [", (long long) capacity_18500, "].", "-> #0  stdlib/regex.fut:115:24-46\n   #1  probes/stdlib-tests.fut:9:7-10:48\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:115:36-46
                
                int8_t tmp_19061 = ((int8_t *) text_mem_19087.mem)[loopres_18511];
                
                // stdlib/regex.fut:115:24-46
                ((int8_t *) mem_19172.mem)[used_18549] = tmp_19061;
                // stdlib/regex.fut:116:28-30
                
                int64_t tmp_19067 = add64((int64_t) 1, loopres_18511);
                
                // stdlib/regex.fut:116:39-41
                
                int64_t tmp_19068 = add64((int64_t) 1, used_18549);
                
                loopres_f_res_18553 = tmp_19067;
                loopres_f_res_18555 = tmp_19068;
            } else {
                // stdlib/regex.fut:117:24-62
                
                int64_t tmp_18567;
                
                if (cond_18550) {
                    // stdlib/regex.fut:117:50-52
                    
                    int64_t tmp_t_res_19069 = add64((int64_t) 1, loopres_18511);
                    
                    tmp_18567 = tmp_t_res_19069;
                } else {
                    tmp_18567 = loopres_18511;
                }
                loopres_f_res_18553 = tmp_18567;
                loopres_f_res_18555 = used_18549;
            }
            loopres_18536 = loopres_f_res_18553;
            loopres_18538 = loopres_f_res_18555;
        }
        // stdlib/regex.fut:102:20-34
        
        bool loop_cond_18569 = sle64(loopres_18536, dz2081U_15586);
        bool loop_while_tmp_19316 = loop_cond_18569;
        int64_t position_tmp_19317 = loopres_18536;
        int64_t used_tmp_19319 = loopres_18538;
        
        loop_while_18506 = loop_while_tmp_19316;
        position_18507 = position_tmp_19317;
        used_18509 = used_tmp_19319;
    }
    replace_res_18502 = loop_while_18506;
    replace_res_18503 = position_18507;
    replace_res_18505 = used_18509;
    // stdlib/regex.fut:118:6-19
    
    bool empty_slice_18570 = replace_res_18505 == (int64_t) 0;
    
    // stdlib/regex.fut:118:6-19
    
    int64_t m_18571 = sub64(replace_res_18505, (int64_t) 1);
    
    // stdlib/regex.fut:118:6-19
    
    bool zzero_leq_i_p_m_t_s_18572 = sle64((int64_t) 0, m_18571);
    
    // stdlib/regex.fut:118:6-19
    
    bool i_p_m_t_s_leq_w_18573 = slt64(m_18571, capacity_18500);
    
    // stdlib/regex.fut:118:6-19
    
    bool i_lte_j_18574 = sle64((int64_t) 0, replace_res_18505);
    
    // stdlib/regex.fut:118:6-19
    
    bool y_18575 = zzero_leq_i_p_m_t_s_18572 && i_p_m_t_s_leq_w_18573;
    
    // stdlib/regex.fut:118:6-19
    
    bool forwards_ok_18576 = i_lte_j_18574 && y_18575;
    
    // stdlib/regex.fut:118:6-19
    
    bool ok_or_empty_18577 = empty_slice_18570 || forwards_ok_18576;
    
    // stdlib/regex.fut:118:6-19
    
    bool index_certs_18578;
    
    if (!ok_or_empty_18577) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) replace_res_18505, "] out of bounds for array of shape [", (long long) capacity_18500, "].", "-> #0  stdlib/regex.fut:118:6-19\n   #1  probes/stdlib-tests.fut:9:7-10:48\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    
    int64_t zm_lhs_18384 = ((int64_t *) mem_19097.mem)[(int64_t) 0];
    int64_t zm_rhs_18387 = ((int64_t *) mem_19095.mem)[(int64_t) 0];
    
    // stdlib/regex.fut:92:22-30
    
    int64_t width_18388 = sub64(zm_lhs_18384, zm_rhs_18387);
    
    // stdlib/regex.fut:93:15-103
    
    bool cond_18389 = slt64((int64_t) 0, width_18388);
    
    // stdlib/regex.fut:93:15-103
    
    bool cond_18390;
    
    if (cond_18389) {
        // stdlib/regex.fut:93:28-42
        
        bool defunc_0_reduce_res_19011;
        bool redout_19074 = 1;
        
        for (int64_t i_19075 = 0; i_19075 < compile_res_18211; i_19075++) {
            int64_t eta_p_19012 = ((int64_t *) mem_19093.mem)[i_19075];
            
            // stdlib/regex.fut:93:32-37
            
            bool lifted_lambda_res_19013 = eta_p_19012 == (int64_t) 1;
            
            // stdlib/regex.fut:93:28-42
            
            bool x_19016 = lifted_lambda_res_19013 && redout_19074;
            bool redout_tmp_19322 = x_19016;
            
            redout_19074 = redout_tmp_19322;
        }
        defunc_0_reduce_res_19011 = redout_19074;
        cond_18390 = defunc_0_reduce_res_19011;
    } else {
        cond_18390 = 0;
    }
    // stdlib/regex.fut:93:15-103
    
    bool fixed_18400;
    
    if (cond_18390) {
        // stdlib/regex.fut:93:46-103
        
        bool defunc_0_reduce_res_19021;
        bool redout_19076 = 1;
        
        for (int64_t i_19077 = 0; i_19077 < dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354; i_19077++) {
            int64_t zm_lhs_19027 = ((int64_t *) mem_19097.mem)[i_19077];
            int64_t zm_rhs_19028 = ((int64_t *) mem_19095.mem)[i_19077];
            
            // stdlib/regex.fut:93:64-72
            
            int64_t zeze_lhs_19029 = sub64(zm_lhs_19027, zm_rhs_19028);
            
            // stdlib/regex.fut:93:73-81
            
            bool lifted_lambda_res_19030 = zeze_lhs_19029 == width_18388;
            
            // stdlib/regex.fut:93:46-103
            
            bool x_19033 = lifted_lambda_res_19030 && redout_19076;
            bool redout_tmp_19323 = x_19033;
            
            redout_19076 = redout_tmp_19323;
        }
        defunc_0_reduce_res_19021 = redout_19076;
        fixed_18400 = defunc_0_reduce_res_19021;
    } else {
        fixed_18400 = 0;
    }
    // stdlib/regex.fut:94:6-98:13
    
    bool cond_18421 = !fixed_18400;
    
    // stdlib/regex.fut:95:88-94
    
    int64_t zp_lhs_18422 = sub64(dz2081U_15586, width_18388);
    
    // stdlib/regex.fut:95:94-96
    
    int64_t max_arg1_18423 = add64((int64_t) 1, zp_lhs_18422);
    
    // probes/stdlib-tests.fut:9:7-10:20
    
    int64_t max_res_18424 = smax64((int64_t) 0, max_arg1_18423);
    
    // stdlib/regex.fut:94:6-98:13
    
    int64_t count_res_18425;
    
    if (cond_18421) {
        // stdlib/regex.fut:79:27-82:61
        
        bool count_general_res_19038;
        int64_t count_general_res_19039;
        int64_t count_general_res_19040;
        bool loop_while_19041;
        int64_t position_19042;
        int64_t count_19043;
        
        loop_while_19041 = 1;
        position_19042 = (int64_t) 0;
        count_19043 = (int64_t) 0;
        while (loop_while_19041) {
            // stdlib/regex.fut:78:20-80:45
            
            int64_t loopres_19044;
            int64_t loopres_19045;
            
            if (futrts_find_10646(ctx, &loopres_19044, &loopres_19045, mem_19091, mem_19093, mem_19095, mem_19097, text_mem_19087, compile_res_18211, compile_res_18211, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354, dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354, dz2081U_15586, position_19042) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/regex.fut:81:8-82:61
            
            bool cond_19046 = slt64(loopres_19045, (int64_t) 0);
            
            // stdlib/regex.fut:81:8-82:61
            
            int64_t loopres_19047;
            int64_t loopres_19048;
            
            if (cond_19046) {
                loopres_19047 = zt_lhs_18498;
                loopres_19048 = count_19043;
            } else {
                // stdlib/regex.fut:82:14-52
                
                bool cond_19050 = loopres_19045 == loopres_19044;
                
                // stdlib/regex.fut:82:14-52
                
                int64_t tmp_19051;
                
                if (cond_19050) {
                    // stdlib/regex.fut:82:40-42
                    
                    int64_t tmp_t_res_19052 = add64((int64_t) 1, loopres_19045);
                    
                    tmp_19051 = tmp_t_res_19052;
                } else {
                    tmp_19051 = loopres_19045;
                }
                // stdlib/regex.fut:82:58-60
                
                int64_t tmp_19053 = add64((int64_t) 1, count_19043);
                
                loopres_19047 = tmp_19051;
                loopres_19048 = tmp_19053;
            }
            // stdlib/regex.fut:79:78-92
            
            bool loop_cond_19054 = sle64(loopres_19047, dz2081U_15586);
            bool loop_while_tmp_19324 = loop_cond_19054;
            int64_t position_tmp_19325 = loopres_19047;
            int64_t count_tmp_19326 = loopres_19048;
            
            loop_while_19041 = loop_while_tmp_19324;
            position_19042 = position_tmp_19325;
            count_19043 = count_tmp_19326;
        }
        count_general_res_19038 = loop_while_19041;
        count_general_res_19039 = position_19042;
        count_general_res_19040 = count_19043;
        count_res_18425 = count_general_res_19040;
    } else {
        // stdlib/regex.fut:95:60-97
        
        bool bounds_invalid_upwards_18445 = slt64(max_res_18424, (int64_t) 0);
        
        // stdlib/regex.fut:95:60-97
        
        bool valid_18446 = !bounds_invalid_upwards_18445;
        
        // stdlib/regex.fut:95:60-97
        
        bool range_valid_c_18447;
        
        if (!valid_18446) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Range ", (long long) (int64_t) 0, "..", (long long) (int64_t) 1, "..<", (long long) max_res_18424, " is invalid.", "-> #0  stdlib/regex.fut:95:60-97\n   #1  probes/stdlib-tests.fut:9:7-10:20\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        if (mem_19157_cached_sizze_19384 < max_res_18424) {
            err = lexical_realloc(ctx, &mem_19157, &mem_19157_cached_sizze_19384, max_res_18424);
            if (err != FUTHARK_SUCCESS)
                goto cleanup;
        }
        // stdlib/regex.fut:95:19-99
        for (int64_t i_19080 = 0; i_19080 < max_res_18424; i_19080++) {
            // stdlib/regex.fut:85:3-90:12
            
            bool fixed_match_res_18451;
            bool found_18453 = 0;
            
            for (int64_t branch_18452 = 0; branch_18452 < dzlz7bUZLzpZRz20Ubranchesz20U1z7dUzg_18354; branch_18452++) {
                // stdlib/regex.fut:86:5-90:12
                
                bool loopres_18454;
                
                if (found_18453) {
                    loopres_18454 = 1;
                } else {
                    int64_t first_18455 = ((int64_t *) mem_19095.mem)[branch_18452];
                    
                    // stdlib/regex.fut:88:21-89:63
                    
                    bool loopres_f_res_18457;
                    int64_t loopres_f_res_18458;
                    bool loopres_f_res_18459;
                    bool loop_while_18460;
                    int64_t i_18461;
                    bool ok_18462;
                    
                    loop_while_18460 = cond_18389;
                    i_18461 = (int64_t) 0;
                    ok_18462 = 1;
                    while (loop_while_18460) {
                        // stdlib/regex.fut:89:59-61
                        
                        int64_t accepts_arg1_18464 = add64(i_18461, i_19080);
                        
                        // stdlib/regex.fut:89:46-62
                        
                        bool x_18465 = sle64((int64_t) 0, accepts_arg1_18464);
                        
                        // stdlib/regex.fut:89:46-62
                        
                        bool y_18466 = slt64(accepts_arg1_18464, dz2081U_15586);
                        
                        // stdlib/regex.fut:89:46-62
                        
                        bool bounds_check_18467 = x_18465 && y_18466;
                        
                        // stdlib/regex.fut:89:46-62
                        
                        bool index_certs_18468;
                        
                        if (!bounds_check_18467) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) accepts_arg1_18464, "] out of bounds for array of shape [", (long long) dz2081U_15586, "].", "-> #0  stdlib/regex.fut:89:46-62\n   #1  stdlib/regex.fut:91:12-95:57\n   #2  stdlib/regex.fut:95:19-99\n   #3  probes/stdlib-tests.fut:9:7-10:20\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:89:46-62
                        
                        int8_t accepts_arg1_18469 = ((int8_t *) text_mem_19087.mem)[accepts_arg1_18464];
                        
                        // stdlib/regex.fut:89:42-44
                        
                        int64_t accepts_arg0_18470 = add64(first_18455, i_18461);
                        
                        // stdlib/regex.fut:89:32-45
                        
                        bool x_18471 = sle64((int64_t) 0, accepts_arg0_18470);
                        
                        // stdlib/regex.fut:89:32-45
                        
                        bool y_18472 = slt64(accepts_arg0_18470, compile_res_18211);
                        
                        // stdlib/regex.fut:89:32-45
                        
                        bool bounds_check_18473 = x_18471 && y_18472;
                        
                        // stdlib/regex.fut:89:32-45
                        
                        bool index_certs_18474;
                        
                        if (!bounds_check_18473) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) accepts_arg0_18470, "] out of bounds for array of shape [", (long long) compile_res_18211, "].", "-> #0  stdlib/regex.fut:89:32-45\n   #1  stdlib/regex.fut:91:12-95:57\n   #2  stdlib/regex.fut:95:19-99\n   #3  probes/stdlib-tests.fut:9:7-10:20\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:53:10-18
                        
                        int64_t u8_res_18585 = zext_i8_i64(accepts_arg1_18469);
                        
                        // stdlib/regex.fut:53:18-21
                        
                        int64_t zgzg_lhs_18586 = sdiv64(u8_res_18585, (int64_t) 64);
                        
                        // stdlib/regex.fut:53:5-22
                        
                        bool x_18587 = sle64((int64_t) 0, zgzg_lhs_18586);
                        
                        // stdlib/regex.fut:53:5-22
                        
                        bool y_18588 = slt64(zgzg_lhs_18586, (int64_t) 4);
                        
                        // stdlib/regex.fut:53:5-22
                        
                        bool bounds_check_18589 = x_18587 && y_18588;
                        
                        // stdlib/regex.fut:53:5-22
                        
                        bool index_certs_18590;
                        
                        if (!bounds_check_18589) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zgzg_lhs_18586, "] out of bounds for array of shape [", (long long) (int64_t) 4, "].", "-> #0  stdlib/regex.fut:53:5-22\n   #1  stdlib/regex.fut:89:24-62\n   #2  stdlib/regex.fut:91:12-95:57\n   #3  stdlib/regex.fut:95:19-99\n   #4  probes/stdlib-tests.fut:9:7-10:20\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:89:11-13
                        
                        int64_t tmp_18463 = add64((int64_t) 1, i_18461);
                        int64_t zgzg_lhs_18591 = ((int64_t *) mem_19091.mem)[accepts_arg0_18470 * (int64_t) 4 + zgzg_lhs_18586];
                        
                        // stdlib/regex.fut:53:35-38
                        
                        int8_t u8_arg0_18592 = umod8(accepts_arg1_18469, (int8_t) 64);
                        
                        // stdlib/regex.fut:53:26-38
                        
                        int64_t u8_res_18593 = zext_i8_i64(u8_arg0_18592);
                        
                        // stdlib/regex.fut:53:23-39
                        
                        int64_t za_lhs_18594 = lshr64(zgzg_lhs_18591, u8_res_18593);
                        
                        // stdlib/regex.fut:53:40-42
                        
                        int64_t znze_lhs_18595 = (int64_t) 1 & za_lhs_18594;
                        
                        // stdlib/regex.fut:53:44-48
                        
                        bool accepts_res_18596 = znze_lhs_18595 == (int64_t) 0;
                        
                        // stdlib/regex.fut:53:44-48
                        
                        bool accepts_res_18597 = !accepts_res_18596;
                        
                        // stdlib/regex.fut:88:53-68
                        
                        bool cond_18477 = slt64(tmp_18463, width_18388);
                        
                        // stdlib/regex.fut:91:12-95:57
                        
                        bool x_18478 = cond_18477 && accepts_res_18597;
                        bool loop_while_tmp_19329 = x_18478;
                        int64_t i_tmp_19330 = tmp_18463;
                        bool ok_tmp_19331 = accepts_res_18597;
                        
                        loop_while_18460 = loop_while_tmp_19329;
                        i_18461 = i_tmp_19330;
                        ok_18462 = ok_tmp_19331;
                    }
                    loopres_f_res_18457 = loop_while_18460;
                    loopres_f_res_18458 = i_18461;
                    loopres_f_res_18459 = ok_18462;
                    loopres_18454 = loopres_f_res_18459;
                }
                
                bool found_tmp_19328 = loopres_18454;
                
                found_18453 = found_tmp_19328;
            }
            fixed_match_res_18451 = found_18453;
            ((bool *) mem_19157)[i_19080] = fixed_match_res_18451;
        }
        // stdlib/regex.fut:96:22-97:60
        
        bool loop_cond_18480 = slt64((int64_t) 0, max_res_18424);
        
        // stdlib/regex.fut:96:22-97:60
        
        bool count_res_f_res_18481;
        int64_t count_res_f_res_18482;
        int64_t count_res_f_res_18483;
        bool loop_while_18484;
        int64_t i_18485;
        int64_t total_18486;
        
        loop_while_18484 = loop_cond_18480;
        i_18485 = (int64_t) 0;
        total_18486 = (int64_t) 0;
        while (loop_while_18484) {
            // stdlib/regex.fut:97:7-60
            
            bool x_18487 = sle64((int64_t) 0, i_18485);
            
            // stdlib/regex.fut:97:7-60
            
            bool y_18488 = slt64(i_18485, max_res_18424);
            
            // stdlib/regex.fut:97:7-60
            
            bool bounds_check_18489 = x_18487 && y_18488;
            
            // stdlib/regex.fut:97:7-60
            
            bool index_certs_18490;
            
            if (!bounds_check_18489) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_18485, "] out of bounds for array of shape [", (long long) max_res_18424, "].", "-> #0  stdlib/regex.fut:97:7-60\n   #1  probes/stdlib-tests.fut:9:7-10:20\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:97:7-60
            
            bool cond_18491 = ((bool *) mem_19157)[i_18485];
            
            // stdlib/regex.fut:97:7-60
            
            int64_t loopres_18492;
            int64_t loopres_18493;
            
            if (cond_18491) {
                // stdlib/regex.fut:97:28-34
                
                int64_t tmp_19055 = add64(width_18388, i_18485);
                
                // stdlib/regex.fut:97:40-42
                
                int64_t tmp_19056 = add64((int64_t) 1, total_18486);
                
                loopres_18492 = tmp_19055;
                loopres_18493 = tmp_19056;
            } else {
                // stdlib/regex.fut:97:51-53
                
                int64_t tmp_18496 = add64((int64_t) 1, i_18485);
                
                loopres_18492 = tmp_18496;
                loopres_18493 = total_18486;
            }
            // stdlib/regex.fut:96:59-75
            
            bool loop_cond_18497 = slt64(loopres_18492, max_res_18424);
            bool loop_while_tmp_19332 = loop_cond_18497;
            int64_t i_tmp_19333 = loopres_18492;
            int64_t total_tmp_19334 = loopres_18493;
            
            loop_while_18484 = loop_while_tmp_19332;
            i_18485 = i_tmp_19333;
            total_18486 = total_tmp_19334;
        }
        count_res_f_res_18481 = loop_while_18484;
        count_res_f_res_18482 = i_18485;
        count_res_f_res_18483 = total_18486;
        count_res_18425 = count_res_f_res_18483;
    }
    if (memblock_unref(ctx, &mem_19091, "mem_19091") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_19093, "mem_19093") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_19095, "mem_19095") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_19097, "mem_19097") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19289, &mem_19172, "mem_19172") != 0)
        return 1;
    prim_out_19290 = replace_res_18505;
    prim_out_19291 = count_res_18425;
    if (memblock_set(ctx, &*mem_out_p_19381, &mem_out_19289, "mem_out_19289") != 0)
        return 1;
    *out_prim_out_19382 = prim_out_19290;
    *out_prim_out_19383 = prim_out_19291;
    
  cleanup:
    {
        free(mem_19157);
        if (memblock_unref(ctx, &mem_19172, "mem_19172") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19112, "ext_mem_19112") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19111, "mem_19111") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19113, "mem_19113") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19114, "ext_mem_19114") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19131, "mem_19131") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19140, "ext_mem_19140") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19306, "mem_param_tmp_19306") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19310, "mem_param_tmp_19310") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19124, "ext_mem_19124") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19121, "mem_param_19121") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19127, "ext_mem_19127") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19118, "mem_param_19118") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19130, "ext_mem_19130") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19115, "mem_19115") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19143, "ext_mem_19143") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19097, "mem_19097") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19095, "mem_19095") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19093, "mem_19093") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19091, "mem_19091") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19289, "mem_out_19289") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_find_10646(struct futhark_context *ctx, int64_t *out_prim_out_19385, int64_t *out_prim_out_19386, struct memblock z30U_mem_19086, struct memblock z31U_mem_19087, struct memblock z32U_mem_19088, struct memblock z33U_mem_19089, struct memblock text_mem_19090, int64_t dz2080U_14728, int64_t dz2081U_14729, int64_t dz2082U_14730, int64_t dz2083U_14731, int64_t dz2080U_14732, int64_t start_14738)
{
    (void) ctx;
    
    int err = 0;
    int64_t prim_out_19289;
    int64_t prim_out_19290;
    
    // stdlib/regex.fut:67:24-76:56
    
    bool cond_14744 = sle64(start_14738, dz2080U_14732);
    
    // stdlib/regex.fut:67:24-76:56
    
    bool find_res_14747;
    int64_t find_res_14748;
    int64_t find_res_14749;
    bool loop_while_14750;
    int64_t position_14751;
    int64_t end_14752;
    
    loop_while_14750 = cond_14744;
    position_14751 = start_14738;
    end_14752 = (int64_t) -1;
    while (loop_while_14750) {
        // stdlib/regex.fut:69:17-75:82
        
        int64_t end_14755;
        int64_t result_14757 = (int64_t) -1;
        
        for (int64_t branch_14756 = 0; branch_14756 < dz2082U_14730; branch_14756++) {
            // stdlib/regex.fut:70:9-75:82
            
            bool cond_14758 = sle64((int64_t) 0, result_14757);
            
            // stdlib/regex.fut:70:9-75:82
            
            int64_t loopres_14759;
            
            if (cond_14758) {
                loopres_14759 = result_14757;
            } else {
                // stdlib/regex.fut:72:25-37
                
                bool y_14767 = slt64(branch_14756, dz2083U_14731);
                
                // stdlib/regex.fut:72:25-37
                
                bool index_certs_14769;
                
                if (!y_14767) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) branch_14756, "] out of bounds for array of shape [", (long long) dz2083U_14731, "].", "-> #0  stdlib/regex.fut:72:25-37\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:71:26-38
                
                int64_t first_14764 = ((int64_t *) z32U_mem_19088.mem)[branch_14756];
                
                // stdlib/regex.fut:72:25-37
                
                int64_t last_14770 = ((int64_t *) z33U_mem_19089.mem)[branch_14756];
                
                // stdlib/regex.fut:73:29-74:78
                
                bool cond_14772 = first_14764 == last_14770;
                
                // stdlib/regex.fut:73:29-74:78
                
                bool cond_14773;
                
                if (cond_14772) {
                    cond_14773 = 1;
                } else {
                    // stdlib/regex.fut:73:46-57
                    
                    bool x_14774 = sle64((int64_t) 0, first_14764);
                    
                    // stdlib/regex.fut:73:46-57
                    
                    bool y_14775 = slt64(first_14764, dz2081U_14729);
                    
                    // stdlib/regex.fut:73:46-57
                    
                    bool bounds_check_14776 = x_14774 && y_14775;
                    
                    // stdlib/regex.fut:73:46-57
                    
                    bool index_certs_14777;
                    
                    if (!bounds_check_14776) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) first_14764, "] out of bounds for array of shape [", (long long) dz2081U_14729, "].", "-> #0  stdlib/regex.fut:73:46-57\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:73:46-57
                    
                    int64_t zeze_lhs_14778 = ((int64_t *) z31U_mem_19087.mem)[first_14764];
                    
                    // stdlib/regex.fut:73:58-62
                    
                    bool cond_f_res_14780 = zeze_lhs_14778 == (int64_t) 0;
                    
                    cond_14773 = cond_f_res_14780;
                }
                // stdlib/regex.fut:73:29-74:78
                
                bool cond_14781;
                
                if (cond_14773) {
                    cond_14781 = 1;
                } else {
                    // stdlib/regex.fut:73:66-77
                    
                    bool x_14782 = sle64((int64_t) 0, first_14764);
                    
                    // stdlib/regex.fut:73:66-77
                    
                    bool y_14783 = slt64(first_14764, dz2081U_14729);
                    
                    // stdlib/regex.fut:73:66-77
                    
                    bool bounds_check_14784 = x_14782 && y_14783;
                    
                    // stdlib/regex.fut:73:66-77
                    
                    bool index_certs_14785;
                    
                    if (!bounds_check_14784) {
                        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) first_14764, "] out of bounds for array of shape [", (long long) dz2081U_14729, "].", "-> #0  stdlib/regex.fut:73:66-77\n"));
                        err = FUTHARK_PROGRAM_ERROR;
                        goto cleanup;
                    }
                    // stdlib/regex.fut:73:66-77
                    
                    int64_t zeze_lhs_14786 = ((int64_t *) z31U_mem_19087.mem)[first_14764];
                    
                    // stdlib/regex.fut:73:78-82
                    
                    bool cond_f_res_14788 = zeze_lhs_14786 == (int64_t) 3;
                    
                    cond_14781 = cond_f_res_14788;
                }
                // stdlib/regex.fut:73:29-74:78
                
                bool possible_14789;
                
                if (cond_14781) {
                    possible_14789 = 1;
                } else {
                    // stdlib/regex.fut:74:17-77
                    
                    bool cond_14792 = slt64(position_14751, dz2080U_14732);
                    
                    // stdlib/regex.fut:74:17-77
                    
                    bool possible_f_res_14793;
                    
                    if (cond_14792) {
                        // stdlib/regex.fut:74:63-77
                        
                        bool x_18677 = sle64((int64_t) 0, position_14751);
                        
                        // stdlib/regex.fut:74:63-77
                        
                        bool bounds_check_18678 = cond_14792 && x_18677;
                        
                        // stdlib/regex.fut:74:63-77
                        
                        bool index_certs_18679;
                        
                        if (!bounds_check_18678) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) position_14751, "] out of bounds for array of shape [", (long long) dz2080U_14732, "].", "-> #0  stdlib/regex.fut:74:63-77\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:74:63-77
                        
                        int8_t accepts_arg1_18680 = ((int8_t *) text_mem_19090.mem)[position_14751];
                        
                        // stdlib/regex.fut:74:51-62
                        
                        bool x_18681 = sle64((int64_t) 0, first_14764);
                        
                        // stdlib/regex.fut:74:51-62
                        
                        bool y_18682 = slt64(first_14764, dz2080U_14728);
                        
                        // stdlib/regex.fut:74:51-62
                        
                        bool bounds_check_18683 = x_18681 && y_18682;
                        
                        // stdlib/regex.fut:74:51-62
                        
                        bool index_certs_18684;
                        
                        if (!bounds_check_18683) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) first_14764, "] out of bounds for array of shape [", (long long) dz2080U_14728, "].", "-> #0  stdlib/regex.fut:74:51-62\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        // stdlib/regex.fut:53:10-18
                        
                        int64_t u8_res_18685 = zext_i8_i64(accepts_arg1_18680);
                        
                        // stdlib/regex.fut:53:18-21
                        
                        int64_t zgzg_lhs_18686 = sdiv64(u8_res_18685, (int64_t) 64);
                        
                        // stdlib/regex.fut:53:5-22
                        
                        bool x_18687 = sle64((int64_t) 0, zgzg_lhs_18686);
                        
                        // stdlib/regex.fut:53:5-22
                        
                        bool y_18688 = slt64(zgzg_lhs_18686, (int64_t) 4);
                        
                        // stdlib/regex.fut:53:5-22
                        
                        bool bounds_check_18689 = x_18687 && y_18688;
                        
                        // stdlib/regex.fut:53:5-22
                        
                        bool index_certs_18690;
                        
                        if (!bounds_check_18689) {
                            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zgzg_lhs_18686, "] out of bounds for array of shape [", (long long) (int64_t) 4, "].", "-> #0  stdlib/regex.fut:53:5-22\n   #1  stdlib/regex.fut:74:43-77\n"));
                            err = FUTHARK_PROGRAM_ERROR;
                            goto cleanup;
                        }
                        
                        int64_t zgzg_lhs_18691 = ((int64_t *) z30U_mem_19086.mem)[first_14764 * (int64_t) 4 + zgzg_lhs_18686];
                        
                        // stdlib/regex.fut:53:35-38
                        
                        int8_t u8_arg0_18692 = umod8(accepts_arg1_18680, (int8_t) 64);
                        
                        // stdlib/regex.fut:53:26-38
                        
                        int64_t u8_res_18693 = zext_i8_i64(u8_arg0_18692);
                        
                        // stdlib/regex.fut:53:23-39
                        
                        int64_t za_lhs_18694 = lshr64(zgzg_lhs_18691, u8_res_18693);
                        
                        // stdlib/regex.fut:53:40-42
                        
                        int64_t znze_lhs_18695 = (int64_t) 1 & za_lhs_18694;
                        
                        // stdlib/regex.fut:53:44-48
                        
                        bool accepts_res_18696 = znze_lhs_18695 == (int64_t) 0;
                        
                        // stdlib/regex.fut:53:44-48
                        
                        bool accepts_res_18697 = !accepts_res_18696;
                        
                        possible_f_res_14793 = accepts_res_18697;
                    } else {
                        possible_f_res_14793 = 0;
                    }
                    possible_14789 = possible_f_res_14793;
                }
                // stdlib/regex.fut:75:17-82
                
                int64_t loopres_f_res_14808;
                
                if (possible_14789) {
                    // stdlib/regex.fut:66:11-75:74
                    
                    int64_t loopres_f_res_t_res_18698;
                    
                    if (futrts_match_branch_10647(ctx, &loopres_f_res_t_res_18698, z30U_mem_19086, z31U_mem_19087, z32U_mem_19088, z33U_mem_19089, text_mem_19090, dz2080U_14728, dz2081U_14729, dz2082U_14730, dz2083U_14731, dz2080U_14732, first_14764, last_14770, position_14751) != 0) {
                        err = 1;
                        goto cleanup;
                    }
                    loopres_f_res_14808 = loopres_f_res_t_res_18698;
                } else {
                    loopres_f_res_14808 = (int64_t) -1;
                }
                loopres_14759 = loopres_f_res_14808;
            }
            
            int64_t result_tmp_19294 = loopres_14759;
            
            result_14757 = result_tmp_19294;
        }
        end_14755 = result_14757;
        // stdlib/regex.fut:76:11-51
        
        bool cond_14812 = slt64(end_14755, (int64_t) 0);
        
        // stdlib/regex.fut:76:11-51
        
        int64_t tmp_14813;
        
        if (cond_14812) {
            // stdlib/regex.fut:76:35-37
            
            int64_t tmp_t_res_18699 = add64((int64_t) 1, position_14751);
            
            tmp_14813 = tmp_t_res_18699;
        } else {
            tmp_14813 = position_14751;
        }
        // stdlib/regex.fut:68:11-45
        
        bool cond_14820 = sle64(tmp_14813, dz2080U_14732);
        bool x_16037 = cond_14812 && cond_14820;
        bool loop_while_tmp_19291 = x_16037;
        int64_t position_tmp_19292 = tmp_14813;
        int64_t end_tmp_19293 = end_14755;
        
        loop_while_14750 = loop_while_tmp_19291;
        position_14751 = position_tmp_19292;
        end_14752 = end_tmp_19293;
    }
    find_res_14747 = loop_while_14750;
    find_res_14748 = position_14751;
    find_res_14749 = end_14752;
    prim_out_19289 = find_res_14748;
    prim_out_19290 = find_res_14749;
    *out_prim_out_19385 = prim_out_19289;
    *out_prim_out_19386 = prim_out_19290;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_grow_10668(struct futhark_context *ctx, struct memblock *mem_out_p_19387, struct memblock *mem_out_p_19388, struct memblock *mem_out_p_19389, int64_t *out_prim_out_19390, int64_t *out_prim_out_19391, struct memblock z30U_mem_19086, struct memblock z30U_mem_19087, struct memblock z30U_mem_19088, int64_t dz2080U_15819, int64_t z31U_15823)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_param_tmp_19299;
    
    mem_param_tmp_19299.references = NULL;
    
    struct memblock mem_param_tmp_19298;
    
    mem_param_tmp_19298.references = NULL;
    
    struct memblock mem_param_tmp_19297;
    
    mem_param_tmp_19297.references = NULL;
    
    struct memblock ext_mem_19109;
    
    ext_mem_19109.references = NULL;
    
    struct memblock ext_mem_19110;
    
    ext_mem_19110.references = NULL;
    
    struct memblock ext_mem_19111;
    
    ext_mem_19111.references = NULL;
    
    struct memblock ext_mem_19114;
    
    ext_mem_19114.references = NULL;
    
    struct memblock ext_mem_19117;
    
    ext_mem_19117.references = NULL;
    
    struct memblock ext_mem_19120;
    
    ext_mem_19120.references = NULL;
    
    struct memblock mem_param_19102;
    
    mem_param_19102.references = NULL;
    
    struct memblock mem_param_19099;
    
    mem_param_19099.references = NULL;
    
    struct memblock mem_param_19096;
    
    mem_param_19096.references = NULL;
    
    struct memblock ext_mem_19127;
    
    ext_mem_19127.references = NULL;
    
    struct memblock ext_mem_19128;
    
    ext_mem_19128.references = NULL;
    
    struct memblock ext_mem_19129;
    
    ext_mem_19129.references = NULL;
    
    struct memblock mem_19093;
    
    mem_19093.references = NULL;
    
    struct memblock mem_19092;
    
    mem_19092.references = NULL;
    
    struct memblock mem_19090;
    
    mem_19090.references = NULL;
    
    struct memblock mem_out_19291;
    
    mem_out_19291.references = NULL;
    
    struct memblock mem_out_19290;
    
    mem_out_19290.references = NULL;
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    int64_t prim_out_19292;
    int64_t prim_out_19293;
    
    // stdlib/hashmap.fut:25:26-40
    
    int64_t dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_15829 = mul64((int64_t) 2, dz2080U_15819);
    int64_t bytes_19089 = (int64_t) 16 * dz2080U_15819;
    
    if (memblock_alloc(ctx, &mem_19090, bytes_19089, "mem_19090")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:25:6-26:73
    for (int64_t nest_i_19294 = 0; nest_i_19294 < dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_15829; nest_i_19294++) {
        ((int64_t *) mem_19090.mem)[nest_i_19294] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_19092, bytes_19089, "mem_19092")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:25:6-26:73
    for (int64_t nest_i_19295 = 0; nest_i_19295 < dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_15829; nest_i_19295++) {
        ((int64_t *) mem_19092.mem)[nest_i_19295] = (int64_t) 0;
    }
    if (memblock_alloc(ctx, &mem_19093, dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_15829, "mem_19093")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/hashmap.fut:25:6-26:73
    for (int64_t nest_i_19296 = 0; nest_i_19296 < dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_15829; nest_i_19296++) {
        ((bool *) mem_19093.mem)[nest_i_19296] = 0;
    }
    // stdlib/hashmap.fut:25:6-26:73
    
    int64_t grow_res_15856;
    int64_t fresh_15861;
    
    if (memblock_set(ctx, &mem_param_19096, &mem_19090, "mem_19090") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_19099, &mem_19092, "mem_19092") != 0)
        return 1;
    if (memblock_set(ctx, &mem_param_19102, &mem_19093, "mem_19093") != 0)
        return 1;
    fresh_15861 = (int64_t) 0;
    for (int64_t i_15857 = 0; i_15857 < dz2080U_15819; i_15857++) {
        // stdlib/hashmap.fut:26:11-19
        
        bool loopres_15868 = ((bool *) z30U_mem_19088.mem)[i_15857];
        
        // stdlib/hashmap.fut:26:52-60
        
        int64_t loopres_t_res_18677;
        
        if (loopres_15868) {
            int64_t x_19268 = ((int64_t *) z30U_mem_19087.mem)[i_15857];
            
            loopres_t_res_18677 = x_19268;
        } else {
            loopres_t_res_18677 = (int64_t) 0;
        }
        // stdlib/hashmap.fut:26:41-49
        
        int64_t loopres_t_res_18678;
        
        if (loopres_15868) {
            int64_t x_19270 = ((int64_t *) z30U_mem_19086.mem)[i_15857];
            
            loopres_t_res_18678 = x_19270;
        } else {
            loopres_t_res_18678 = (int64_t) 0;
        }
        
        int64_t loopres_15875;
        
        if (loopres_15868) {
            // stdlib/hashmap.fut:26:27-62
            
            int64_t loopres_t_res_18682;
            
            if (futrts_put_raw_10670(ctx, &ext_mem_19111, &ext_mem_19110, &ext_mem_19109, &loopres_t_res_18682, mem_param_19096, mem_param_19099, mem_param_19102, dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_15829, fresh_15861, loopres_t_res_18678, loopres_t_res_18677) != 0) {
                err = 1;
                goto cleanup;
            }
            if (memblock_set(ctx, &ext_mem_19120, &ext_mem_19111, "ext_mem_19111") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19117, &ext_mem_19110, "ext_mem_19110") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19114, &ext_mem_19109, "ext_mem_19109") != 0)
                return 1;
            loopres_15875 = loopres_t_res_18682;
        } else {
            if (memblock_set(ctx, &ext_mem_19120, &mem_param_19096, "mem_param_19096") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19117, &mem_param_19099, "mem_param_19099") != 0)
                return 1;
            if (memblock_set(ctx, &ext_mem_19114, &mem_param_19102, "mem_param_19102") != 0)
                return 1;
            loopres_15875 = fresh_15861;
        }
        if (memblock_set(ctx, &mem_param_tmp_19297, &ext_mem_19120, "ext_mem_19120") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_19298, &ext_mem_19117, "ext_mem_19117") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_tmp_19299, &ext_mem_19114, "ext_mem_19114") != 0)
            return 1;
        
        int64_t fresh_tmp_19303 = loopres_15875;
        
        if (memblock_set(ctx, &mem_param_19096, &mem_param_tmp_19297, "mem_param_tmp_19297") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_19099, &mem_param_tmp_19298, "mem_param_tmp_19298") != 0)
            return 1;
        if (memblock_set(ctx, &mem_param_19102, &mem_param_tmp_19299, "mem_param_tmp_19299") != 0)
            return 1;
        fresh_15861 = fresh_tmp_19303;
    }
    if (memblock_set(ctx, &ext_mem_19129, &mem_param_19096, "mem_param_19096") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_19128, &mem_param_19099, "mem_param_19099") != 0)
        return 1;
    if (memblock_set(ctx, &ext_mem_19127, &mem_param_19102, "mem_param_19102") != 0)
        return 1;
    grow_res_15856 = fresh_15861;
    if (memblock_unref(ctx, &mem_19090, "mem_19090") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_19092, "mem_19092") != 0)
        return 1;
    if (memblock_unref(ctx, &mem_19093, "mem_19093") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19289, &ext_mem_19129, "ext_mem_19129") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19290, &ext_mem_19128, "ext_mem_19128") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19291, &ext_mem_19127, "ext_mem_19127") != 0)
        return 1;
    prim_out_19292 = dzlz7bUZLztZRz20UZLlengthz20UslotsZRz20U2z7dUzg_15829;
    prim_out_19293 = grow_res_15856;
    if (memblock_set(ctx, &*mem_out_p_19387, &mem_out_19289, "mem_out_19289") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19388, &mem_out_19290, "mem_out_19290") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19389, &mem_out_19291, "mem_out_19291") != 0)
        return 1;
    *out_prim_out_19390 = prim_out_19292;
    *out_prim_out_19391 = prim_out_19293;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_param_tmp_19299, "mem_param_tmp_19299") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19298, "mem_param_tmp_19298") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_tmp_19297, "mem_param_tmp_19297") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19109, "ext_mem_19109") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19110, "ext_mem_19110") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19111, "ext_mem_19111") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19114, "ext_mem_19114") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19117, "ext_mem_19117") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19120, "ext_mem_19120") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19102, "mem_param_19102") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19099, "mem_param_19099") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_param_19096, "mem_param_19096") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19127, "ext_mem_19127") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19128, "ext_mem_19128") != 0)
            return 1;
        if (memblock_unref(ctx, &ext_mem_19129, "ext_mem_19129") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19093, "mem_19093") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19092, "mem_19092") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19090, "mem_19090") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19291, "mem_out_19291") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19290, "mem_out_19290") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19289, "mem_out_19289") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_literal_10630(struct futhark_context *ctx, int8_t *out_prim_out_19392, int64_t *out_prim_out_19393, struct memblock pattern_mem_19086, int64_t dz2080U_14038, int64_t p_14040)
{
    (void) ctx;
    
    int err = 0;
    int8_t prim_out_19289;
    int64_t prim_out_19290;
    
    // stdlib/regex.fut:8:6-16
    
    bool x_14041 = sle64((int64_t) 0, p_14040);
    
    // stdlib/regex.fut:8:6-16
    
    bool y_14042 = slt64(p_14040, dz2080U_14038);
    
    // stdlib/regex.fut:8:6-16
    
    bool bounds_check_14043 = x_14041 && y_14042;
    
    // stdlib/regex.fut:8:6-16
    
    bool index_certs_14044;
    
    if (!bounds_check_14043) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) p_14040, "] out of bounds for array of shape [", (long long) dz2080U_14038, "].", "-> #0  stdlib/regex.fut:8:6-16\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:8:6-16
    
    int8_t zeze_lhs_14045 = ((int8_t *) pattern_mem_19086.mem)[p_14040];
    
    // stdlib/regex.fut:8:3-9:24
    
    bool cond_14047 = zeze_lhs_14045 == (int8_t) 92;
    
    // stdlib/regex.fut:8:27-29
    
    int64_t zl_lhs_14049 = add64((int64_t) 1, p_14040);
    
    // stdlib/regex.fut:8:30-46
    
    bool cond_t_res_14053 = slt64(zl_lhs_14049, dz2080U_14038);
    bool x_18580 = cond_14047 && cond_t_res_14053;
    
    // stdlib/regex.fut:8:3-9:24
    
    int8_t literal_res_14054;
    int64_t literal_res_14055;
    
    if (x_18580) {
        // stdlib/regex.fut:8:61-73
        
        bool x_18677 = sle64((int64_t) 0, zl_lhs_14049);
        
        // stdlib/regex.fut:8:61-73
        
        bool bounds_check_18678 = cond_t_res_14053 && x_18677;
        
        // stdlib/regex.fut:8:61-73
        
        bool index_certs_18679;
        
        if (!bounds_check_18678) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zl_lhs_14049, "] out of bounds for array of shape [", (long long) dz2080U_14038, "].", "-> #0  stdlib/regex.fut:8:61-73\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:8:61-73
        
        int8_t escaped_arg0_18680 = ((int8_t *) pattern_mem_19086.mem)[zl_lhs_14049];
        
        // stdlib/regex.fut:6:25-100
        
        bool cond_18681 = escaped_arg0_18680 == (int8_t) 110;
        
        // stdlib/regex.fut:6:25-100
        
        int8_t escaped_res_18682;
        
        if (cond_18681) {
            escaped_res_18682 = (int8_t) 10;
        } else {
            // stdlib/regex.fut:6:50-100
            
            bool cond_18683 = escaped_arg0_18680 == (int8_t) 114;
            
            // stdlib/regex.fut:6:50-100
            
            int8_t escaped_res_f_res_18684;
            
            if (cond_18683) {
                escaped_res_f_res_18684 = (int8_t) 13;
            } else {
                // stdlib/regex.fut:6:75-100
                
                bool cond_18685 = escaped_arg0_18680 == (int8_t) 116;
                
                // stdlib/regex.fut:6:75-100
                
                int8_t escaped_res_f_res_f_res_18686;
                
                if (cond_18685) {
                    escaped_res_f_res_f_res_18686 = (int8_t) 9;
                } else {
                    escaped_res_f_res_f_res_18686 = escaped_arg0_18680;
                }
                escaped_res_f_res_18684 = escaped_res_f_res_f_res_18686;
            }
            escaped_res_18682 = escaped_res_f_res_18684;
        }
        // stdlib/regex.fut:8:75-77
        
        int64_t tmp_18687 = add64((int64_t) 2, p_14040);
        
        literal_res_14054 = escaped_res_18682;
        literal_res_14055 = tmp_18687;
    } else {
        literal_res_14054 = zeze_lhs_14045;
        literal_res_14055 = zl_lhs_14049;
    }
    prim_out_19289 = literal_res_14054;
    prim_out_19290 = literal_res_14055;
    *out_prim_out_19392 = prim_out_19289;
    *out_prim_out_19393 = prim_out_19290;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_mask_set_8060(struct futhark_context *ctx, struct memblock *mem_out_p_19394, struct memblock mask_mem_19086, int8_t c_14091)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    // stdlib/regex.fut:11:14-22
    
    int64_t u8_res_16064 = zext_i8_i64(c_14091);
    
    // stdlib/regex.fut:11:22-25
    
    int64_t word_14097 = sdiv64(u8_res_16064, (int64_t) 64);
    
    // stdlib/regex.fut:12:20-30
    
    bool x_14099 = sle64((int64_t) 0, word_14097);
    
    // stdlib/regex.fut:12:20-30
    
    bool y_14100 = slt64(word_14097, (int64_t) 4);
    
    // stdlib/regex.fut:12:20-30
    
    bool bounds_check_14101 = x_14099 && y_14100;
    
    // stdlib/regex.fut:12:20-30
    
    bool index_certs_14102;
    
    if (!bounds_check_14101) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) word_14097, "] out of bounds for array of shape [", (long long) (int64_t) 4, "].", "-> #0  stdlib/regex.fut:12:20-30\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/regex.fut:12:20-30
    
    int64_t zb_lhs_14103 = ((int64_t *) mask_mem_19086.mem)[word_14097];
    
    // stdlib/regex.fut:12:51-54
    
    int8_t u8_arg0_14108 = umod8(c_14091, (int8_t) 64);
    
    // stdlib/regex.fut:12:42-54
    
    int64_t u8_res_16188 = zext_i8_i64(u8_arg0_14108);
    
    // stdlib/regex.fut:12:39-55
    
    int64_t zb_rhs_14112 = shl64((int64_t) 1, u8_res_16188);
    
    // stdlib/regex.fut:12:31-55
    
    int64_t tmp_14114 = zb_lhs_14103 | zb_rhs_14112;
    
    // stdlib/regex.fut:12:7-56
    ((int64_t *) mask_mem_19086.mem)[word_14097] = tmp_14114;
    if (memblock_set(ctx, &mem_out_19289, &mask_mem_19086, "mask_mem_19086") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19394, &mem_out_19289, "mem_out_19289") != 0)
        return 1;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_19289, "mem_out_19289") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_match_branch_10647(struct futhark_context *ctx, int64_t *out_prim_out_19395, struct memblock z30U_mem_19086, struct memblock z31U_mem_19087, struct memblock z32U_mem_19088, struct memblock z33U_mem_19089, struct memblock text_mem_19090, int64_t dz2080U_14624, int64_t dz2081U_14625, int64_t dz2082U_14626, int64_t dz2083U_14627, int64_t dz2080U_14628, int64_t token_14633, int64_t last_14634, int64_t position_14636)
{
    (void) ctx;
    
    int err = 0;
    int64_t prim_out_19289;
    
    // stdlib/regex.fut:55:3-65:14
    
    bool cond_14637 = token_14633 == last_14634;
    
    // stdlib/regex.fut:55:3-65:14
    
    int64_t match_branch_res_14638;
    
    if (cond_14637) {
        match_branch_res_14638 = position_14636;
    } else {
        // stdlib/regex.fut:57:16-27
        
        bool x_14639 = sle64((int64_t) 0, token_14633);
        
        // stdlib/regex.fut:57:16-27
        
        bool y_14640 = slt64(token_14633, dz2081U_14625);
        
        // stdlib/regex.fut:57:16-27
        
        bool bounds_check_14641 = x_14639 && y_14640;
        
        // stdlib/regex.fut:57:16-27
        
        bool index_certs_14642;
        
        if (!bounds_check_14641) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) token_14633, "] out of bounds for array of shape [", (long long) dz2081U_14625, "].", "-> #0  stdlib/regex.fut:57:16-27\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/regex.fut:57:16-27
        
        int64_t kind_14643 = ((int64_t *) z31U_mem_19087.mem)[token_14633];
        
        // stdlib/regex.fut:58:19-58
        
        bool cond_14645 = kind_14643 == (int64_t) 0;
        
        // stdlib/regex.fut:58:40-44
        
        bool cond_f_res_14647 = kind_14643 == (int64_t) 3;
        bool x_16035 = !cond_14645;
        bool y_16036 = cond_f_res_14647 && x_16035;
        bool cond_14646 = cond_14645 || y_16036;
        bool cond_neg_16037 = !cond_14646;
        int64_t minimum_14648 = btoi_bool_i64(cond_neg_16037);
        
        // stdlib/regex.fut:59:17-99
        
        bool cond_14650 = kind_14643 == (int64_t) 1;
        bool x_16039 = !cond_14650;
        bool y_16040 = cond_f_res_14647 && x_16039;
        bool cond_14651 = cond_14650 || y_16040;
        
        // stdlib/regex.fut:59:17-99
        
        int64_t limit_14653;
        
        if (cond_14651) {
            // stdlib/regex.fut:59:65-67
            
            int64_t min_arg0_18677 = add64((int64_t) 1, position_14636);
            
            // stdlib/regex.fut:59:48-81
            
            int64_t min_res_18678 = smin64(dz2080U_14628, min_arg0_18677);
            
            limit_14653 = min_res_18678;
        } else {
            limit_14653 = dz2080U_14628;
        }
        // stdlib/regex.fut:60:15-94
        
        bool cond_14662 = slt64(position_14636, limit_14653);
        
        // stdlib/regex.fut:60:15-94
        
        bool loop_cond_14663;
        
        if (cond_14662) {
            // stdlib/regex.fut:60:76-85
            
            bool x_18679 = sle64((int64_t) 0, position_14636);
            
            // stdlib/regex.fut:60:76-85
            
            bool y_18680 = slt64(position_14636, dz2080U_14628);
            
            // stdlib/regex.fut:60:76-85
            
            bool bounds_check_18681 = x_18679 && y_18680;
            
            // stdlib/regex.fut:60:76-85
            
            bool index_certs_18682;
            
            if (!bounds_check_18681) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) position_14636, "] out of bounds for array of shape [", (long long) dz2080U_14628, "].", "-> #0  stdlib/regex.fut:60:76-85\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:60:76-85
            
            int8_t accepts_arg1_18683 = ((int8_t *) text_mem_19090.mem)[position_14636];
            
            // stdlib/regex.fut:60:64-75
            
            bool y_18684 = slt64(token_14633, dz2080U_14624);
            
            // stdlib/regex.fut:60:64-75
            
            bool bounds_check_18685 = x_14639 && y_18684;
            
            // stdlib/regex.fut:60:64-75
            
            bool index_certs_18686;
            
            if (!bounds_check_18685) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) token_14633, "] out of bounds for array of shape [", (long long) dz2080U_14624, "].", "-> #0  stdlib/regex.fut:60:64-75\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/regex.fut:53:10-18
            
            int64_t u8_res_18687 = zext_i8_i64(accepts_arg1_18683);
            
            // stdlib/regex.fut:53:18-21
            
            int64_t zgzg_lhs_18688 = sdiv64(u8_res_18687, (int64_t) 64);
            
            // stdlib/regex.fut:53:5-22
            
            bool x_18689 = sle64((int64_t) 0, zgzg_lhs_18688);
            
            // stdlib/regex.fut:53:5-22
            
            bool y_18690 = slt64(zgzg_lhs_18688, (int64_t) 4);
            
            // stdlib/regex.fut:53:5-22
            
            bool bounds_check_18691 = x_18689 && y_18690;
            
            // stdlib/regex.fut:53:5-22
            
            bool index_certs_18692;
            
            if (!bounds_check_18691) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zgzg_lhs_18688, "] out of bounds for array of shape [", (long long) (int64_t) 4, "].", "-> #0  stdlib/regex.fut:53:5-22\n   #1  stdlib/regex.fut:60:56-85\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            
            int64_t zgzg_lhs_18693 = ((int64_t *) z30U_mem_19086.mem)[token_14633 * (int64_t) 4 + zgzg_lhs_18688];
            
            // stdlib/regex.fut:53:35-38
            
            int8_t u8_arg0_18694 = umod8(accepts_arg1_18683, (int8_t) 64);
            
            // stdlib/regex.fut:53:26-38
            
            int64_t u8_res_18695 = zext_i8_i64(u8_arg0_18694);
            
            // stdlib/regex.fut:53:23-39
            
            int64_t za_lhs_18696 = lshr64(zgzg_lhs_18693, u8_res_18695);
            
            // stdlib/regex.fut:53:40-42
            
            int64_t znze_lhs_18697 = (int64_t) 1 & za_lhs_18696;
            
            // stdlib/regex.fut:53:44-48
            
            bool accepts_res_18698 = znze_lhs_18697 == (int64_t) 0;
            
            // stdlib/regex.fut:53:44-48
            
            bool accepts_res_18699 = !accepts_res_18698;
            
            loop_cond_14663 = accepts_res_18699;
        } else {
            loop_cond_14663 = 0;
        }
        // stdlib/regex.fut:60:15-94
        
        bool end_14677;
        int64_t end_14678;
        bool loop_while_14679;
        int64_t end_14680;
        
        loop_while_14679 = loop_cond_14663;
        end_14680 = position_14636;
        while (loop_while_14679) {
            // stdlib/regex.fut:60:92-94
            
            int64_t loopres_14681 = add64((int64_t) 1, end_14680);
            
            // stdlib/regex.fut:60:41-85
            
            bool cond_14683 = slt64(loopres_14681, limit_14653);
            
            // stdlib/regex.fut:60:41-85
            
            bool loop_cond_14684;
            
            if (cond_14683) {
                // stdlib/regex.fut:60:76-85
                
                bool x_18700 = sle64((int64_t) 0, loopres_14681);
                
                // stdlib/regex.fut:60:76-85
                
                bool y_18701 = slt64(loopres_14681, dz2080U_14628);
                
                // stdlib/regex.fut:60:76-85
                
                bool bounds_check_18702 = x_18700 && y_18701;
                
                // stdlib/regex.fut:60:76-85
                
                bool index_certs_18703;
                
                if (!bounds_check_18702) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_14681, "] out of bounds for array of shape [", (long long) dz2080U_14628, "].", "-> #0  stdlib/regex.fut:60:76-85\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:60:76-85
                
                int8_t accepts_arg1_18704 = ((int8_t *) text_mem_19090.mem)[loopres_14681];
                
                // stdlib/regex.fut:60:64-75
                
                bool y_18705 = slt64(token_14633, dz2080U_14624);
                
                // stdlib/regex.fut:60:64-75
                
                bool bounds_check_18706 = x_14639 && y_18705;
                
                // stdlib/regex.fut:60:64-75
                
                bool index_certs_18707;
                
                if (!bounds_check_18706) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) token_14633, "] out of bounds for array of shape [", (long long) dz2080U_14624, "].", "-> #0  stdlib/regex.fut:60:64-75\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                // stdlib/regex.fut:53:10-18
                
                int64_t u8_res_18708 = zext_i8_i64(accepts_arg1_18704);
                
                // stdlib/regex.fut:53:18-21
                
                int64_t zgzg_lhs_18709 = sdiv64(u8_res_18708, (int64_t) 64);
                
                // stdlib/regex.fut:53:5-22
                
                bool x_18710 = sle64((int64_t) 0, zgzg_lhs_18709);
                
                // stdlib/regex.fut:53:5-22
                
                bool y_18711 = slt64(zgzg_lhs_18709, (int64_t) 4);
                
                // stdlib/regex.fut:53:5-22
                
                bool bounds_check_18712 = x_18710 && y_18711;
                
                // stdlib/regex.fut:53:5-22
                
                bool index_certs_18713;
                
                if (!bounds_check_18712) {
                    set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zgzg_lhs_18709, "] out of bounds for array of shape [", (long long) (int64_t) 4, "].", "-> #0  stdlib/regex.fut:53:5-22\n   #1  stdlib/regex.fut:60:56-85\n"));
                    err = FUTHARK_PROGRAM_ERROR;
                    goto cleanup;
                }
                
                int64_t zgzg_lhs_18714 = ((int64_t *) z30U_mem_19086.mem)[token_14633 * (int64_t) 4 + zgzg_lhs_18709];
                
                // stdlib/regex.fut:53:35-38
                
                int8_t u8_arg0_18715 = umod8(accepts_arg1_18704, (int8_t) 64);
                
                // stdlib/regex.fut:53:26-38
                
                int64_t u8_res_18716 = zext_i8_i64(u8_arg0_18715);
                
                // stdlib/regex.fut:53:23-39
                
                int64_t za_lhs_18717 = lshr64(zgzg_lhs_18714, u8_res_18716);
                
                // stdlib/regex.fut:53:40-42
                
                int64_t znze_lhs_18718 = (int64_t) 1 & za_lhs_18717;
                
                // stdlib/regex.fut:53:44-48
                
                bool accepts_res_18719 = znze_lhs_18718 == (int64_t) 0;
                
                // stdlib/regex.fut:53:44-48
                
                bool accepts_res_18720 = !accepts_res_18719;
                
                loop_cond_14684 = accepts_res_18720;
            } else {
                loop_cond_14684 = 0;
            }
            
            bool loop_while_tmp_19290 = loop_cond_14684;
            int64_t end_tmp_19291 = loopres_14681;
            
            loop_while_14679 = loop_while_tmp_19290;
            end_14680 = end_tmp_19291;
        }
        end_14677 = loop_while_14679;
        end_14678 = end_14680;
        // stdlib/regex.fut:61:27-64:28
        
        int64_t zgze_rhs_14702 = add64(position_14636, minimum_14648);
        
        // stdlib/regex.fut:61:27-64:28
        
        bool cond_14704 = sle64(zgze_rhs_14702, end_14678);
        
        // stdlib/regex.fut:63:44-46
        
        int64_t match_branch_arg1_14713 = add64((int64_t) 1, token_14633);
        
        // stdlib/regex.fut:61:27-64:28
        
        bool match_branch_res_f_res_14707;
        int64_t match_branch_res_f_res_14708;
        int64_t match_branch_res_f_res_14709;
        bool loop_while_14710;
        int64_t trial_14711;
        int64_t result_14712;
        
        loop_while_14710 = cond_14704;
        trial_14711 = end_14678;
        result_14712 = (int64_t) -1;
        while (loop_while_14710) {
            // stdlib/regex.fut:54:19-63:63
            
            int64_t result_14715;
            
            if (futrts_match_branch_10647(ctx, &result_14715, z30U_mem_19086, z31U_mem_19087, z32U_mem_19088, z33U_mem_19089, text_mem_19090, dz2080U_14624, dz2081U_14625, dz2082U_14626, dz2083U_14627, dz2080U_14628, match_branch_arg1_14713, last_14634, trial_14711) != 0) {
                err = 1;
                goto cleanup;
            }
            // stdlib/regex.fut:64:18-20
            
            int64_t tmp_14717 = sub64(trial_14711, (int64_t) 1);
            
            // stdlib/regex.fut:62:13-52
            
            bool cond_14723 = sle64(zgze_rhs_14702, tmp_14717);
            
            // stdlib/regex.fut:62:49-52
            
            bool loop_cond_t_res_14725 = slt64(result_14715, (int64_t) 0);
            bool x_16044 = cond_14723 && loop_cond_t_res_14725;
            bool loop_while_tmp_19292 = x_16044;
            int64_t trial_tmp_19293 = tmp_14717;
            int64_t result_tmp_19294 = result_14715;
            
            loop_while_14710 = loop_while_tmp_19292;
            trial_14711 = trial_tmp_19293;
            result_14712 = result_tmp_19294;
        }
        match_branch_res_f_res_14707 = loop_while_14710;
        match_branch_res_f_res_14708 = trial_14711;
        match_branch_res_f_res_14709 = result_14712;
        match_branch_res_14638 = match_branch_res_f_res_14709;
    }
    prim_out_19289 = match_branch_res_14638;
    *out_prim_out_19395 = prim_out_19289;
    
  cleanup:
    { }
    return err;
}
FUTHARK_FUN_ATTR int futrts_mul_abs_10600(struct futhark_context *ctx, struct memblock *mem_out_p_19396, int64_t *out_prim_out_19397, struct memblock a_mem_19086, int64_t dz2080U_13461, int32_t b_13463)
{
    (void) ctx;
    
    int err = 0;
    struct memblock ext_mem_19097;
    
    ext_mem_19097.references = NULL;
    
    struct memblock mem_19088;
    
    mem_19088.references = NULL;
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    int64_t prim_out_19290;
    
    // stdlib/bigint.fut:44:52-62
    
    int64_t dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_13465 = add64((int64_t) 1, dz2080U_13461);
    int64_t bytes_19087 = (int64_t) 4 * dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_13465;
    
    // stdlib/bigint.fut:48:7-36
    
    bool y_13513 = slt64(dz2080U_13461, dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_13465);
    
    // stdlib/bigint.fut:48:7-36
    
    bool index_certs_13515;
    
    if (!y_13513) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) dz2080U_13461, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_13465, "].", "-> #0  stdlib/bigint.fut:48:7-36\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    if (memblock_alloc(ctx, &mem_19088, bytes_19087, "mem_19088")) {
        err = 1;
        goto cleanup;
    }
    // stdlib/bigint.fut:44:21-47:27
    for (int64_t nest_i_19291 = 0; nest_i_19291 < dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_13465; nest_i_19291++) {
        ((int32_t *) mem_19088.mem)[nest_i_19291] = 0;
    }
    
    int64_t u32_res_16188 = zext_i32_i64(b_13463);
    
    // stdlib/bigint.fut:44:21-47:27
    
    int64_t mul_abs_res_13478;
    int64_t carry_13481 = (int64_t) 0;
    
    for (int64_t i_13479 = 0; i_13479 < dz2080U_13461; i_13479++) {
        // stdlib/bigint.fut:46:9-33
        
        bool y_13499 = slt64(i_13479, dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_13465);
        
        // stdlib/bigint.fut:46:9-33
        
        bool index_certs_13501;
        
        if (!y_13499) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) i_13479, "] out of bounds for array of shape [", (long long) dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_13465, "].", "-> #0  stdlib/bigint.fut:46:9-33\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/bigint.fut:45:27-31
        
        int32_t u32_arg0_13486 = ((int32_t *) a_mem_19086.mem)[i_13479];
        
        // stdlib/bigint.fut:45:19-31
        
        int64_t u32_res_16190 = zext_i32_i64(u32_arg0_13486);
        
        // stdlib/bigint.fut:45:31-41
        
        int64_t zp_lhs_13492 = mul64(u32_res_16188, u32_res_16190);
        
        // stdlib/bigint.fut:45:41-47
        
        int64_t product_13494 = add64(carry_13481, zp_lhs_13492);
        
        // stdlib/bigint.fut:46:18-33
        
        int32_t u64_res_16192 = zext_i64_i32(product_13494);
        
        // stdlib/bigint.fut:46:9-33
        ((int32_t *) mem_19088.mem)[i_13479] = u64_res_16192;
        // stdlib/bigint.fut:47:21-26
        
        int64_t tmp_13504 = lshr64(product_13494, (int64_t) 32);
        int64_t carry_tmp_19293 = tmp_13504;
        
        carry_13481 = carry_tmp_19293;
    }
    mul_abs_res_13478 = carry_13481;
    // stdlib/bigint.fut:48:23-36
    
    int32_t u64_res_16194 = zext_i64_i32(mul_abs_res_13478);
    
    // stdlib/bigint.fut:48:7-36
    ((int32_t *) mem_19088.mem)[dz2080U_13461] = u64_res_16194;
    // stdlib/bigint.fut:49:7-26
    
    int64_t mul_abs_res_13518;
    bool mul_abs_res_13519;
    
    if (futrts_normalise_10583(ctx, &ext_mem_19097, &mul_abs_res_13518, &mul_abs_res_13519, mem_19088, dzlz7bUZLzpZRz20UZLlengthz20UaZRz20U1z7dUzg_13465, 0) != 0) {
        err = 1;
        goto cleanup;
    }
    if (memblock_unref(ctx, &mem_19088, "mem_19088") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19289, &ext_mem_19097, "ext_mem_19097") != 0)
        return 1;
    prim_out_19290 = mul_abs_res_13518;
    if (memblock_set(ctx, &*mem_out_p_19396, &mem_out_19289, "mem_out_19289") != 0)
        return 1;
    *out_prim_out_19397 = prim_out_19290;
    
  cleanup:
    {
        if (memblock_unref(ctx, &ext_mem_19097, "ext_mem_19097") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_19088, "mem_19088") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19289, "mem_out_19289") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_normalise_10583(struct futhark_context *ctx, struct memblock *mem_out_p_19398, int64_t *out_prim_out_19399, bool *out_prim_out_19400, struct memblock a_mem_19086, int64_t dz2080U_13055, bool negative_13056)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    int64_t prim_out_19290;
    bool prim_out_19291;
    
    // stdlib/bigint.fut:5:14-65
    
    bool cond_13061 = slt64((int64_t) 1, dz2080U_13055);
    
    // stdlib/bigint.fut:5:50-52
    
    int64_t zeze_lhs_13063 = sub64(dz2080U_13055, (int64_t) 1);
    
    // stdlib/bigint.fut:5:47-53
    
    bool x_13065 = sle64((int64_t) 0, zeze_lhs_13063);
    
    // stdlib/bigint.fut:5:47-53
    
    bool y_13066 = slt64(zeze_lhs_13063, dz2080U_13055);
    
    // stdlib/bigint.fut:5:47-53
    
    bool bounds_check_13067 = x_13065 && y_13066;
    bool loop_not_taken_16036 = !cond_13061;
    bool protect_assert_disj_16037 = bounds_check_13067 || loop_not_taken_16036;
    
    // stdlib/bigint.fut:5:47-53
    
    bool index_certs_13068;
    
    if (!protect_assert_disj_16037) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zeze_lhs_13063, "] out of bounds for array of shape [", (long long) dz2080U_13055, "].", "-> #0  stdlib/bigint.fut:5:47-53\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:5:47-53
    
    int32_t zeze_lhs_13069;
    
    if (cond_13061) {
        int32_t x_18677 = ((int32_t *) a_mem_19086.mem)[zeze_lhs_13063];
        
        zeze_lhs_13069 = x_18677;
    } else {
        zeze_lhs_13069 = 0;
    }
    // stdlib/bigint.fut:5:54-58
    
    bool loop_cond_t_res_13071 = zeze_lhs_13069 == 0;
    bool x_16040 = cond_13061 && loop_cond_t_res_13071;
    
    // stdlib/bigint.fut:5:14-65
    
    bool used_13072;
    int64_t used_13073;
    bool loop_while_13074;
    int64_t k_13075;
    
    loop_while_13074 = x_16040;
    k_13075 = dz2080U_13055;
    while (loop_while_13074) {
        // stdlib/bigint.fut:5:63-65
        
        int64_t loopres_13076 = sub64(k_13075, (int64_t) 1);
        
        // stdlib/bigint.fut:5:38-58
        
        bool cond_13078 = slt64((int64_t) 1, loopres_13076);
        
        // stdlib/bigint.fut:5:38-58
        
        bool loop_cond_13079;
        
        if (cond_13078) {
            // stdlib/bigint.fut:5:50-52
            
            int64_t zeze_lhs_18678 = sub64(loopres_13076, (int64_t) 1);
            
            // stdlib/bigint.fut:5:47-53
            
            bool x_18679 = sle64((int64_t) 0, zeze_lhs_18678);
            
            // stdlib/bigint.fut:5:47-53
            
            bool y_18680 = slt64(zeze_lhs_18678, dz2080U_13055);
            
            // stdlib/bigint.fut:5:47-53
            
            bool bounds_check_18681 = x_18679 && y_18680;
            
            // stdlib/bigint.fut:5:47-53
            
            bool index_certs_18682;
            
            if (!bounds_check_18681) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) zeze_lhs_18678, "] out of bounds for array of shape [", (long long) dz2080U_13055, "].", "-> #0  stdlib/bigint.fut:5:47-53\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            // stdlib/bigint.fut:5:47-53
            
            int32_t zeze_lhs_18683 = ((int32_t *) a_mem_19086.mem)[zeze_lhs_18678];
            
            // stdlib/bigint.fut:5:54-58
            
            bool loop_cond_t_res_18684 = zeze_lhs_18683 == 0;
            
            loop_cond_13079 = loop_cond_t_res_18684;
        } else {
            loop_cond_13079 = 0;
        }
        
        bool loop_while_tmp_19292 = loop_cond_13079;
        int64_t k_tmp_19293 = loopres_13076;
        
        loop_while_13074 = loop_while_tmp_19292;
        k_13075 = k_tmp_19293;
    }
    used_13072 = loop_while_13074;
    used_13073 = k_13075;
    // stdlib/bigint.fut:6:15-26
    
    bool empty_slice_16068 = used_13073 == (int64_t) 0;
    
    // stdlib/bigint.fut:6:15-26
    
    int64_t m_16069 = sub64(used_13073, (int64_t) 1);
    
    // stdlib/bigint.fut:6:15-26
    
    bool zzero_leq_i_p_m_t_s_16070 = sle64((int64_t) 0, m_16069);
    
    // stdlib/bigint.fut:6:15-26
    
    bool i_p_m_t_s_leq_w_16071 = slt64(m_16069, dz2080U_13055);
    
    // stdlib/bigint.fut:6:15-26
    
    bool i_lte_j_16072 = sle64((int64_t) 0, used_13073);
    
    // stdlib/bigint.fut:6:15-26
    
    bool y_16073 = zzero_leq_i_p_m_t_s_16070 && i_p_m_t_s_leq_w_16071;
    
    // stdlib/bigint.fut:6:15-26
    
    bool forwards_ok_16074 = i_lte_j_16072 && y_16073;
    
    // stdlib/bigint.fut:6:15-26
    
    bool ok_or_empty_16075 = empty_slice_16068 || forwards_ok_16074;
    
    // stdlib/bigint.fut:6:15-26
    
    bool index_certs_16076;
    
    if (!ok_or_empty_16075) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, ":", (long long) used_13073, "] out of bounds for array of shape [", (long long) dz2080U_13055, "].", "-> #0  stdlib/bigint.fut:6:15-26\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/bigint.fut:7:7-48
    
    bool tmp_13092;
    
    if (negative_13056) {
        // stdlib/bigint.fut:7:21-47
        
        bool cond_18689 = used_13073 == (int64_t) 1;
        
        // stdlib/bigint.fut:7:21-47
        
        bool tmp_18690;
        
        if (cond_18689) {
            // stdlib/bigint.fut:7:34-42
            
            bool y_18691 = slt64((int64_t) 0, used_13073);
            
            // stdlib/bigint.fut:7:34-42
            
            bool index_certs_18692;
            
            if (!y_18691) {
                set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) (int64_t) 0, "] out of bounds for array of shape [", (long long) used_13073, "].", "-> #0  stdlib/bigint.fut:7:34-42\n"));
                err = FUTHARK_PROGRAM_ERROR;
                goto cleanup;
            }
            
            int32_t zeze_lhs_18693 = ((int32_t *) a_mem_19086.mem)[(int64_t) 0];
            
            // stdlib/bigint.fut:7:43-47
            
            bool tmp_t_res_18694 = zeze_lhs_18693 == 0;
            
            tmp_18690 = tmp_t_res_18694;
        } else {
            tmp_18690 = 0;
        }
        // stdlib/bigint.fut:7:19-48
        
        bool tmp_t_res_18695 = !tmp_18690;
        
        tmp_13092 = tmp_t_res_18695;
    } else {
        tmp_13092 = 0;
    }
    if (memblock_set(ctx, &mem_out_19289, &a_mem_19086, "a_mem_19086") != 0)
        return 1;
    prim_out_19290 = used_13073;
    prim_out_19291 = tmp_13092;
    if (memblock_set(ctx, &*mem_out_p_19398, &mem_out_19289, "mem_out_19289") != 0)
        return 1;
    *out_prim_out_19399 = prim_out_19290;
    *out_prim_out_19400 = prim_out_19291;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_19289, "mem_out_19289") != 0)
            return 1;
    }
    return err;
}
FUTHARK_FUN_ATTR int futrts_put_raw_10670(struct futhark_context *ctx, struct memblock *mem_out_p_19401, struct memblock *mem_out_p_19402, struct memblock *mem_out_p_19403, int64_t *out_prim_out_19404, struct memblock z30U_mem_19086, struct memblock z30U_mem_19087, struct memblock z30U_mem_19088, int64_t dz2080U_15782, int64_t z31U_15786, int64_t key_15787, int64_t value_15788)
{
    (void) ctx;
    
    int err = 0;
    struct memblock mem_out_19291;
    
    mem_out_19291.references = NULL;
    
    struct memblock mem_out_19290;
    
    mem_out_19290.references = NULL;
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    int64_t prim_out_19292;
    
    // stdlib/hashmap.fut:10:26-28
    
    int64_t mask_18588 = sub64(dz2080U_15782, (int64_t) 1);
    
    // stdlib/hashmap.fut:6:19-24
    
    int64_t zc_rhs_18589 = lshr64(key_15787, (int64_t) 30);
    
    // stdlib/hashmap.fut:6:14-24
    
    int64_t zt_lhs_18590 = key_15787 ^ zc_rhs_18589;
    
    // stdlib/hashmap.fut:6:26-48
    
    int64_t x_18591 = mul64((int64_t) -4658895280553007687, zt_lhs_18590);
    
    // stdlib/hashmap.fut:7:19-24
    
    int64_t zc_rhs_18592 = lshr64(x_18591, (int64_t) 27);
    
    // stdlib/hashmap.fut:7:14-24
    
    int64_t zt_lhs_18593 = x_18591 ^ zc_rhs_18592;
    
    // stdlib/hashmap.fut:7:26-48
    
    int64_t x_18594 = mul64((int64_t) -7723592293110705685, zt_lhs_18593);
    
    // stdlib/hashmap.fut:8:13-18
    
    int64_t zc_rhs_18595 = lshr64(x_18594, (int64_t) 31);
    
    // stdlib/hashmap.fut:8:8-18
    
    int64_t hash_res_18596 = x_18594 ^ zc_rhs_18595;
    
    // stdlib/hashmap.fut:11:33-47
    
    int64_t u64_arg0_18597 = mask_18588 & hash_res_18596;
    
    // stdlib/hashmap.fut:11:6-12:57
    
    bool x_18598 = sle64((int64_t) 0, u64_arg0_18597);
    
    // stdlib/hashmap.fut:11:6-12:57
    
    bool y_18599 = slt64(u64_arg0_18597, dz2080U_15782);
    
    // stdlib/hashmap.fut:11:6-12:57
    
    bool bounds_check_18600 = x_18598 && y_18599;
    
    // stdlib/hashmap.fut:11:6-12:57
    
    bool index_certs_18601;
    
    if (!bounds_check_18600) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) u64_arg0_18597, "] out of bounds for array of shape [", (long long) dz2080U_15782, "].", "-> #0  stdlib/hashmap.fut:11:6-12:57\n   #1  stdlib/hashmap.fut:19:11-25\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/hashmap.fut:11:6-12:57
    
    bool loop_cond_18602 = ((bool *) z30U_mem_19088.mem)[u64_arg0_18597];
    
    // stdlib/hashmap.fut:11:6-12:57
    
    bool loop_cond_18603;
    
    if (loop_cond_18602) {
        // stdlib/hashmap.fut:12:26-34
        
        int64_t loop_cond_t_res_18677 = ((int64_t *) z30U_mem_19086.mem)[u64_arg0_18597];
        
        // stdlib/hashmap.fut:12:37-43
        
        bool loop_cond_t_res_18678 = loop_cond_t_res_18677 == key_15787;
        
        // stdlib/hashmap.fut:12:37-43
        
        bool loop_cond_t_res_18679 = !loop_cond_t_res_18678;
        
        loop_cond_18603 = loop_cond_t_res_18679;
    } else {
        loop_cond_18603 = 0;
    }
    // stdlib/hashmap.fut:11:6-12:57
    
    bool slot_res_18611;
    int64_t slot_res_18612;
    bool loop_while_18613;
    int64_t i_18614;
    
    loop_while_18613 = loop_cond_18603;
    i_18614 = u64_arg0_18597;
    while (loop_while_18613) {
        // stdlib/hashmap.fut:12:49-51
        
        int64_t za_lhs_18615 = add64((int64_t) 1, i_18614);
        
        // stdlib/hashmap.fut:12:52-57
        
        int64_t loopres_18616 = mask_18588 & za_lhs_18615;
        
        // stdlib/hashmap.fut:12:12-20
        
        bool x_18617 = sle64((int64_t) 0, loopres_18616);
        
        // stdlib/hashmap.fut:12:12-20
        
        bool y_18618 = slt64(loopres_18616, dz2080U_15782);
        
        // stdlib/hashmap.fut:12:12-20
        
        bool bounds_check_18619 = x_18617 && y_18618;
        
        // stdlib/hashmap.fut:12:12-20
        
        bool index_certs_18620;
        
        if (!bounds_check_18619) {
            set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) loopres_18616, "] out of bounds for array of shape [", (long long) dz2080U_15782, "].", "-> #0  stdlib/hashmap.fut:12:12-20\n   #1  stdlib/hashmap.fut:19:11-25\n"));
            err = FUTHARK_PROGRAM_ERROR;
            goto cleanup;
        }
        // stdlib/hashmap.fut:12:12-20
        
        bool loop_cond_18621 = ((bool *) z30U_mem_19088.mem)[loopres_18616];
        
        // stdlib/hashmap.fut:12:12-43
        
        bool loop_cond_18622;
        
        if (loop_cond_18621) {
            // stdlib/hashmap.fut:12:26-34
            
            int64_t loop_cond_t_res_18680 = ((int64_t *) z30U_mem_19086.mem)[loopres_18616];
            
            // stdlib/hashmap.fut:12:37-43
            
            bool loop_cond_t_res_18681 = loop_cond_t_res_18680 == key_15787;
            
            // stdlib/hashmap.fut:12:37-43
            
            bool loop_cond_t_res_18682 = !loop_cond_t_res_18681;
            
            loop_cond_18622 = loop_cond_t_res_18682;
        } else {
            loop_cond_18622 = 0;
        }
        
        bool loop_while_tmp_19293 = loop_cond_18622;
        int64_t i_tmp_19294 = loopres_18616;
        
        loop_while_18613 = loop_while_tmp_19293;
        i_18614 = i_tmp_19294;
    }
    slot_res_18611 = loop_while_18613;
    slot_res_18612 = i_18614;
    // stdlib/hashmap.fut:20:27-35
    
    bool x_15795 = sle64((int64_t) 0, slot_res_18612);
    
    // stdlib/hashmap.fut:20:27-35
    
    bool y_15796 = slt64(slot_res_18612, dz2080U_15782);
    
    // stdlib/hashmap.fut:20:27-35
    
    bool bounds_check_15797 = x_15795 && y_15796;
    
    // stdlib/hashmap.fut:20:27-35
    
    bool index_certs_15798;
    
    if (!bounds_check_15797) {
        set_error(ctx, msgprintf("Error: %s%lld%s%lld%s\n\nBacktrace:\n%s", "Index [", (long long) slot_res_18612, "] out of bounds for array of shape [", (long long) dz2080U_15782, "].", "-> #0  stdlib/hashmap.fut:20:27-35\n"));
        err = FUTHARK_PROGRAM_ERROR;
        goto cleanup;
    }
    // stdlib/hashmap.fut:20:27-35
    
    bool put_raw_res_15801 = ((bool *) z30U_mem_19088.mem)[slot_res_18612];
    bool cond_neg_16034 = !put_raw_res_15801;
    int64_t zp_rhs_15805 = btoi_bool_i64(cond_neg_16034);
    
    // stdlib/hashmap.fut:20:21-51
    
    int64_t count_15807 = add64(z31U_15786, zp_rhs_15805);
    
    // stdlib/hashmap.fut:21:7-34
    ((int64_t *) z30U_mem_19086.mem)[slot_res_18612] = key_15787;
    // stdlib/hashmap.fut:21:7-34
    ((int64_t *) z30U_mem_19087.mem)[slot_res_18612] = value_15788;
    // stdlib/hashmap.fut:21:7-34
    ((bool *) z30U_mem_19088.mem)[slot_res_18612] = 1;
    if (memblock_set(ctx, &mem_out_19289, &z30U_mem_19086, "z30U_mem_19086") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19290, &z30U_mem_19087, "z30U_mem_19087") != 0)
        return 1;
    if (memblock_set(ctx, &mem_out_19291, &z30U_mem_19088, "z30U_mem_19088") != 0)
        return 1;
    prim_out_19292 = count_15807;
    if (memblock_set(ctx, &*mem_out_p_19401, &mem_out_19289, "mem_out_19289") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19402, &mem_out_19290, "mem_out_19290") != 0)
        return 1;
    if (memblock_set(ctx, &*mem_out_p_19403, &mem_out_19291, "mem_out_19291") != 0)
        return 1;
    *out_prim_out_19404 = prim_out_19292;
    
  cleanup:
    {
        if (memblock_unref(ctx, &mem_out_19291, "mem_out_19291") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19290, "mem_out_19290") != 0)
            return 1;
        if (memblock_unref(ctx, &mem_out_19289, "mem_out_19289") != 0)
            return 1;
    }
    return err;
}

int futhark_entry_arithmetic(struct futhark_context *ctx, struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 **out, const bool in0, const struct futhark_u32_1d *in1, const bool in2, const struct futhark_u32_1d *in3, const uint32_t in4)
{
    int64_t dz2080U_13919 = (int64_t) 0;
    int64_t dz2081U_13920 = (int64_t) 0;
    bool negative_a_13921 = 0;
    bool negative_b_13923 = 0;
    int32_t factor_13925 = 0;
    int64_t prim_out_19293 = (int64_t) 0;
    int64_t prim_out_19294 = (int64_t) 0;
    int64_t prim_out_19295 = (int64_t) 0;
    int64_t prim_out_19296 = (int64_t) 0;
    bool prim_out_19297 = 0;
    bool prim_out_19298 = 0;
    bool prim_out_19299 = 0;
    bool prim_out_19300 = 0;
    int64_t prim_out_19301 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_19292;
    
    mem_out_19292.references = NULL;
    
    struct memblock mem_out_19291;
    
    mem_out_19291.references = NULL;
    
    struct memblock mem_out_19290;
    
    mem_out_19290.references = NULL;
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    struct memblock b_mem_19087;
    
    b_mem_19087.references = NULL;
    
    struct memblock a_mem_19086;
    
    a_mem_19086.references = NULL;
    negative_a_13921 = in0;
    a_mem_19086 = in1->mem;
    dz2080U_13919 = in1->shape[0];
    negative_b_13923 = in2;
    b_mem_19087 = in3->mem;
    dz2081U_13920 = in3->shape[0];
    factor_13925 = in4;
    if (!(dz2080U_13919 == in1->shape[0] && dz2081U_13920 == in3->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_arithmetic(ctx, &mem_out_19289, &mem_out_19290, &mem_out_19291, &mem_out_19292, &prim_out_19293, &prim_out_19294, &prim_out_19295, &prim_out_19296, &prim_out_19297, &prim_out_19298, &prim_out_19299, &prim_out_19300, &prim_out_19301, a_mem_19086, b_mem_19087, dz2080U_13919, dz2081U_13920, negative_a_13921, negative_b_13923, factor_13925);
        if (ret == 0) {
            assert((*out = (struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64 *) malloc(sizeof(struct futhark_opaque_tup5_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_tup2_bool_arr1d_u32_i64))) != NULL);
            (*out)->v0 = prim_out_19297;
            assert(((*out)->v1 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v1->mem = mem_out_19289;
            (*out)->v1->shape[0] = prim_out_19293;
            (*out)->v2 = prim_out_19298;
            assert(((*out)->v3 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v3->mem = mem_out_19290;
            (*out)->v3->shape[0] = prim_out_19294;
            (*out)->v4 = prim_out_19299;
            assert(((*out)->v5 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v5->mem = mem_out_19291;
            (*out)->v5->shape[0] = prim_out_19295;
            (*out)->v6 = prim_out_19300;
            assert(((*out)->v7 = (struct futhark_u32_1d *) malloc(sizeof(struct futhark_u32_1d))) != NULL);
            (*out)->v7->mem = mem_out_19292;
            (*out)->v7->shape[0] = prim_out_19296;
            (*out)->v8 = prim_out_19301;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_hashmap_test(struct futhark_context *ctx, struct futhark_i64_1d **out, const struct futhark_u64_1d *in0)
{
    int64_t dz2080U_16031 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    struct memblock keys_mem_19086;
    
    keys_mem_19086.references = NULL;
    keys_mem_19086 = in0->mem;
    dz2080U_16031 = in0->shape[0];
    if (!(dz2080U_16031 == in0->shape[0])) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_hashmap_test(ctx, &mem_out_19289, keys_mem_19086, dz2080U_16031);
        if (ret == 0) {
            assert((*out = (struct futhark_i64_1d *) malloc(sizeof(struct futhark_i64_1d))) != NULL);
            (*out)->mem = mem_out_19289;
            (*out)->shape[0] = dz2080U_16031;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
int futhark_entry_regex_test(struct futhark_context *ctx, struct futhark_opaque_tup2_i64_arr1d_u8 **out, const struct futhark_u8_1d *in0, const struct futhark_u8_1d *in1, const struct futhark_u8_1d *in2)
{
    int64_t dz2080U_15585 = (int64_t) 0;
    int64_t dz2081U_15586 = (int64_t) 0;
    int64_t dz2082U_15587 = (int64_t) 0;
    int64_t prim_out_19290 = (int64_t) 0;
    int64_t prim_out_19291 = (int64_t) 0;
    int ret = 0;
    
    lock_lock(&ctx->lock);
    
    struct memblock mem_out_19289;
    
    mem_out_19289.references = NULL;
    
    struct memblock replacement_mem_19088;
    
    replacement_mem_19088.references = NULL;
    
    struct memblock text_mem_19087;
    
    text_mem_19087.references = NULL;
    
    struct memblock pattern_mem_19086;
    
    pattern_mem_19086.references = NULL;
    pattern_mem_19086 = in0->mem;
    dz2080U_15585 = in0->shape[0];
    text_mem_19087 = in1->mem;
    dz2081U_15586 = in1->shape[0];
    replacement_mem_19088 = in2->mem;
    dz2082U_15587 = in2->shape[0];
    if (!(dz2080U_15585 == in0->shape[0] && (dz2081U_15586 == in1->shape[0] && dz2082U_15587 == in2->shape[0]))) {
        ret = 1;
        set_error(ctx, msgprintf("Error: entry point arguments have invalid sizes.\n"));
    }
    if (ret == 0) {
        ret = futrts_entry_regex_test(ctx, &mem_out_19289, &prim_out_19290, &prim_out_19291, pattern_mem_19086, text_mem_19087, replacement_mem_19088, dz2080U_15585, dz2081U_15586, dz2082U_15587);
        if (ret == 0) {
            assert((*out = (struct futhark_opaque_tup2_i64_arr1d_u8 *) malloc(sizeof(struct futhark_opaque_tup2_i64_arr1d_u8))) != NULL);
            (*out)->v0 = prim_out_19291;
            assert(((*out)->v1 = (struct futhark_u8_1d *) malloc(sizeof(struct futhark_u8_1d))) != NULL);
            (*out)->v1->mem = mem_out_19289;
            (*out)->v1->shape[0] = prim_out_19290;
        }
    }
    lock_unlock(&ctx->lock);
    return ret;
}
  
