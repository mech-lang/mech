/* Test-only instrumentation of ordinary APR allocator calls. */
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

extern void *apr_palloc(void *, size_t);
extern void apr_pool_clear(void *);
static _Atomic uint64_t allocations, bytes, non_16_byte_allocations, clears;
static void *count_allocate(void *pool, size_t size) {
  atomic_fetch_add(&allocations, 1);
  atomic_fetch_add(&bytes, size);
  if (size != 16) atomic_fetch_add(&non_16_byte_allocations, 1);
  return apr_palloc(pool, size);
}
static void count_clear(void *pool) {
  atomic_fetch_add(&clears, 1);
  apr_pool_clear(pool);
}
__attribute__((used, section("__DATA,__interpose")))
static const struct { const void *replacement; const void *original; } interposers[] = {
  {count_allocate, apr_palloc}, {count_clear, apr_pool_clear}
};
__attribute__((destructor)) static void report_counter(void) {
  fprintf(stderr, "{\"allocations\":%llu,\"bytes\":%llu,\"non_16_byte_allocations\":%llu,\"clears\":%llu}\n",
    (unsigned long long)atomic_load(&allocations),
    (unsigned long long)atomic_load(&bytes),
    (unsigned long long)atomic_load(&non_16_byte_allocations),
    (unsigned long long)atomic_load(&clears));
}
