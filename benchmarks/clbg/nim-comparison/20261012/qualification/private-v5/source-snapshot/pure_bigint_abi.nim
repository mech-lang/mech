import std/options
import bigints
let a = initBigInt(17)
let b = initBigInt(5)
doAssert a div b == initBigInt(3)
doAssert a mod b == initBigInt(2)
# BigInts 1.1.0 defines floor division and divisor-signed modulo.
doAssert initBigInt(-17) div b == initBigInt(-4)
doAssert initBigInt(-17) mod b == initBigInt(3)
doAssert a div initBigInt(-5) == initBigInt(-4)
doAssert a mod initBigInt(-5) == initBigInt(-3)
doAssert initBigInt(-17) div initBigInt(-5) == initBigInt(3)
doAssert initBigInt(-17) mod initBigInt(-5) == initBigInt(-2)
doAssert (a * b) - a == initBigInt(68)
doAssert toInt[int](initBigInt(9)).get() == 9
echo "pure-bigint-contract-ok"
