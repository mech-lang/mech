/* The Computer Language Benchmarks Game
 * https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
**
** regex-dna program contributed by Mike Pall
** converted from regex-dna program by Jeremy Zerfas
**
*/

#define __USE_STRING_INLINES
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <pcre.h>


typedef struct fbuf {
  char *buf;
  size_t size, len;
} fbuf_t;

static void fb_init(fbuf_t *b)
{
  b->buf = NULL;
  b->len = b->size = 0;
}

static char *fb_need(fbuf_t *b, size_t need)
{
  need += b->len;
  if (need > b->size) {
    if (b->size == 0) b->size = need;
    else while (need > b->size) b->size += b->size;
    if (!(b->buf = realloc(b->buf, b->size))) exit(1);
  }
  return b->buf+b->len;
}

#define FB_MINREAD	(3<<16)

/* Read all of a stdio stream into dst buffer. */
static size_t fb_readall(fbuf_t *dst, FILE *fp)
{
  char *dp;
  int n;
  for (dp = fb_need(dst, FB_MINREAD);
       (n = fread(dp, 1, dst->size-dst->len, fp)) > 0;
       dp = fb_need(dst, FB_MINREAD)) dst->len += n;
  if (ferror(fp)) exit(1);
  return dst->len;
}

/* Substitute pattern p with replacement r, copying from src to dst buffer. */
static size_t fb_subst(fbuf_t *dst, fbuf_t *src, const char *p, const char *r)
{
  pcre *re;
  pcre_extra *re_ex;
  const char *re_e;
  char *dp;
  int re_eo, m[3], pos, rlen, clen;
  if (!(re = pcre_compile(p, 0, &re_e, &re_eo, NULL))) exit(1);
  re_ex = pcre_study(re, PCRE_STUDY_JIT_COMPILE, &re_e);
  for (dst->len = 0, rlen = strlen(r), pos = 0;
       pcre_exec(re, re_ex, src->buf, src->len, pos, 0, m, 3) >= 0;
       pos = m[1]) {
    clen = m[0]-pos;
    dp = fb_need(dst, clen+rlen);
    dst->len += clen+rlen;
    memcpy(dp, src->buf+pos, clen);
    memcpy(dp+clen, r, rlen);
  }
  clen = src->len-pos;
  dp = fb_need(dst, clen);
  dst->len += clen;
  memcpy(dp, src->buf+pos, clen);
  return dst->len;
}

/* Count all matches with pattern p in src buffer. */
static int fb_countmatches(fbuf_t *src, const char *p)
{
  pcre *re;
  pcre_extra *re_ex;
  const char *re_e;
  int re_eo, m[3], pos, count;
  if (!(re = pcre_compile(p, 0, &re_e, &re_eo, NULL))) exit(1);
  re_ex = pcre_study(re, PCRE_STUDY_JIT_COMPILE, &re_e);
  for (count = 0, pos = 0;
       pcre_exec(re, re_ex, src->buf, src->len, pos, 0, m, 3) >= 0;
       pos = m[1]) count++;
  return count;
}

static const char *variants[] = {
  "agggtaaa|tttaccct",         "[cgt]gggtaaa|tttaccc[acg]",
  "a[act]ggtaaa|tttacc[agt]t", "ag[act]gtaaa|tttac[agt]ct",
  "agg[act]taaa|ttta[agt]cct", "aggg[acg]aaa|ttt[cgt]ccct",
  "agggt[cgt]aa|tt[acg]accct", "agggta[cgt]a|t[acg]taccct",
  "agggtaa[cgt]|[acg]ttaccct", NULL
};

static const char *subst[] = {
  "tHa[Nt]", "<4>",
  "aND|caN|Ha[DS]|WaS", "<3>",
  "a[NSt]|BY", "<2>",
  "<[^>]*>", "|",
  "\\|[^|][^|]*\\|", "-",
  NULL
};

int main(int argc, char **argv)
{
  fbuf_t seq[2];
  const char **pp;
  size_t ilen, clen, slen;
  int flip;
  fb_init(&seq[0]);
  fb_init(&seq[1]);
  ilen = fb_readall(&seq[0], stdin);
  clen = fb_subst(&seq[1], &seq[0], ">.*|\n", "");
  for (pp = variants; *pp; pp++)
    printf("%s %d\n", *pp, fb_countmatches(&seq[1], *pp));

  for (slen = 0, flip = 1, pp = subst; *pp; pp += 2, flip = 1-flip)
    slen = fb_subst(&seq[1-flip], &seq[flip], *pp, pp[1]);

  printf("\n%zu\n%zu\n%zu\n", ilen, clen, slen);
  return 0;
}
