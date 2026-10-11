#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"
#include "gmp.h"

/*
Math::GMP, a Perl module for high-speed arbitrary size integer
calculations
Copyright (C) 2000 James H. Turner

This library is free software; you can redistribute it and/or
modify it under the terms of the GNU Library General Public
License as published by the Free Software Foundation; either
version 2 of the License, or (at your option) any later version.

This library is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
Library General Public License for more details.

You should have received a copy of the GNU Library General Public
License along with this library; if not, write to the Free
Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA

You can contact the author at chip@redhat.com, chipt@cpan.org, or by mail:

Chip Turner
Red Hat Inc.
2600 Meridian Park Blvd
Durham, NC 27713
*/

#define SWAP_GMP if (swap) { mpz_t* t = m; m = n; n = t; }

/*
 * mpz_rootrem() has bug with negative first argument before 5.1.0
 */
static int need_rootrem_workaround(mpz_t* m, unsigned long n) {
    /* workaround only valid for n odd (n even should be an error) */
    if ((n & 1) == 0)
        return 0;

    /* workaround only relevant for m negative */
    if (mpz_sgn(*m) >= 0)
        return 0;

    /* workaround only needed for gmp_version < 5.1.0 */
    if ((gmp_version[0] && gmp_version[1] != '.')            /* >= 10.0.0 */
        || (gmp_version[0] > '5')                            /* >=  6.0.0 */
        || (gmp_version[0] == '5' && gmp_version[2] != '0')  /* >=  5.1.0 */
    )
        return 0;

    return 1;
}

static int
not_here(char *s)
{
    croak("%s not implemented on this architecture", s);
    return -1;
}

#if 0
static double
constant(char *name, int arg)
{
    errno = 0;
    switch (*name) {
    }
    errno = EINVAL;
    return 0;

not_there:
    errno = ENOENT;
    return 0;
}
#endif

mpz_t *
pv2gmp(char* pv)
{
	mpz_t* z;
	SV* sv;

	z = malloc (sizeof(mpz_t));
	mpz_init_set_str(*z, pv, 0);
	sv = sv_newmortal();
	sv_setref_pv(sv, "Math::GMP", (void*)z);
	return z;
}

mpz_t *
sv2gmp(SV* sv)
{
	char* pv;

	/* MAYCHANGE in perlguts.pod - bug in perl */
	if (SvGMAGICAL(sv)) mg_get(sv);

	if (SvROK(sv) && sv_derived_from(sv, "Math::GMP")) {
		IV tmp = SvIV((SV*)SvRV(sv));
		return (mpz_t *)tmp;
	}

	pv = SvPV_nolen(sv);
	return pv2gmp(pv);
}


MODULE = Math::GMP		PACKAGE = Math::GMP
PROTOTYPES: ENABLE

mpz_t *
new_from_scalar(s)
	char *	s

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init_set_str(*RETVAL, s, 0);
  OUTPUT:
    RETVAL

mpz_t *
new_from_scalar_with_base(s, b)
        char *  s
        int b

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init_set_str(*RETVAL, s, b);
  OUTPUT:
    RETVAL

void
destroy(n)
	mpz_t *n

  CODE:
    mpz_clear(*n);
    free(n);

SV *
_gmp_build_version()
  CODE:
    char buf[] = STRINGIFY(__GNU_MP_VERSION)
        "." STRINGIFY(__GNU_MP_VERSION_MINOR)
        "." STRINGIFY(__GNU_MP_VERSION_PATCHLEVEL);
    RETVAL = newSV(6);
    scan_vstring(buf, buf + sizeof(buf), RETVAL);
  OUTPUT:
    RETVAL

SV *
_gmp_lib_version()
  CODE:
    const char* v = gmp_version;
    RETVAL = newSV(6);
    scan_vstring(v, v + strlen(v), RETVAL);
  OUTPUT:
    RETVAL

SV *
stringify(n)
	mpz_t *	n

  PREINIT:
    int len;

  CODE:
    len = mpz_sizeinbase(*n, 10);
    {
      char *buf;
      buf = malloc(len + 2);
      mpz_get_str(buf, 10, *n);
      RETVAL = newSVpv(buf, strlen(buf));
      free(buf);
    }
  OUTPUT:
    RETVAL


SV *
get_str_gmp(n, b)
       mpz_t * n
       int b

  PREINIT:
    int len;

  CODE:
    len = mpz_sizeinbase(*n, b);
    {
        char *buf;
        buf = malloc(len + 2);
        mpz_get_str(buf, b, *n);
        RETVAL = newSVpv(buf, strlen(buf));
        free(buf);
    }
  OUTPUT:
    RETVAL

