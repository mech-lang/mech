/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Branimir Maksimovic
   first permutation algo taken from Miroslav Rubanets program
*/
#include <cstdlib>
#include <cstdio>
#include <algorithm>
#include <future>
#include <unistd.h>

typedef unsigned char int_t;

void rotate(int_t* p, int n)
{
   int_t tmp = p[0];
   for(int i = 0; i < n; ++i)p[i]=p[i+1];
   p[n] = tmp;
}

void next_permutation(int_t* beg, int n, int_t* c)
{
   int i = 1;
   while(i<n)
   {
      rotate(beg,i);
      if(c[i]>=i)c[i++]=0;
      else break;
   }
   ++c[i];
}

class Perm{
public:
struct P{
   int_t p[16];
};
Perm(unsigned n)
: cnt {0},n(n),permcount(0)
{
   fact[0]=1;
   for(unsigned i=1;i<n+1;++i)
   {
      fact[i]=fact[i-1]*i;
   }
}
P get(int idx)
{ 
    char pp[16]={};
    permcount = idx;
    int_t i = 0;
    std::generate(perm.p,perm.p+n,[&i](){return ++i;});
    for ( unsigned i=n-1; i>0; --i ) {
        unsigned d = idx / fact[i];
        cnt[i] = d;
        idx = idx % fact[i];
        std::copy( &perm.p[0], &perm.p[i+1], &pp[0] );
        for (unsigned j=0; j<=i; ++j ){
         perm.p[j] = j+d <= i ? pp[j+d] : pp[j+d-i-1];
      }
    }
   return perm;
}
P next()
{
   next_permutation(perm.p,n,cnt);
   ++permcount;
   return perm;
}
unsigned count()const { return permcount; }
unsigned max()const { return fact[n]; }
private:
   int_t cnt[16];
   unsigned fact[16],n,permcount;
   P perm;
};

struct Result{
   int checksum;
   int maxflips;
};

Result work(Perm perm,unsigned n,unsigned max)
{
   Result r={0};
   Perm::P p = perm.get(n);
   for(; perm.count()<max;p=perm.next())
   {
      int flips = 0;
      while(p.p[0] != 1)
      {
         std::reverse(p.p,p.p+p.p[0]);
         ++flips;
      }
      r.checksum += (perm.count()%2 == 0)?flips:-flips;
      r.maxflips = std::max(r.maxflips,flips);
   }
   return r;
}

Result fannkuch(int n)
{
   Result tmp = {0};
   Perm perm(n);
   
   unsigned N = sysconf(_SC_NPROCESSORS_ONLN);
   std::future<Result> ft[N];
   
   unsigned k = perm.max()/N;
   unsigned j = 0;
   for(unsigned i = 0 ; i < N;++i,j+=k)
   {
      unsigned max = i<N-1?j+k:perm.max();
      ft[i] = std::async(std::launch::async,work,perm,j,max);
   }
   for(unsigned i = 0; i < N; ++i)
   {
      auto r = ft[i].get();
      tmp.checksum += r.checksum;
      tmp.maxflips = std::max(tmp.maxflips,r.maxflips);
   }
   return tmp;
}

int main(int argc, char** argv)
{
   int n = 7;
   if(argc > 1)n = atoi(argv[1]);
   if(n < 3 || n > 12)
   {
      printf("n should be between [3 and 12]\n");
      return 0;
   }
   Result r = fannkuch(n);
   printf("%d\nPfannkuchen(%d) = %d\n",r.checksum,n,r.maxflips);
}
