/* Ordinary mpz compatibility bindings for Lua. No application algorithms.
 * c-gmp preserves the published LGMP calling conventions: mpz_set and the
 * add/sub-multiple operations take destination first; ordinary add/mul/div
 * take operand(s),destination.
 * bn supplies the published immutable arithmetic API using the same GMP library.
 */
#include <lua.h>
#include <lauxlib.h>
#include <gmp.h>
#include <limits.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>
#define MPZ_MT "clbg.general.mpz"
typedef struct { mpz_t value; int live; } Cell;
static Cell *cell(lua_State *L, int index) {
    Cell *z=(Cell *)luaL_checkudata(L,index,MPZ_MT);
    luaL_argcheck(L,z->live,index,"cleared integer");
    return z;
}
static Cell *new_cell(lua_State *L) {
    Cell *z=(Cell *)lua_newuserdatauv(L,sizeof(Cell),0);
    mpz_init(z->value);
    z->live=1;
    luaL_setmetatable(L,MPZ_MT);
    return z;
}
static long signed_word(lua_State *L,int index) {
    lua_Integer n=luaL_checkinteger(L,index);
    if (n<LONG_MIN || n>LONG_MAX) luaL_argerror(L,index,"integer outside signed word range");
    return (long)n;
}
static unsigned long unsigned_word(lua_State *L,int index) {
    lua_Integer n=luaL_checkinteger(L,index);
    if (n<0 || (unsigned long long)n>ULONG_MAX) luaL_argerror(L,index,"integer outside unsigned word range");
    return (unsigned long)n;
}
static int collect(lua_State *L) {
    Cell *z=(Cell *)luaL_checkudata(L,1,MPZ_MT);
    if(z->live) { mpz_clear(z->value);z->live=0; }
    return 0;
}
static int equal(lua_State *L) { lua_pushboolean(L,mpz_cmp(cell(L,1)->value,cell(L,2)->value)==0);return 1; }
static int less(lua_State *L) { lua_pushboolean(L,mpz_cmp(cell(L,1)->value,cell(L,2)->value)<0);return 1; }
static int less_equal(lua_State *L) { lua_pushboolean(L,mpz_cmp(cell(L,1)->value,cell(L,2)->value)<=0);return 1; }
static void push_numeric(lua_State *L,mpz_srcptr value) {
    /* Lua 5.1 had one numeric type. Canonical integral values now use Lua's
       native integer representation; this preserves the old io.write output.
       Values outside that exact range retain mpz_get_d's ordinary conversion. */
    if(mpz_fits_slong_p(value)) lua_pushinteger(L,(lua_Integer)mpz_get_si(value));
    else lua_pushnumber(L,(lua_Number)mpz_get_d(value));
}
static int get_number(lua_State *L) { push_numeric(L,cell(L,1)->value);return 1; }
static int text(lua_State *L) {
    Cell *z=cell(L,1);
    size_t length=mpz_sizeinbase(z->value,10)+2;
    luaL_Buffer buffer;
    char *out=luaL_buffinitsize(L,&buffer,length);
    mpz_get_str(out,10,z->value);
    luaL_pushresultsize(&buffer,strlen(out));
    return 1;
}
static int init_set_d(lua_State *L) {
    double input=luaL_checknumber(L,1);
    luaL_argcheck(L,isfinite(input),1,"finite number required");
    Cell *z=new_cell(L);mpz_set_d(z->value,input);return 1;
}
static int copy_into(lua_State *L) { mpz_set(cell(L,1)->value,cell(L,2)->value);lua_settop(L,1);return 1; }
static int add_into(lua_State *L) { mpz_add(cell(L,3)->value,cell(L,1)->value,cell(L,2)->value);lua_settop(L,3);return 1; }
static int mul_si_into(lua_State *L) { mpz_mul_si(cell(L,3)->value,cell(L,1)->value,signed_word(L,2));lua_settop(L,3);return 1; }
static int mul_ui_into(lua_State *L) { mpz_mul_ui(cell(L,3)->value,cell(L,1)->value,unsigned_word(L,2));lua_settop(L,3);return 1; }
static int addmul_ui_into(lua_State *L) { mpz_addmul_ui(cell(L,1)->value,cell(L,2)->value,unsigned_word(L,3));lua_settop(L,1);return 1; }
static int submul_ui_into(lua_State *L) { mpz_submul_ui(cell(L,1)->value,cell(L,2)->value,unsigned_word(L,3));lua_settop(L,1);return 1; }
static int tdiv_into(lua_State *L) {
    Cell *a=cell(L,1),*b=cell(L,2),*out=cell(L,3);
    luaL_argcheck(L,mpz_sgn(b->value)!=0,2,"zero denominator");
    mpz_tdiv_q(out->value,a->value,b->value);lua_settop(L,3);return 1;
}
static int fdiv_into(lua_State *L) {
    Cell *a=cell(L,1),*b=cell(L,2),*out=cell(L,3);
    luaL_argcheck(L,mpz_sgn(b->value)!=0,2,"zero denominator");
    mpz_fdiv_q(out->value,a->value,b->value);lua_settop(L,3);return 1;
}
static int number(lua_State *L) {
    int type=lua_type(L,1);
    if(type==LUA_TSTRING) {
        const char *input=luaL_checkstring(L,1);Cell *z=new_cell(L);
        luaL_argcheck(L,mpz_set_str(z->value,input,10)==0,1,"invalid decimal integer");
    } else if(type==LUA_TNUMBER && lua_isinteger(L,1)) {
        long input=signed_word(L,1);Cell *z=new_cell(L);mpz_set_si(z->value,input);
    } else {
        double input=luaL_checknumber(L,1);luaL_argcheck(L,isfinite(input),1,"finite number required");
        Cell *z=new_cell(L);mpz_set_d(z->value,input);
    }
    return 1;
}
static int add(lua_State *L) { Cell *a=cell(L,1),*b=cell(L,2),*out=new_cell(L);mpz_add(out->value,a->value,b->value);return 1; }
static int subtract(lua_State *L) { Cell *a=cell(L,1),*b=cell(L,2),*out=new_cell(L);mpz_sub(out->value,a->value,b->value);return 1; }
static int multiply(lua_State *L) {
    Cell *a=cell(L,1);
    if(lua_isnumber(L,2)) { long b=signed_word(L,2);Cell *out=new_cell(L);mpz_mul_si(out->value,a->value,b); }
    else { Cell *b=cell(L,2),*out=new_cell(L);mpz_mul(out->value,a->value,b->value); }
    return 1;
}
static int divide(lua_State *L) {
    Cell *a=cell(L,1),*b=cell(L,2);
    luaL_argcheck(L,mpz_sgn(b->value)!=0,2,"zero denominator");
    Cell *out=new_cell(L);mpz_tdiv_q(out->value,a->value,b->value);return 1;
}
static const luaL_Reg meta[]={ {"__gc",collect},{"__eq",equal},{"__lt",less},{"__le",less_equal},{"__tostring",text},{NULL,NULL} };
static const luaL_Reg mutable_api[]={
 {"mpz_init_set_d",init_set_d},{"mpz_get_d",get_number},{"mpz_set",copy_into},
 {"mpz_add",add_into},{"mpz_mul_si",mul_si_into},{"mpz_mul_ui",mul_ui_into},
 {"mpz_addmul_ui",addmul_ui_into},{"mpz_submul_ui",submul_ui_into},
 {"mpz_tdiv_q",tdiv_into},{"mpz_fdiv_q",fdiv_into},{NULL,NULL}
};
static const luaL_Reg immutable_api[]={ {"number",number},{"tonumber",get_number},{"add",add},{"sub",subtract},{"mul",multiply},{"div",divide},{NULL,NULL} };
static void metatable(lua_State *L) {
    if(luaL_newmetatable(L,MPZ_MT)) luaL_setfuncs(L,meta,0);
    lua_pop(L,1);
}
static int populate(lua_State *L) {
    luaL_checktype(L,1,LUA_TTABLE);
    lua_pushvalue(L,1);luaL_setfuncs(L,mutable_api,0);return 1;
}
int luaopen_gmp(lua_State *L) { metatable(L);lua_pushcfunction(L,populate);return 1; }
int luaopen_c_gmp(lua_State *L) { return luaopen_gmp(L); }
int luaopen_bn(lua_State *L) { metatable(L);luaL_newlib(L,immutable_api);return 1; }
