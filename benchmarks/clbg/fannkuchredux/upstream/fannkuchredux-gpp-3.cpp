/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Branimir Maksimovic
*/
#include <cstdlib>
#include <cstdio>
#include <algorithm>

typedef unsigned char int_t;

void rotate(int_t* p, int n)
{
   int_t tmp = p[0];
   for(int i = 0; i < n; ++i)p[i]=p[i+1];
   p[n] = tmp;
}

bool next_permutation(int_t* beg, int n, int_t* c)
{
   int i = 1;
   while(i<n)
   {
      rotate(beg,i);
      if(c[i]>=i)c[i++]=0;
      else break;
   }
   if(i>=n)return false;
   ++c[i];
   return true;
}


struct Result{
   int checksum;
   int maxflips;
};

Result fannkuch(int n)
{
   Result tmp = {0};
   int i=0,permcount=0;
   int_t perm[16],tperm[16],cnt[16]={0};
   
   std::generate(perm,perm+n,[&i](){ return ++i; });
   
   do
   {
      std::copy(perm,perm+n,tperm);
      int flips = 0;
      while(tperm[0] != 1)
      {
         std::reverse(tperm,tperm+tperm[0]);
         ++flips;
      }
      tmp.checksum += (permcount%2 == 0)?flips:-flips;
      tmp.maxflips = std::max(tmp.maxflips,flips);
   }while(++permcount,next_permutation(perm,n,cnt));

   return tmp;
}

int main(int argc, char** argv)
{
   int n = 7;
   if(argc > 1)n = atoi(argv[1]);
   if(n < 3 || n > 16)
   {
      printf("n should be between [3 and 16]\n");
      return 0;
   }
   Result r = fannkuch(n);
   printf("%d\nPfannkuchen(%d) = %d\n",r.checksum,n,r.maxflips);
}
