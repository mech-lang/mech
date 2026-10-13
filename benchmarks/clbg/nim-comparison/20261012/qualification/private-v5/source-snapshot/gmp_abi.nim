import "support/gmp_ops"
doAssert sizeof(Mpz) == 16
doAssert alignof(Mpz) == 8
var a: Mpz
var b: Mpz
var q: Mpz
var zero: Mpz
var expected: Mpz
init(a)
init(b)
init(q)
init(zero)
init(expected)
setUi(a, 17)
setUi(b, 5)
divide(q, a, b)
doAssert getUi(q) == 3
divide(a, a, b)
doAssert getUi(a) == 3
setUi(a, 17)
sub(a, zero, a)
divide(q, a, b)
setUi(expected, 3)
sub(expected, zero, expected)
doAssert compare(q, expected) == 0
setUi(a, 7)
mulUi(a, a, 3)
doAssert getUi(a) == 21
add(a, a, a)
doAssert getUi(a) == 42
addmulUi(a, a, 2)
doAssert getUi(a) == 126
clear(a)
clear(b)
clear(q)
clear(zero)
clear(expected)
echo "gmp-lp64-alias-sign-contract-ok"