int
sizeinbase_gmp(n, b)
       mpz_t * n
       int b

  CODE:
    RETVAL = mpz_sizeinbase(*n, b);
  OUTPUT:
    RETVAL

unsigned long
uintify(n)
       mpz_t * n

  CODE:
    RETVAL = mpz_get_ui(*n);
  OUTPUT:
    RETVAL

void
add_ui_gmp(n, v)
       mpz_t * n
       unsigned long v

  CODE:
    mpz_add_ui(*n, *n, v);


long
intify(n)
	mpz_t *	n

  CODE:
    RETVAL = mpz_get_si(*n);
  OUTPUT:
    RETVAL

mpz_t *
mul_2exp_gmp(n, e)
       mpz_t * n
       unsigned long e

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_mul_2exp(*RETVAL, *n, e);
  OUTPUT:
    RETVAL

mpz_t *
div_2exp_gmp(n, e)
       mpz_t * n
       unsigned long e

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_div_2exp(*RETVAL, *n, e);
  OUTPUT:
    RETVAL


mpz_t *
powm_gmp(n, exp, mod)
       mpz_t * n
       mpz_t * exp
       mpz_t * mod

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_powm(*RETVAL, *n, *exp, *mod);
  OUTPUT:
    RETVAL


mpz_t *
mmod_gmp(a, b)
       mpz_t * a
       mpz_t * b

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_mmod(*RETVAL, *a, *b);
  OUTPUT:
    RETVAL


mpz_t *
mod_2exp_gmp(in, cnt)
       mpz_t * in
       unsigned long cnt

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_mod_2exp(*RETVAL, *in, cnt);
  OUTPUT:
    RETVAL


