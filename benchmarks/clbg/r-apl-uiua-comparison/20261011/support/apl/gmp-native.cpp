// General arbitrary-integer binding for GNU APL's published native interface.
// Interface conventions derived from GNU APL 2.0 src/native/template.hh (GPL-3.0-or-later).
// This file is GPL-3.0-or-later. It contains no benchmark algorithm.
#include "Native_interface.hh"
#include "Value.hh"
#include <gmp.h>
#include <memory>
#include <unordered_map>
#include <cstring>

class NativeFunction;
struct Integer {
    mpz_t value;
    explicit Integer(long initial) { mpz_init_set_si(value, initial); }
    ~Integer() { mpz_clear(value); }
};
static std::unordered_map<long, std::unique_ptr<Integer>> integers;
static long next_id = 1;
static mpz_ptr lookup(long id) {
    auto found = integers.find(id);
    if (found == integers.end()) DOMAIN_ERROR;
    return found->second->value;
}
static long scalar(const Value_P &v) {
    if (v->element_count() != 1) LENGTH_ERROR;
    return v->get_cfirst().get_near_int();
}
static Token result(long value) {
    Value_P z = IntScalar(value, LOC);
    z->check_value(LOC);
    return Token(TOK_APL_VALUE1, z);
}
static Fun_signature signature() { return SIG_Z_A_F2_B; }
static bool close_binding(Cause, const NativeFunction *) { integers.clear(); return true; }
static Token monad(Value_P x, Value_P b, const NativeFunction *) {
    long operation = scalar(x);
    if (operation == 0) {
        long id = next_id++;
        integers.emplace(id, std::unique_ptr<Integer>(new Integer(scalar(b))));
        return result(id);
    }
    if (operation == 1) {
        long id = scalar(b);
        lookup(id);
        integers.erase(id);
        return result(0);
    }
    if (operation == 8) {
        if (b->element_count() != 2) LENGTH_ERROR;
        return result(mpz_cmp(lookup(b->get_cravel(0).get_near_int()),
                              lookup(b->get_cravel(1).get_near_int())));
    }
    if (operation == 9) {
        mpz_srcptr source = lookup(scalar(b));
        if (!mpz_fits_slong_p(source)) DOMAIN_ERROR;
        return result(mpz_get_si(source));
    }
    DOMAIN_ERROR;
}
static Token dyad(Value_P a, Value_P x, Value_P b, const NativeFunction *) {
    long destination_id = scalar(a);
    mpz_ptr destination = lookup(destination_id);
    long operation = scalar(x);
    if (operation == 2) mpz_set(destination, lookup(scalar(b)));
    else if (operation == 3) mpz_set_si(destination, scalar(b));
    else {
        if (b->element_count() != 2) LENGTH_ERROR;
        mpz_srcptr left = lookup(b->get_cravel(0).get_near_int());
        mpz_srcptr right = lookup(b->get_cravel(1).get_near_int());
        if (operation == 4) mpz_add(destination, left, right);
        else if (operation == 5) mpz_sub(destination, left, right);
        else if (operation == 6) mpz_mul(destination, left, right);
        else if (operation == 7) {
            if (mpz_sgn(right) == 0) DOMAIN_ERROR;
            mpz_fdiv_q(destination, left, right);
        } else DOMAIN_ERROR;
    }
    return result(destination_id);
}
extern "C" void *get_function_mux(const char *name) {
    if (!std::strcmp(name, "get_signature")) return reinterpret_cast<void *>(&signature);
    if (!std::strcmp(name, "close_fun")) return reinterpret_cast<void *>(&close_binding);
    if (!std::strcmp(name, "eval_XB")) return reinterpret_cast<void *>(&monad);
    if (!std::strcmp(name, "eval_AXB")) return reinterpret_cast<void *>(&dyad);
    return nullptr;
}
