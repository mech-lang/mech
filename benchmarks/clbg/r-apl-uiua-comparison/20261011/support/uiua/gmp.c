/* General mutable GMP integer operations, with no benchmark-specific kernels. */
#include <gmp.h>
#include <stdlib.h>
typedef struct { mpz_t value; } Integer;
void *ui_big_new(long n) {
  Integer *p = malloc(sizeof(*p));
  if (!p) abort();
  mpz_init_set_si(p->value, n);
  return p;
}
void ui_big_free(Integer *p) {
  mpz_clear(p->value);
  free(p);
}
void *ui_big_set(Integer *out, long n) {
  mpz_set_si(out->value, n);
  return out;
}
void *ui_big_copy(Integer *out, Integer *in) {
  mpz_set(out->value, in->value);
  return out;
}
void *ui_big_add(Integer *out, Integer *a, Integer *b) {
  mpz_add(out->value, a->value, b->value);
  return out;
}
void *ui_big_sub(Integer *out, Integer *a, Integer *b) {
  mpz_sub(out->value, a->value, b->value);
  return out;
}
void *ui_big_mul(Integer *out, Integer *a, Integer *b) {
  mpz_mul(out->value, a->value, b->value);
  return out;
}
void *ui_big_mul_small(Integer *out, Integer *a, long b) {
  mpz_mul_si(out->value, a->value, b);
  return out;
}
void *ui_big_floor_div(Integer *out, Integer *a, Integer *b) {
  mpz_fdiv_q(out->value, a->value, b->value);
  return out;
}
long ui_big_get(Integer *p) {
  return mpz_get_si(p->value);
}
int ui_big_compare(Integer *a, Integer *b) {
  return mpz_cmp(a->value, b->value);
}
