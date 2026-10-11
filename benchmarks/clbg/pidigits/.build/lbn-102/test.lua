-- test bn library
local bn=require"bn"

------------------------------------------------------------------------------
print(bn.version)
print()

p=bn.aprime(100)
	print("p",p)
q=bn.aprime(250)
	print("q",q)
m=p*q
	print("m",m)
mx=(p-1)*(q-1)
	print("mx",mx)
e=bn.new"0x10001"
	print("e",e)
d=bn.invmod(e,mx)
	print("d",d)
	assert(bn.mulmod(e,d,mx):isone())
t=bn.new(2)
x=bn.powmod(t,e,m)
y=bn.powmod(x,d,m)
	assert(t==y)
	print()

message="The quick brown fox jumps over the lazy dog"
	print("message as text")
	print(message)

encoded=bn.text(message)
	print("encoded message")
	print(encoded)
	assert(encoded<m)
	assert(message==bn.totext(encoded))

x=bn.powmod(encoded,e,m)
	print("encrypted message")
	print(x)

y=bn.powmod(x,d,m)
	print("decrypted message as number")
	print(y)
	assert(y==encoded)
y=bn.totext(y)
	print("decrypted message as text")
	print(y)
	assert(y==message)

------------------------------------------------------------------------------
print()
print(bn.version)
