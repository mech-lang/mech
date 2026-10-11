/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

contributed by Miroslav Rubanets
algorithm is based on Java 6 source code by Oleg Mazurov
source is based on my C++ submission.

Building checked in Ubuntu 11.4 with g++ 4.5 (both x86 and amd64).
gcc -pipe -Wall -O3 -fomit-frame-pointer -march=native -lpthread \
    -falign-labels=8 fannkuchredux.gcc-2.c -o fannkuchredux.gcc-2.gcc_run
note that -falign-labels=8 is needed only on x86 with gcc 4.5
*/

//std stuff
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
//posix threads
#include <pthread.h>
//linux (for sysconf)
#include <unistd.h>
//gcc stuff
#define unlikely(x)     __builtin_expect((x),0)
//hardcoded limits
#define MAX_PROBLEM_SIZE 12
#define MAX_CPU_LIMIT 4

inline int max(int a, int b){ return a > b ? a : b;}
inline int min(int a, int b){ return a < b ? a : b;}

typedef struct tagResult{int maxflips, checksum; } Result;
typedef struct tagPermutationGenerator{
    int perm[MAX_PROBLEM_SIZE];
    int count[MAX_PROBLEM_SIZE];
    int* factorials;    
    int length;
} G;

inline void copy(int* dst, int* src, int n){
    int* e = src+n;
    for(; src != e; ++src,++dst )
        *dst = *src;
}
inline void rotate( int* data, int n){
    int i;
    int first = data[0];
    for (i=0; i<n; ++i)
        data[i] = data[i+1];
    data[n] = first;
}
inline void reverse( int*data, int n ){
    int * lo = &data[0], * hi = &data[n];
    for (; lo < hi; ++lo, --hi){ 
        int tmp = *lo; *lo = *hi; *hi = tmp;
    }
}
void first_permutation( G* g, int idx ){
    int p[MAX_PROBLEM_SIZE];
    int pp[MAX_PROBLEM_SIZE];
    int len = g->length;
    int d, i, j;
    memset(p, 0, MAX_PROBLEM_SIZE*sizeof(int));
    memset(pp, 0, MAX_PROBLEM_SIZE*sizeof(int));
    for ( i=0; i<len; ++i ) 
        p[i] = i;
    for ( i=len-1; i>0; --i ) {
        d = idx / g->factorials[i];
        g->count[i] = d;
        idx = idx % g->factorials[i];
        copy( &pp[0], &p[0], (i+1) );
        for ( j=0; j<=i; ++j ){
            p[j] = j+d <= i ? pp[j+d] : pp[j+d-i-1];
        }
    }
    copy( &g->perm[0], &p[0], len );
}
void next_permutation( G*p ){
    int i=1;
    rotate( p->perm, i);
    while (++p->count[i] > i){
        p->count[i++] = 0;
        rotate( p->perm, i );
    }
}
typedef struct tagTaskContext{
    union{// to avoid false sharing on multi cpu.
        pthread_t thread;
        char padding[64];
    };
    G generator;
    int first_index, last_index;
    Result result;
} Task;

void* task_body( void *pvoid ){
    Task* p = (Task*)pvoid;
    int total_flips;
    int i = p->first_index;
    int n = p->last_index;
    first_permutation( &p->generator, i );
    for(; i < n; ++i){
        int data[MAX_PROBLEM_SIZE];
        register int flips = 0;        
        register int f =  p->generator.perm[0];
        if(f){
            copy( &data[0], &p->generator.perm[0], p->generator.length );
            do{
                reverse( data, f );
                ++flips;
            }while( unlikely( f = data[0] ) );
        }
        total_flips = flips;
        p->result.maxflips = max(p->result.maxflips, total_flips);
        p->result.checksum += i%2 ==0 ? total_flips : -total_flips;
        next_permutation( &p->generator );
    }
    return 0;
}
int hardware_concurrency(){//linux specific
    long numCPU = sysconf( _SC_NPROCESSORS_ONLN );
    if( numCPU <= 0 ) return 1;
    if( numCPU >= MAX_CPU_LIMIT ) return MAX_CPU_LIMIT;
    return (int)numCPU;
}
const char* usage = "usage fannkuchredux number\n\
number has to be in range [3-12]\n";
int main(int argc, char* argv[]){
    int len;
    int factorials[MAX_PROBLEM_SIZE+1];
    int n_cpu;
    int i, n, index, index_max, index_step, err;
    Result result;
    Task parts[MAX_CPU_LIMIT];
    if( argc < 2 ){
        printf("%s", usage);
        return 1;
    }
    len = atoi(argv[1] ); 
    if( len < 3 || len > MAX_PROBLEM_SIZE ){
        printf("%s", usage);
        return 2;
    }
    factorials[0] = 1;
    for( i = 1; i<len+1; ++i )
        factorials[i] = factorials[i-1]*i;
    n_cpu = hardware_concurrency();
    result.maxflips = 0;
    result.checksum = 0;
    n = min( n_cpu, MAX_CPU_LIMIT );
    index = 0;
    index_max = factorials[len]; 
    index_step = (index_max + n-1)/n;
    for(i = 0; i<n; ++i, index += index_step){
        Task* p = &parts[i];
        //init task
        memset( p, 0, sizeof(Task) );
        p->generator.factorials = factorials;
        p->generator.length = len;
        p->first_index = index;
        p->last_index = index + index_step;
        err = pthread_create( &p->thread, 0, task_body, p );
        if( err ) return err;
    }    
    for(i = 0; i<n; ++i){
        Task *p = &parts[i];
        err = pthread_join( p->thread, 0 );
        if( err ) return err;
        result.maxflips = max( p->result.maxflips, result.maxflips );
        result.checksum += p->result.checksum;
    }
    printf("%d\nPfannkuchen(%d) = %d\n", result.checksum, len, result.maxflips);
    return 0;
}
