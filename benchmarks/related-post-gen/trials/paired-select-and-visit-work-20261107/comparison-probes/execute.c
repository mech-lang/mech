#include <stdint.h>
#include <stdio.h>
extern uint64_t bool_eq_and_scalar(uint64_t,uint64_t,uint64_t,uint64_t),bool_eq_and_pair(uint64_t,uint64_t,uint64_t,uint64_t),bool_ne_or_scalar(uint64_t,uint64_t,uint64_t,uint64_t),bool_ne_or_pair(uint64_t,uint64_t,uint64_t,uint64_t);
int main(void){uint64_t v[]={0,1,2,0xffffffffULL,0x100000000ULL,0x20000000000001ULL,0x8000000000000000ULL,~0ULL};unsigned n=0;
for(unsigned i=0;i<8;i++)for(unsigned j=0;j<8;j++)for(unsigned k=0;k<8;k++)for(unsigned l=0;l<8;l++){
uint64_t a=v[i],b=v[j],c=v[k],d=v[l],eq=a==b&&c==d,ne=a!=b||c!=d;
if(bool_eq_and_scalar(a,b,c,d)!=eq||bool_eq_and_pair(a,b,c,d)!=eq||bool_ne_or_scalar(a,b,c,d)!=ne||bool_ne_or_pair(a,b,c,d)!=ne)return 1;n++;}
printf("%u native comparison-pair cases passed\n",n);return 0;}
