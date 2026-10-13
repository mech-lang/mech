/* Lua 5.1/LuaJIT ABI-only compatibility. No application or pi logic. */
#ifndef CLBG_LUA51_COMPAT_H
#define CLBG_LUA51_COMPAT_H
#include <lua.h>
#include <lauxlib.h>
#include <gmp.h>
#include <limits.h>
#include <math.h>
#if LUA_VERSION_NUM != 501
#error This compatibility header is specifically for the recorded LuaJIT 5.1 ABI.
#endif
static int clbg_is_integer(lua_State *L,int index) {
    if(lua_type(L,index)!=LUA_TNUMBER) return 0;
    lua_Number number=lua_tonumber(L,index);
    return isfinite(number) && number>=-0x1p63 && number<0x1p63 && floor(number)==number;
}
static lua_Integer clbg_check_integer(lua_State *L,int index) {
    lua_Number number=luaL_checknumber(L,index);
    luaL_argcheck(L,isfinite(number) && number>=-0x1p63 && number<0x1p63 && floor(number)==number,index,"finite integral signed-word value required");
    return (lua_Integer)number;
}
static void *clbg_new_userdata(lua_State *L,size_t size,int uservalues) {
    luaL_argcheck(L,uservalues==0,1,"Lua 5.1 does not support userdata uservalue slots");
    return lua_newuserdata(L,size);
}
typedef struct { lua_State *state; char *storage; } Clbg_Buffer;
static char *clbg_buffer_init(lua_State *L,Clbg_Buffer *buffer,size_t size) {
    buffer->state=L;buffer->storage=(char *)lua_newuserdata(L,size);
    return buffer->storage;
}
static void clbg_buffer_result(Clbg_Buffer *buffer,size_t size) {
    /* The temporary userdata retains ownership across an allocation error in
       lua_pushlstring. Returning one result discards the temporary stack slot. */
    lua_pushlstring(buffer->state,buffer->storage,size);
}
static void clbg_push_gmp_integer(lua_State *L,lua_Integer number) {
    /* Lua 5.1 stores doubles only. Keep the ordinary mpz_get_d truncation for
       integers beyond its exact range instead of claiming Lua 5.5 semantics. */
    if(number>=-9007199254740992LL && number<=9007199254740992LL) {
        lua_pushnumber(L,(lua_Number)number);
    } else {
        mpz_t value;mpz_init_set_si(value,(long)number);
        double converted=mpz_get_d(value);mpz_clear(value);
        lua_pushnumber(L,(lua_Number)converted);
    }
}
#define lua_isinteger clbg_is_integer
#define luaL_checkinteger clbg_check_integer
#define lua_newuserdatauv clbg_new_userdata
#define luaL_Buffer Clbg_Buffer
#define luaL_buffinitsize clbg_buffer_init
#define luaL_pushresultsize clbg_buffer_result
#define lua_pushinteger clbg_push_gmp_integer
#endif
