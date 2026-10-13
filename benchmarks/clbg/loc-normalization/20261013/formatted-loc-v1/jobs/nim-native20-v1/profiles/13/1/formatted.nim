# Compatibility/name adapter over the privately pinned public nim-gmp header.
# The upstream csize spelling predates Nim's csize_t spelling.
type csize = csize_t
include gmp/header
type Mpz* = mpz_t
template init*(value: var Mpz) = mpz_init(value)
template clear*(value: var Mpz) = mpz_clear(value)
template setUi*(value: var Mpz, input: culong) = mpz_set_ui(value, input)
template add*(destination: var Mpz, left: var Mpz, right: var Mpz) = mpz_add(
    destination, left, right)
template sub*(destination: var Mpz, left: var Mpz, right: var Mpz) = mpz_sub(
    destination, left, right)
template mulUi*(destination: var Mpz, left: var Mpz,
    right: culong) = mpz_mul_ui(destination, left, right)
template addmulUi*(destination: var Mpz, left: var Mpz,
    right: culong) = mpz_addmul_ui(destination, left, right)
template divide*(destination: var Mpz, numerator: var Mpz,
    denominator: var Mpz) = mpz_tdiv_q(destination, numerator, denominator)
template compare*(left: var Mpz, right: var Mpz): cint = mpz_cmp(left, right)
template getUi*(value: var Mpz): culong = mpz_get_ui(value)
