/* General FFI bindings: GMP integer arithmetic, PCRE2 regular expressions,
   and the unmodified khash uint64->uint64 map. No benchmark algorithms,
   input parsing, DNA encoding, permutations, or benchmark constants live here. */
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <gmp.h>
/* General black-box scalar: keeps a computation inside its requested loop. */
int64_t opaque_int(int64_t value) { __asm__ __volatile__("" : "+r"(value) : : "memory");return value; }
#define PCRE2_CODE_UNIT_WIDTH 8
#include <pcre2.h>
#include "khash.h"
KHASH_MAP_INIT_INT64(counts, uint64_t)

void *z_new(uint64_t n) { mpz_t *p=malloc(sizeof(mpz_t)); if(!p)abort(); mpz_init_set_ui(*p,n); return p; }
void z_close(void *p) { mpz_clear(*(mpz_t*)p); free(p); }
void z_add(void *o,void *a,void *b) { mpz_add(*(mpz_t*)o,*(mpz_t*)a,*(mpz_t*)b); }
void z_sub(void *o,void *a,void *b) { mpz_sub(*(mpz_t*)o,*(mpz_t*)a,*(mpz_t*)b); }
void z_mul_ui(void *o,void *a,uint64_t b) { mpz_mul_ui(*(mpz_t*)o,*(mpz_t*)a,b); }
void z_div(void *o,void *a,void *b) { mpz_fdiv_q(*(mpz_t*)o,*(mpz_t*)a,*(mpz_t*)b); }
uint64_t z_ui(void *p) { return mpz_get_ui(*(mpz_t*)p); }
int z_cmp(void *a,void *b) { return mpz_cmp(*(mpz_t*)a,*(mpz_t*)b); }
void z_copy(void *o,void *a) { mpz_set(*(mpz_t*)o,*(mpz_t*)a); }
void z_neg(void *o,void *a) { mpz_neg(*(mpz_t*)o,*(mpz_t*)a); }
void z_mul_si(void *o,void *a,int64_t b) { mpz_mul_si(*(mpz_t*)o,*(mpz_t*)a,b); }
int64_t z_i64(void *a) { if(!mpz_fits_slong_p(*(mpz_t*)a))abort();return mpz_get_si(*(mpz_t*)a); }
void z_set_decimal(void *o,const uint8_t *s,size_t n) {char*p=malloc(n+1);if(!p)abort();memcpy(p,s,n);p[n]=0;if(mpz_set_str(*(mpz_t*)o,p,10))abort();free(p);}
size_t z_decimal_size(void *a) {return mpz_sizeinbase(*(mpz_t*)a,10)+2;}
void z_decimal(void *a,char *out) {mpz_get_str(out,10,*(mpz_t*)a);}
/* General a*factor+b, with every destination/input alias combination supported. */
void z_mul_add_ui(void *o,void *a,uint64_t factor,void *b) {
 if(o==b && o!=a){mpz_addmul_ui(*(mpz_t*)o,*(mpz_t*)a,factor);}
 else if(o==a){mpz_t tmp;mpz_init(tmp);mpz_mul_ui(tmp,*(mpz_t*)a,factor);mpz_add(tmp,tmp,*(mpz_t*)b);mpz_swap(tmp,*(mpz_t*)o);mpz_clear(tmp);}
 else{mpz_mul_ui(*(mpz_t*)o,*(mpz_t*)a,factor);mpz_add(*(mpz_t*)o,*(mpz_t*)o,*(mpz_t*)b);}
}

void *map_new(void) { void *p=kh_init(counts);if(!p)abort();return p; }
void map_close(void *p) {kh_destroy(counts,p);}
uint64_t map_get(void *p,uint64_t key) {khash_t(counts)*h=p;khiter_t i=kh_get(counts,h,key);return i==kh_end(h)?0:kh_value(h,i);}
void map_set(void *p,uint64_t key,uint64_t value) {khash_t(counts)*h=p;int ret;khiter_t i=kh_put(counts,h,key,&ret);if(ret<0)abort();kh_value(h,i)=value;}

