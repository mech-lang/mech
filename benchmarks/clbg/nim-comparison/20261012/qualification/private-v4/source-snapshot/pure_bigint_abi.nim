import std/options
import bigints
let a = initBigInt(17)
let b = initBigInt(5)
doAssert a div b == initBigInt(3)
doAssert a mod b == initBigInt(2)
doAssert initBigInt(-17) div b == initBigInt(-3)
doAssert (a * b) - a == initBigInt(68)
doAssert toInt[int](initBigInt(9)).get() == 9
echo "pure-bigint-contract-ok"
