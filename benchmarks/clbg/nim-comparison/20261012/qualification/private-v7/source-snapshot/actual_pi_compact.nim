import std/[os, strutils, options]
import bigints

let n = parseInt(paramStr(1))
var q = initBigInt(1)
var r = initBigInt(0)
var t = initBigInt(1)
var k = 1
var printed = 0
var line = ""
let audit = open(getEnv("MECH_NIM_WORK_TRACE"), fmWrite)
audit.writeLine("PI_NIM1|compact|", n)
while printed < n:
  audit.writeLine("STATE|", q, "|", r, "|", t, "|", k, "|", printed)
  let numerator3 = q * initBigInt(3) + r
  let numerator4 = q * initBigInt(4) + r
  let digit3 = numerator3 div t
  let digit4 = numerator4 div t
  audit.writeLine("DIV|", numerator3, "|", digit3, "|", numerator4, "|", digit4)
  if digit3 == digit4:
    let digit = toInt[int](digit3).get()
    line.add(char(ord('0') + digit))
    inc printed
    if printed mod 10 == 0 or printed == n:
      line.add(repeat(' ', (10 - printed mod 10) mod 10))
      stdout.write(line, "\t:", printed, "\n")
      line.setLen(0)
    r = (r - digit3 * t) * initBigInt(10)
    q *= initBigInt(10)
  else:
    let factor = initBigInt(2 * k + 1)
    r = (q * initBigInt(2) + r) * factor
    q *= initBigInt(k)
    t *= factor
    inc k

audit.writeLine("END_PI_NIM1|", printed)
audit.close()
