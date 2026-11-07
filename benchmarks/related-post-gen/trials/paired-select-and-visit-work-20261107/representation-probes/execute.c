#include <stdint.h>
#include <string.h>
#include <stdio.h>
typedef struct {uint64_t a,b;} Pair;
extern Pair independent_selects(uint64_t,uint64_t),paired_selects(uint64_t,uint64_t);
extern void scalar_clear(void*),vector_clear(void*);
extern uint64_t bool_cone_i8(uint64_t,uint64_t,uint64_t,uint64_t),bool_cone_i64(uint64_t,uint64_t,uint64_t,uint64_t),checked_add(uint64_t,uint64_t);
int main(void){
uint64_t v[]={0,1,2,0xffffffffULL,0x100000000ULL,0x20000000000001ULL,0x8000000000000000ULL,~0ULL};
unsigned tests=0;for(unsigned a=0;a<8;a++)for(unsigned b=0;b<8;b++){
 Pair x=independent_selects(v[a],v[b]),y=paired_selects(v[a],v[b]);
 if(x.a!=y.a||x.b!=y.b||y.a!=(v[a]>v[b]?v[a]:v[b])||y.b!=(v[a]>v[b]?v[b]:v[a]))return 1;tests++;
 for(unsigned c=0;c<8;c++)for(unsigned d=0;d<8;d++){uint64_t want=v[a]<v[b]&&v[c]<=v[d];if(bool_cone_i8(v[a],v[b],v[c],v[d])!=want||bool_cone_i64(v[a],v[b],v[c],v[d])!=want)return 2;tests++;}
 uint64_t want=v[a]>UINT64_MAX-v[b]?0:v[a]+v[b];if(checked_add(v[a],v[b])!=want)return 3;
}
for(unsigned offset=0;offset<32;offset++){unsigned char a[64],b[64];memset(a,0xad,64);memset(b,0xad,64);scalar_clear(a+offset);vector_clear(b+offset);if(memcmp(a,b,64))return 4;for(unsigned i=0;i<64;i++)if(a[i]!=(i>=offset&&i<offset+16?0:0xad))return 5;tests++;}
printf("%u native probe cases passed\n",tests);return 0;}
