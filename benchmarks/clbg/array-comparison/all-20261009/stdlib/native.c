/* General FFI bindings: GMP integer arithmetic, PCRE2 regular expressions,
   and the unmodified khash uint64->uint64 map. No benchmark algorithms,
   input parsing, DNA encoding, permutations, or benchmark constants live here. */
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <gmp.h>
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
 regex_t*r=p;PCRE2_SIZE length=0;
 uint32_t options=PCRE2_SUBSTITUTE_GLOBAL|PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
 int status=pcre2_substitute(r->code,s,n,0,options,r->match,NULL,replacement,rn,NULL,&length);
 if(status<0&&status!=PCRE2_ERROR_NOMEMORY)abort();
 buffer_t*b=malloc(sizeof(*b));if(!b)abort();b->data=malloc(length+1);if(!b->data)abort();
 PCRE2_SIZE capacity=length+1;
 status=pcre2_substitute(r->code,s,n,0,PCRE2_SUBSTITUTE_GLOBAL,r->match,NULL,replacement,rn,b->data,&capacity);
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

#ifdef GROWLER
#include <k.h>
/* K's extension ABI owns its argument references. Integer handles avoid
   relying on double precision to store pointers. */
static void*objects[256];static unsigned used;
static K handle(void*p){if(used==256)abort();objects[used]=p;return Ki(used++);}
static void*obj(K k){int i=iK(k);unref(k);if(i<0||i>=used||!objects[i])abort();return objects[i];}
static uint64_t number(K k){uint64_t v=TK(k)==14?*(int64_t*)dK(k):(int64_t)iK(k);unref(k);return v;}
K k_z_new(K n){return handle(z_new(number(n)));}
K k_z_add(K o,K a,K b){z_add(obj(o),obj(a),obj(b));return Kv();}
K k_z_sub(K o,K a,K b){z_sub(obj(o),obj(a),obj(b));return Kv();}
K k_z_mul(K o,K a,K n){z_mul_ui(obj(o),obj(a),number(n));return Kv();}
K k_z_div(K o,K a,K b){z_div(obj(o),obj(a),obj(b));return Kv();}
K k_z_ui(K p){return Kp((void*)(uintptr_t)z_ui(obj(p)));}
K k_z_cmp(K a,K b){return Ki(z_cmp(obj(a),obj(b)));}
K k_map_new(K unused){unref(unused);return handle(map_new());}
K k_map_get(K h,K key){return Kp((void*)(uintptr_t)map_get(obj(h),number(key)));}
K k_map_set(K h,K key,K value){map_set(obj(h),number(key),number(value));return Kv();}
K k_map_close(K h){map_close(obj(h));return Kv();}
K k_map_keys(K h){khash_t(counts)*m=obj(h);size_t n=kh_size(m),j=0;K*keys=malloc(n*sizeof(K));if(!keys&&n)abort();for(khiter_t i=kh_begin(m);i!=kh_end(m);i++)if(kh_exist(m,i))keys[j++]=Kp((void*)(uintptr_t)kh_key(m,i));K out=KL(keys,n);free(keys);return out;}
typedef struct {uint8_t one;const uint8_t*data;size_t size;} kbytes_t;
static void kbytes(K x,kbytes_t*b){if(TK(x)==16){b->one=cK(x);b->data=&b->one;b->size=1;}else if(TK(x)==9){b->data=dK(x);b->size=NK(x);}else abort();}
K k_regex_new(K pattern){kbytes_t b;kbytes(pattern,&b);void*p=regex_new(b.data,b.size);unref(pattern);return handle(p);}
K k_regex_count(K h,K s){kbytes_t b;kbytes(s,&b);uint64_t n=regex_count(obj(h),b.data,b.size);unref(s);return Kp((void*)(uintptr_t)n);}
K k_regex_replace(K h,K s,K r){kbytes_t a,b;kbytes(s,&a);kbytes(r,&b);buffer_t*out=regex_replace(obj(h),a.data,a.size,b.data,b.size);unref(s);unref(r);K value=KC((C*)out->data,out->size);buffer_close(out);return value;}
K k_process_new(K unused){unref(unused);return handle(process_fork());}
K k_process_pid(K h){return Ki(process_pid(obj(h)));}
K k_process_fd(K h){return Ki(process_fd(obj(h)));}
K k_process_wait(K h){process_wait(obj(h));return Kv();}
K k_process_exit(K n){process_exit(iK(n));return Kv();}
#endif
