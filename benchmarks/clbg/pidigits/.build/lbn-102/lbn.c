/*
* lbn.c
* big-number library for Lua based on OpenSSL bn
* Luiz Henrique de Figueiredo <lhf@tecgraf.puc-rio.br>
* 05 Oct 2026 09:13:56
* This code is hereby placed in the public domain and also under the MIT license
*/

#include <stdlib.h>
#include <openssl/bn.h>
#include <openssl/err.h>
#include <openssl/rand.h>

#include "lua.h"
#include "lauxlib.h"
#include "mycompat.h"

#define MYNAME		"bn"
#define MYVERSION	MYNAME " library for " LUA_VERSION " / Oct 2026 / "\
			"based on " OPENSSL_VERSION_TEXT
#define MYTYPE		MYNAME " bignumber"

static BN_CTX *ctx=NULL;

static void report(lua_State *L, int rc)
{
 if (rc!=0) return;
 const char *message=ERR_reason_error_string(ERR_peek_last_error());
 luaL_error(L,"(%s) %s",MYNAME,message);
}

static BIGNUM *Pnew(lua_State *L)
{
 BIGNUM *x=BN_new();
 report(L,x!=NULL);
 luaL_boxpointer(L,x);
 luaL_setmetatable(L,MYTYPE);
 return x;
}

static BIGNUM *Pget(lua_State *L, int i)
{
 switch (lua_type(L,i))
 {
  case LUA_TNUMBER:
  case LUA_TSTRING:
  {
   BIGNUM *x=Pnew(L);
   const char *s=lua_tostring(L,i);
   if (s[0]=='0' && (s[1]=='x' || s[1]=='X'))
    BN_hex2bn(&x,s+2);
   else 
    BN_dec2bn(&x,s);
   lua_replace(L,i);
   return x;
  }
  default:
   return luaL_unboxpointer(L,i,MYTYPE);
 }
 return NULL;
}

static int Lbits(lua_State *L)			/** bits(x) */
{
 BIGNUM *a=Pget(L,1);
 lua_pushinteger(L,BN_num_bits(a));
 return 1;
}

static int Ltostring(lua_State *L)		/** tostring(x) */
{
 BIGNUM *a=Pget(L,1);
 char *s=BN_bn2dec(a);
 lua_pushstring(L,s);
 OPENSSL_free(s);
 return 1;
}

static int Ltohex(lua_State *L)			/** tohex(x) */
{
 BIGNUM *a=Pget(L,1);
 char *s=BN_bn2hex(a);
 lua_pushstring(L,s);
 OPENSSL_free(s);
 return 1;
}

static int Ltotext(lua_State *L)		/** totext(x) */
{
 BIGNUM *a=Pget(L,1);
 int n=BN_num_bytes(a);
 void *s=malloc(n);
 if (s==NULL) return 0;
 BN_bn2bin(a,s);
 lua_pushlstring(L,s,n);
 free(s);
 return 1;
}

static int Ltonumber(lua_State *L)		/** tonumber(x) */
{
 Ltostring(L);
 lua_pushnumber(L,lua_tonumber(L,-1));
 return 1;
}

static int Liszero(lua_State *L)		/** iszero(x) */
{
 BIGNUM *a=Pget(L,1);
 lua_pushboolean(L,BN_is_zero(a));
 return 1;
}

static int Lisone(lua_State *L)			/** isone(x) */
{
 BIGNUM *a=Pget(L,1);
 lua_pushboolean(L,BN_is_one(a));
 return 1;
}

static int Lisodd(lua_State *L)			/** isodd(x) */
{
 BIGNUM *a=Pget(L,1);
 lua_pushboolean(L,BN_is_odd(a));
 return 1;
}

static int Lisnegative(lua_State *L)		/** isnegative(x) */
{
 BIGNUM *a=Pget(L,1);
 lua_pushboolean(L,BN_is_negative(a));
 return 1;
}

static int Lnew(lua_State *L)			/** new(x) */
{
 Pget(L,1);
 lua_settop(L,1);
 return 1;
}

static int Ltext(lua_State *L)			/** text(t) */
{
 size_t l;
 const char *s=luaL_checklstring(L,1,&l);
 BIGNUM *x=Pnew(L);
 BN_bin2bn((const unsigned char *)s,l,x);
 return 1;
}

