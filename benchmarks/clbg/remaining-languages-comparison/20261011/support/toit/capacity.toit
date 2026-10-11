import bignum show Bignum
main:
  bytes := ByteArray 70000
  bytes[0] = 1
  large := Bignum.with-bytes false bytes
  result := large * 2
  print result.stringify.size
  digit := Bignum.hex "09"
  print (int.parse digit.stringify)
