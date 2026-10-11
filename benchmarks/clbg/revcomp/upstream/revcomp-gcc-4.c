/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
 *
 * contributed by Bob W 
 */

#include <stdio.h>
#include <stdlib.h>

#define JBFSIZE 82      // line input buffer size
#define QBFSIZE 5200     // output buffer initial size
#define Z16     "\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"
#define V32     "\0TVGH\0\0CD\0\0M\0KN\0\0\0YSA\0BW\0R\0\0\0\0\0\0"
#define VALL    Z16 Z16 Z16 Z16 V32 V32 Z16 Z16 Z16 Z16 Z16 Z16 Z16 Z16

int errex(char *s, int n) {      // error message+value, return 1
  fprintf(stderr,"\n*** Error: %s [%d]!\n", s, n);
  return 1;
}

int main () {                    // ***** main *****
  char *pj, *pq, *pr;            // buffer pointers: inp,out,/out
  char *jjj = malloc(JBFSIZE);   // allocate input line buffer
  char *qqq = malloc(QBFSIZE);   // output buffer (dyn. size)
  char *pqstop = qqq+QBFSIZE;    // end-of-buffer pointer
  char xtab[256] = VALL;         // char conversion table

  if (!jjj || !qqq)
    return errex("Buffer allocation", !jjj + !qqq);
  pj = fgets(jjj,JBFSIZE,stdin);         // fetch 1st line
  if (!pj)
    return errex("No input data",0);
  if (*jjj != '>')
    return errex("1st char not '>'", 0);

  while (pj) {                           // MAIN LOOP: process data
    fputs(jjj, stdout);                  // output ID line

    for (pq=qqq+1, pr=pqstop; ; pq++) {  // LOOP: fill output buffer
      pj = fgets(jjj, JBFSIZE, stdin);   // get line from stdin
      if (!pj || (*jjj=='>'))  break;    // EOF or new ID line
      if (pr <= (pq+61)) {               // need to resize buffer
        char *newstop = pqstop + 12777888;
        char *newptr  = realloc(qqq, newstop-qqq);
        if (!newptr)
          return errex("Out of memory", 0);
        if (newptr != qqq) {             // new base: adj. pointers
          size_t x = newptr-qqq;         // offset for pointer update
          pq+=x;  pr+=x;  qqq+=x;
          newstop+=x;  pqstop+=x;
        }
        pr = __builtin_memmove(newstop-(pqstop-pr), pr, pqstop-pr);
        pqstop = newstop;                // buffer resize complete
      }
      while (*pj) {                      // LOOP: conv. & revert line
        char c = xtab[(unsigned char)(*pj++)];
        if (c)                           // conversion valid
          *(--pr) = c;
      }
    }

    for (pq = qqq; pr<pqstop; ) {        // LOOP: format output
      size_t x = (pqstop-pr)<60 ? pqstop-pr : 60;
      __builtin_memmove(pq,pr,x);        // move line to free space
      pr+=x;  pq+=x;  *(pq++) = 0xA;     // adjust pointers, add LF
    }
    fwrite(qqq, 1, pq-qqq, stdout);      // output converted data
  }
  return 0;
}
