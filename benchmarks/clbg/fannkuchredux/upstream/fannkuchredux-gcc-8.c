/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Rex Kerr's Scala program
   contributed by Isaac Gouy 
*/

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

int fannkuch(int n)
{
    int perm1[n]; 
    for (int i=0; i<n; i+=1) perm1[i] = i;
    int perm[n];
    int count[n];
    int f = 0, flips = 0, nperm = 0, checksum = 0; 
    int i, k, r;    

    r = n;
    while (r > 0) { 
        i = 0;  
        while (r != 1) { count[r-1] = r; r -= 1; }  
        while (i < n) { perm[i] = perm1[i]; i += 1; }     
        
        // Count flips and update max and checksum
        f = 0;
        k = perm[0];  
        while (k != 0) {
            i = 0;  
            while (2*i < k) {          
                int t = perm[i]; perm[i] = perm[k-i]; perm[k-i] = t;  
                i += 1;           
            }
            k = perm[0];
            f += 1;   
        }  
        if (f > flips) flips = f;         
        if ((nperm & 0x1) == 0) checksum += f; else checksum -= f;
         
        // Use incremental change to generate another permutation   
        bool more = true;
        while (more) {   
            if (r == n) {
            printf("%d\n", checksum);
            return flips; 
        }     
        int p0 = perm1[0];
        i = 0;
        while (i < r) {
            int j = i + 1;
            perm1[i] = perm1[j];
            i = j;            
        }
        perm1[r] = p0; 
         
        count[r] -= 1;         
        if (count[r] > 0) more = false; else r += 1; 
        }
        nperm += 1;                
    }         
    return flips;
}

int main(int argc, char *argv[])
{
    int n = argc > 1 ? atoi(argv[1]) : 7;
    printf("Pfannkuchen(%d) = %d\n", n, fannkuch(n));
    return 0;
}
