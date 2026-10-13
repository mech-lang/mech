import std/[os, strutils]
import "../../support/gmp_ops"

let n = parseInt(paramStr(1))
var q: Mpz
var r: Mpz
var t: Mpz
var numerator3: Mpz
var numerator4: Mpz
var digit3: Mpz
var digit4: Mpz
var temporary: Mpz
init(q)
init(r)
init(t)
init(numerator3)
init(numerator4)
init(digit3)
init(digit4)
init(temporary)
setUi(q, 1)
setUi(t, 1)
var k = 1
var printed = 0
var line = newStringOfCap(64 * 1024)
while printed < n:
  mulUi(numerator3, q, 3)
  add(numerator3, numerator3, r)
  mulUi(numerator4, q, 4)
  add(numerator4, numerator4, r)
  divide(digit3, numerator3, t)
  divide(digit4, numerator4, t)
  if compare(digit3, digit4) == 0:
    let digit = getUi(digit3)
    line.add(char(culong(ord('0')) + digit))
    inc printed
    if printed mod 10 == 0 or printed == n:
      line.add(repeat(' ', (10 - printed mod 10) mod 10))
      line.add("\t:" & $printed & "\n")
    if line.len >= 60 * 1024:
      stdout.write(line)
      line.setLen(0)
    mulUi(temporary, t, digit)
    sub(r, r, temporary)
    mulUi(r, r, 10)
    mulUi(q, q, 10)
  else:
    let factor = culong(2 * k + 1)
    addmulUi(r, q, 2)
    mulUi(r, r, factor)
    mulUi(q, q, culong(k))
    mulUi(t, t, factor)
    inc k
stdout.write(line)
clear(q)
clear(r)
clear(t)
clear(numerator3)
clear(numerator4)
clear(digit3)
clear(digit4)
clear(temporary)
