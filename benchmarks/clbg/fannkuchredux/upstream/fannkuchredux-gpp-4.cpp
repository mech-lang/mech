/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

contributed by Miroslav Rubanets
based on Java 6 source code by Oleg Mazurov.

License full text is here: https://salsa.debian.org/benchmarksgame-team/benchmarksgame/license.php

Building checked in Ubuntu 10.4 with g++ 4.4 (both x86 and amd64).
   one needs to install libboost-thread-dev package to get this working 
   g++ -c -O3 -pipe -pthread -fomit-frame-pointer -march=native f.cpp
   g++ -lpthread -lboost_thread f.o
*/
//std stuff
#include <cassert>
#include <algorithm>
#include <cstdio>
using std::copy;using std::max;using std::min; using std::atoi;
using std::printf;using std::swap;
//threads stuff
#include <boost/thread.hpp>
using boost::thread;using boost::thread_group;using boost::ref;

struct P{//permutation
    char data[16];
    char& operator[](int i){ return data[i]; }
    const char& operator[](int i)const{ return data[i]; }
    void assign( const char* p, int len){ copy(p, p+len, &data[0] ); }
    void rotate(int i){
        int first = data[0];
        for (int j=0; j<i; ++j)
            data[j] = data[j+1];
        data[i] = first;
    }
    void reverse(int n){
        // following lines are very carefully written to meet both test 
        // conditions and reasonable timings. 
        char * lo = &data[0], * hi = &data[n];
        for (; lo < hi; ++lo, --hi) 
            swap( *lo, *hi );
    }
};
class G{// permutation generator
    P p;
    int count[16];
    union{//for alignment
        struct{
            const int * fact;
            int len;
        };
        int padding[4];
    };
public:
    G():fact(), len(){}
    void init(const int*p, int n){
        fact = p;
        len = n;
        std::fill(&count[0], &count[16], 0);
    }
    bool constructed()const{ return fact != 0;}
    void first_permutation(int idx){
        char p[16]={};
        char pp[16]={};
        for ( int i=0; i<len; ++i ) 
           p[i] = i;
        for ( int i=len-1; i>0; --i ) {
            int d = idx / fact[i];
            count[i] = d;
            idx = idx % fact[i];
            copy( &p[0], &p[i+1], &pp[0] );
            for ( int j=0; j<=i; ++j ){
                p[j] = j+d <= i ? pp[j+d] : pp[j+d-i-1];
            }
        }
        this->p.assign(p, 16);
    }
    void next_permutation(){
        p.rotate(1);
        int i=1;
        while (++count[i] > i){
            count[i++] = 0;
            p.rotate( i );
        }
    }
    char get_first_item()const{ return p[0];}
    void get_permutation( P & out ){ out = p;}
};
struct Fannkuchredux{
    struct R{ int maxflips, checksum; };
    R run(G* g, int i, int N){
        R r = { 0, 0 };
        g->first_permutation(i);
        for(; i < N; ++i, g->next_permutation()){
            //count flips
            register int flips = 0;
            char f = g->get_first_item();
            if(f){
                P p;
                g->get_permutation(p);
                do{
                    ++flips;
                    p.reverse(f);
                }while(f = p[0]);
            }
            int total_flips = flips;
            r.maxflips = max(r.maxflips, total_flips);
            r.checksum += i%2 ==0 ? total_flips : -total_flips;
        }
        return r;
    }
};
//two phase construction to be default constructible
class Task : public Fannkuchredux {
    G g;
    int first_index, last_index;
    int padding[14];
public:
    Fannkuchredux::R r;
    void init(const int *p, int n, int b, int e){
        g.init( p, n );
        first_index = b;
        last_index = e;
        r.checksum = r.maxflips = 0;
    }
    void operator()()
    {
        assert( g.constructed() );
        r = run( &g, first_index, last_index );
    }
};
const char* usage = "usage fannkuchredux number\n\
number has to be in range [3-12]\n";
int main(int argc, char* argv[])
{
    if( argc < 2 ){
        printf("%s", usage);
        return 1;
    }
    int len = atoi(argv[1] ); 
    if( len < 3 || len > 12 ){
        printf("%s", usage);
        return 2;
    }
    int fact[16];
    fact[0] = 1;
    for(int i = 1; i<len+1; ++i)
        fact[i] = fact[i-1]*i;
    unsigned n_cpu = thread::hardware_concurrency();
    Fannkuchredux::R r= { 0, 0};
    const unsigned max_cpu_limit = 4;
    Task parts[max_cpu_limit];
    unsigned n = min(n_cpu, max_cpu_limit);
    thread_group tg;
    int index = 0;
    int index_max = fact[len]; 
    int index_step = (index_max + n-1)/n;
    for(unsigned i = 0; i<n; ++i, index += index_step){
        Task& p = parts[i];
        p.init(fact, len, index, index + index_step);
        tg.create_thread(ref(p));
    }
    tg.join_all();
    for(unsigned i = 0; i<n; ++i){
        Task const& p = parts[i];
        r.maxflips = max( p.r.maxflips, r.maxflips );
        r.checksum += p.r.checksum;
    }
    printf("%d\nPfannkuchen(%d) = %d\n", r.checksum, len, r.maxflips);
    return 0;
}
