import std/[os, strutils]

let n = parseInt(paramStr(1))
doAssert n > 0
let stride = (n + 7) div 8
let scale = 2.0 / float64(n)
var image = newString(n * stride)
for y in 0 ..< n:
  let ci = float64(y) * scale - 1.0
  for x in 0 ..< n:
    let cr = float64(x) * scale - 1.5
    var zr = 0.0
    var zi = 0.0
    var tr = 0.0
    var ti = 0.0
    for iteration in 0 ..< 50:
      zi = (zr + zr) * zi + ci
      zr = tr - ti + cr
      tr = zr * zr
      ti = zi * zi
      if tr + ti > 4.0:
        break
    if tr + ti <= 4.0:
      let offset = y * stride + x div 8
      image[offset] = char(ord(image[offset]) or (128 shr (x mod 8)))
stdout.write("P4\n", n, " ", n, "\n")
stdout.write(image)