mpz_t *
op_add(m,n,swap)
	mpz_t *		m
	mpz_t *		n
	bool		swap

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_add(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL


mpz_t *
op_sub(m,n,swap)
	mpz_t *		m
	mpz_t *		n
	bool		swap

  CODE:
    SWAP_GMP
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_sub(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL


mpz_t *
op_mul(m,n,swap)
	mpz_t *		m
	mpz_t *		n
	bool		swap

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_mul(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL


mpz_t *
op_div(m,n,swap)
	mpz_t *		m
	mpz_t *		n
	bool		swap

  CODE:
    SWAP_GMP
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_div(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL


void
bdiv(m,n)
	mpz_t *		m
	mpz_t *		n

  PREINIT:
    mpz_t * quo;
    mpz_t * rem;
  PPCODE:
    quo = malloc (sizeof(mpz_t));
    rem = malloc (sizeof(mpz_t));
    mpz_init(*quo);
    mpz_init(*rem);
    mpz_tdiv_qr(*quo, *rem, *m, *n);
  EXTEND(SP, 2);
  PUSHs(sv_setref_pv(sv_newmortal(), "Math::GMP", (void*)quo));
  PUSHs(sv_setref_pv(sv_newmortal(), "Math::GMP", (void*)rem));



mpz_t *
op_mod(m,n,swap)
	mpz_t *		m
	mpz_t *		n
	bool		swap

  CODE:
    SWAP_GMP
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_mod(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL

mpz_t *
bmodinv(m,n)
	mpz_t *		m
	mpz_t *		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_invert(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL


int
op_spaceship(m,n,swap)
	mpz_t *		m
	mpz_t *		n
	bool		swap

  PREINIT:
    int i;
  CODE:
    i = mpz_cmp(*m, *n);
    if (swap) {
        i = -i;
    }
    RETVAL = (i < 0) ? -1 : (i > 0) ? 1 : 0;
  OUTPUT:
    RETVAL

int
op_eq(m,n,swap)
	mpz_t*		m
	mpz_t*		n
	bool		swap

  PREINIT:
    int i;
  CODE:
    i = mpz_cmp(*m, *n);
    RETVAL = (i == 0) ? 1 : 0;
  OUTPUT:
    RETVAL

int
legendre(m, n)
        mpz_t *         m
        mpz_t *         n

  CODE:
    RETVAL = mpz_legendre(*m, *n);
  OUTPUT:
    RETVAL

int
jacobi(m, n)
        mpz_t *         m
        mpz_t *         n

  CODE:
    RETVAL = mpz_jacobi(*m, *n);
  OUTPUT:
    RETVAL

int
probab_prime(m, reps)
    mpz_t * m
    int reps

    CODE:
        RETVAL = mpz_probab_prime_p(*m, reps);
    OUTPUT:
        RETVAL

mpz_t *
op_pow(m,n)
	mpz_t *		m
	long		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
/*    fprintf(stderr, "n is %ld\n", n);*/
    mpz_pow_ui(*RETVAL, *m, n);
  OUTPUT:
    RETVAL


mpz_t *
bgcd(m,n)
	mpz_t *		m
	mpz_t *		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_gcd(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL


mpz_t *
blcm(m,n)
	mpz_t *		m
	mpz_t *		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_lcm(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL


mpz_t *
fibonacci(n)
	long		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_fib_ui(*RETVAL, n);
  OUTPUT:
    RETVAL


mpz_t *
band(m,n, ...)
	mpz_t *		m
	mpz_t *		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_and(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL

mpz_t *
bxor(m,n, ...)
	mpz_t *		m
	mpz_t *		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_xor(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL

mpz_t *
bior(m,n, ...)
	mpz_t *		m
	mpz_t *		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_ior(*RETVAL, *m, *n);
  OUTPUT:
    RETVAL

mpz_t *
blshift(m,n,swap)
	mpz_t *		m
	mpz_t *		n
	bool		swap

  CODE:
    SWAP_GMP
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_mul_2exp(*RETVAL, *m, mpz_get_ui(*n));
  OUTPUT:
    RETVAL

mpz_t *
brshift(m,n,swap)
	mpz_t *		m
	mpz_t *		n
	bool		swap

  CODE:
    SWAP_GMP
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_div_2exp(*RETVAL, *m, mpz_get_ui(*n));
  OUTPUT:
    RETVAL

mpz_t *
bfac(n)
	long		n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_fac_ui(*RETVAL, n);
  OUTPUT:
    RETVAL


mpz_t *
gmp_copy(m)
	mpz_t *		m

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init_set(*RETVAL, *m);
  OUTPUT:
    RETVAL

int
gmp_tstbit(m,n)
	mpz_t *		m
	long		n

  CODE:
    RETVAL = mpz_tstbit(*m,n);
  OUTPUT:
    RETVAL

mpz_t *
broot(m,n)
	mpz_t *		m
	unsigned long	n

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_root(*RETVAL, *m, n);
  OUTPUT:
    RETVAL

void
brootrem(m,n)
	mpz_t *		m
	unsigned long	n

  PREINIT:
    mpz_t * root;
    mpz_t * remainder;
  PPCODE:
    root = malloc (sizeof(mpz_t));
    remainder = malloc (sizeof(mpz_t));
    mpz_init(*root);
    mpz_init(*remainder);
    if (need_rootrem_workaround(m, n)) {
        /* Older libgmp have bugs for negative m, but if we need to we can
         * work on abs(m) then negate the results.
         */
        mpz_neg(*root, *m);
        mpz_rootrem(*root, *remainder, *root, n);
        mpz_neg(*root, *root);
        mpz_neg(*remainder, *remainder);
    } else {
        mpz_rootrem(*root, *remainder, *m, n);
    }
  EXTEND(SP, 2);
  PUSHs(sv_setref_pv(sv_newmortal(), "Math::GMP", (void*)root));
  PUSHs(sv_setref_pv(sv_newmortal(), "Math::GMP", (void*)remainder));

mpz_t *
bsqrt(m)
	mpz_t *		m

  CODE:
    RETVAL = malloc (sizeof(mpz_t));
    mpz_init(*RETVAL);
    mpz_sqrt(*RETVAL, *m);
  OUTPUT:
    RETVAL

void
bsqrtrem(m)
	mpz_t *		m

  PREINIT:
    mpz_t * sqrt;
    mpz_t * remainder;
  PPCODE:
    sqrt = malloc (sizeof(mpz_t));
    remainder = malloc (sizeof(mpz_t));
    mpz_init(*sqrt);
    mpz_init(*remainder);
    mpz_sqrtrem(*sqrt, *remainder, *m);
  EXTEND(SP, 2);
  PUSHs(sv_setref_pv(sv_newmortal(), "Math::GMP", (void*)sqrt));
  PUSHs(sv_setref_pv(sv_newmortal(), "Math::GMP", (void*)remainder));

int
is_perfect_power(m)
	mpz_t *		m

  CODE:
    RETVAL = mpz_perfect_power_p(*m) ? 1 : 0;
  OUTPUT:
    RETVAL

int
is_perfect_square(m)
	mpz_t *		m

  CODE:
    RETVAL = mpz_perfect_square_p(*m) ? 1 : 0;
  OUTPUT:
    RETVAL