static int Lcompare(lua_State *L)		/** compare(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 lua_pushinteger(L,BN_cmp(a,b));
 return 1;
}

static int Leq(lua_State *L)
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 lua_pushboolean(L,BN_cmp(a,b)==0);
 return 1;
}

static int Lle(lua_State *L)
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 lua_pushboolean(L,BN_cmp(a,b)<=0);
 return 1;
}

static int Llt(lua_State *L)
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 lua_pushboolean(L,BN_cmp(a,b)<0);
 return 1;
}

static int Lsqr(lua_State *L)			/** sqr(x) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *c=Pnew(L);
 report(L,BN_sqr(c,a,ctx));
 return 1;
}

static void Pneg(lua_State *L, BIGNUM *a)
{
 BIGNUM *c=Pnew(L);
 report(L,BN_sub(c,c,a));
}

static int Lneg(lua_State *L)			/** neg(x) */
{
 BIGNUM *a=Pget(L,1);
 Pneg(L,a);
 return 1;
}

static int Labs(lua_State *L)			/** abs(x) */
{
 BIGNUM *a=Pget(L,1);
 if (BN_is_negative(a))
  Pneg(L,a);
 else
  lua_settop(L,1);
 return 1;
}

static int Ladd(lua_State *L)			/** add(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *c=Pnew(L);
 report(L,BN_add(c,a,b));
 return 1;
}

static int Lsub(lua_State *L)			/** sub(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *c=Pnew(L);
 report(L,BN_sub(c,a,b));
 return 1;
}

static int Lmul(lua_State *L)			/** mul(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *c=Pnew(L);
 report(L,BN_mul(c,a,b,ctx));
 return 1;
}

static int Ldiv(lua_State *L)			/** div(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *q=Pnew(L);
 BIGNUM *r=NULL;
 report(L,BN_div(q,r,a,b,ctx));
 return 1;
}

static int Lmod(lua_State *L)			/** mod(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *r=Pnew(L);
 report(L,BN_nnmod(r,a,b,ctx));
 return 1;
}

static int Lquotrem(lua_State *L)		/** quotrem(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *q=Pnew(L);
 BIGNUM *r=Pnew(L);
 report(L,BN_div(q,r,a,b,ctx));
 return 2;
}

static int Lgcd(lua_State *L)			/** gcd(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *c=Pnew(L);
 report(L,BN_gcd(c,a,b,ctx));
 return 1;
}

static int Lpow(lua_State *L)			/** pow(x,y) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *c=Pnew(L);
 report(L,BN_exp(c,a,b,ctx));
 return 1;
}

static int Laddmod(lua_State *L)		/** addmod(x,y,m) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *m=Pget(L,3);
 BIGNUM *c=Pnew(L);
 report(L,BN_mod_add(c,a,b,m,ctx));
 return 1;
}

static int Lsubmod(lua_State *L)		/** submod(x,y,m) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *m=Pget(L,3);
 BIGNUM *c=Pnew(L);
 report(L,BN_mod_sub(c,a,b,m,ctx));
 return 1;
}

static int Lmulmod(lua_State *L)		/** mulmod(x,y,m) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *m=Pget(L,3);
 BIGNUM *c=Pnew(L);
 report(L,BN_mod_mul(c,a,b,m,ctx));
 return 1;
}

static int Lpowmod(lua_State *L)		/** powmod(x,y,m) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *b=Pget(L,2);
 BIGNUM *m=Pget(L,3);
 BIGNUM *c=Pnew(L);
 report(L,BN_mod_exp(c,a,b,m,ctx));
 return 1;
}

static int Lsqrmod(lua_State *L)		/** sqrmod(x) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *m=Pget(L,2);
 BIGNUM *c=Pnew(L);
 report(L,BN_mod_sqr(c,a,m,ctx));
 return 1;
}

static int Linvmod(lua_State *L)		/** invmod(x) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *m=Pget(L,2);
 BIGNUM *c=Pnew(L);
 report(L,BN_mod_inverse(c,a,m,ctx)!=NULL);
 return 1;
}

static int Lsqrtmod(lua_State *L)		/** sqrtmod(x) */
{
 BIGNUM *a=Pget(L,1);
 BIGNUM *m=Pget(L,2);
 BIGNUM *c=Pnew(L);
 report(L,BN_mod_sqrt(c,a,m,ctx)!=NULL);
 return 1;
}