/* Count is a normal regex library operation: each call traverses exactly
   one supplied pattern. Replacement uses PCRE2's global substitute API. */
typedef struct {pcre2_code*code;pcre2_match_data*match;} regex_t;
typedef struct {uint8_t*data;size_t size;} buffer_t;
void *regex_new(const uint8_t*pattern,size_t size) {
 int error;PCRE2_SIZE offset;regex_t*r=malloc(sizeof(*r));if(!r)abort();
 r->code=pcre2_compile(pattern,size,0,&error,&offset,NULL);if(!r->code)abort();
 r->match=pcre2_match_data_create_from_pattern(r->code,NULL);if(!r->match)abort();
 pcre2_jit_compile(r->code,PCRE2_JIT_COMPLETE);return r;
}
void regex_close(void*p){regex_t*r=p;pcre2_match_data_free(r->match);pcre2_code_free(r->code);free(r);}
uint64_t regex_count(void*p,const uint8_t*s,size_t size){
 regex_t*r=p;PCRE2_SIZE offset=0;uint64_t count=0;int result;
 while((result=pcre2_match(r->code,s,size,offset,0,r->match,NULL))>=0){
  PCRE2_SIZE*v=pcre2_get_ovector_pointer(r->match);count++;offset=v[1];
  if(v[0]==v[1]){if(offset==size)break;offset++;}
 }
 if(result<0&&result!=PCRE2_ERROR_NOMATCH)abort();return count;
}
void *regex_replace(void*p,const uint8_t*s,size_t n,const uint8_t*replacement,size_t rn){
 regex_t*r=p;
 uint32_t options=PCRE2_SUBSTITUTE_GLOBAL|PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
 buffer_t*b=malloc(sizeof(*b));if(!b)abort();b->data=malloc(n+1);if(!b->data)abort();
 PCRE2_SIZE capacity=n+1;
 int status=pcre2_substitute(r->code,s,n,0,options,r->match,NULL,replacement,rn,b->data,&capacity);
 if(status==PCRE2_ERROR_NOMEMORY){
  b->data=realloc(b->data,capacity+1);if(!b->data)abort();capacity++;
  status=pcre2_substitute(r->code,s,n,0,PCRE2_SUBSTITUTE_GLOBAL,r->match,NULL,replacement,rn,b->data,&capacity);
 }
 if(status<0)abort();b->size=capacity;return b;
}
uint8_t *buffer_data(void*p){return ((buffer_t*)p)->data;}
size_t buffer_size(void*p){return ((buffer_t*)p)->size;}
void buffer_close(void*p){buffer_t*b=p;free(b->data);free(b);}

/* Generic process map support. The child resumes the language interpreter;
   this binding contains no callback computation or benchmark selection. */
#include <unistd.h>
#include <sys/wait.h>
typedef struct {pid_t pid;int fd;} process_t;
void *process_fork(void){
 int fds[2];if(pipe(fds))abort();fflush(NULL);
 process_t*p=malloc(sizeof(*p));if(!p)abort();p->pid=fork();if(p->pid<0)abort();
 if(!p->pid){close(fds[0]);if(dup2(fds[1],STDOUT_FILENO)<0)abort();close(fds[1]);p->fd=STDOUT_FILENO;}
 else{close(fds[1]);p->fd=fds[0];}return p;
}
int process_pid(void*p){return ((process_t*)p)->pid;}
int process_fd(void*p){return ((process_t*)p)->fd;}
void process_wait(void*p){process_t*h=p;int status;if(waitpid(h->pid,&status,0)<0||!WIFEXITED(status)||WEXITSTATUS(status))abort();close(h->fd);free(h);}
void process_exit(int status){exit(status);}
