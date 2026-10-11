/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   contributed by Mr Ledrug
   modified by Attila Balazs
*/

#define _GNU_SOURCE
#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <ctype.h>
#include <unistd.h>
#include <string.h>

char *pairs = "ATCGGCTAUAMKRYWWSSYRKMVBHDDHBVNN\n\n";
char tbl[128];

void process(char *from, char *to) {
   while (*from++ != '\n');

   size_t len = to - from;
   size_t off = 60 - (len % 61);

   if (off) {
      char *m;
      for (m = from + 60 - off; m < to; m += 61) {
         memmove(m + 1, m, off);
         *m = '\n';
      }
   }

   char c;
   for (to--; from <= to; from++, to--)
      c = tbl[(int)*from], *from = tbl[(int)*to], *to = c;
}

int main() {
   char *s;
   for (s = pairs; *s; s += 2) {
      tbl[toupper(s[0])] = s[1];
      tbl[tolower(s[0])] = s[1];
   }


   const size_t _1M = 1024 * 1024;
   size_t buflen = 8 * 1024, len, end = 0;
   char *buf = malloc(buflen);

   int in = fileno(stdin);
   while ((len = read(in, buf + end, buflen - 256 - end))) {
      if (len < 0) break;
      end += len;
      if (end >= buflen - 256) {
         buflen = (buflen >= _1M) ? buflen + _1M : buflen * 2;
         buf = realloc(buf, buflen);
      }
   }
   buf[end] = '>';

   char *from, *to = buf + end - 1;
   while (1) {
      for (from = to; *from != '>'; from--);

      process(from, to);

      to = from - 1;
      if (to < buf) break;
   }

   write(fileno(stdout), buf, end);
   free(buf);

   return 0;
}