static int Lrandom(lua_State *L)		/** random(bits) */
{
 int bits=luaL_optinteger(L,1,64);
 BIGNUM *x=Pnew(L);
 report(L,BN_rand(x,bits,-1,0));
 return 1;
}

static int Laprime(lua_State *L)		/** aprime(bits) */
{
 int bits=luaL_optinteger(L,1,64);
 BIGNUM *x=Pnew(L);
 report(L,BN_generate_prime(x,bits,0,NULL,NULL,NULL,NULL)!=NULL);
 return 1;
}

static int Lisprime(lua_State *L)		/** isprime(x) */
{
 BIGNUM *a=Pget(L,1);
 int rc=BN_is_prime_fasttest(a,BN_prime_checks,NULL,ctx,NULL,1);
 report(L,rc>=0);
 lua_pushboolean(L,rc);
 return 1;
}

static int Lgc(lua_State *L)
{
 BIGNUM *a=Pget(L,1);
 BN_free(a);
 lua_pushnil(L);
 lua_setmetatable(L,1);
 return 0;
}

static const luaL_Reg R[] =
{
	{ "__add",	Ladd	},		/** __add(x,y) */
	{ "__div",	Ldiv	},		/** __div(x,y) */
	{ "__eq",	Leq	},		/** __eq(x,y) */
	{ "__gc",	Lgc	},
	{ "__le",	Lle	},		/** __le(x,y) */
	{ "__lt",	Llt	},		/** __lt(x,y) */
	{ "__mod",	Lmod	},		/** __mod(x,y) */
	{ "__mul",	Lmul	},		/** __mul(x,y) */
	{ "__pow",	Lpow	},		/** __pow(x,y) */
	{ "__sub",	Lsub	},		/** __sub(x,y) */
	{ "__tostring",	Ltostring},		/** __tostring(x) */
	{ "__unm",	Lneg	},		/** __unm(x) */
	{ "abs",	Labs	},
	{ "add",	Ladd	},
	{ "addmod",	Laddmod	},
	{ "aprime",	Laprime	},
	{ "bits",	Lbits	},
	{ "compare",	Lcompare},
	{ "div",	Ldiv	},
	{ "gcd",	Lgcd	},
	{ "invmod",	Linvmod	},
	{ "isnegative",	Lisnegative},
	{ "isodd",	Lisodd	},
	{ "isone",	Lisone	},
	{ "isprime",	Lisprime},
	{ "iszero",	Liszero	},
	{ "mod",	Lmod	},
	{ "mul",	Lmul	},
	{ "mulmod",	Lmulmod	},
	{ "neg",	Lneg	},
	{ "new",	Lnew	},
	{ "pow",	Lpow	},
	{ "powmod",	Lpowmod	},
	{ "quotrem",	Lquotrem},
	{ "random",	Lrandom	},
	{ "sqr",	Lsqr	},
	{ "sqrmod",	Lsqrmod	},
	{ "sqrtmod",	Lsqrtmod},
	{ "sub",	Lsub	},
	{ "submod",	Lsubmod	},
	{ "text",	Ltext	},
	{ "tohex",	Ltohex	},
	{ "tonumber",	Ltonumber},
	{ "tostring",	Ltostring},
	{ "totext",	Ltotext	},
	{ NULL,		NULL	}
};

LUALIB_API int luaopen_bn(lua_State *L)
{
 ctx=BN_CTX_new();
 ERR_load_BN_strings();
 RAND_seed(MYVERSION,sizeof(MYVERSION));
 luaL_newmetatable(L,MYTYPE);
 luaL_setfuncs(L,R,0);
 lua_pushliteral(L,"version");			/** version */
 lua_pushliteral(L,MYVERSION);
 lua_settable(L,-3);
 lua_pushliteral(L,"__index");
 lua_pushvalue(L,-2);
 lua_settable(L,-3);
 return 1;
}
