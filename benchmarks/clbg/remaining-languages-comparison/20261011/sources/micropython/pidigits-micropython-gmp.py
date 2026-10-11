# Ordinary general GMP arithmetic; LFT control and both extractions in Python.
import sys
import ffi
lib = ffi.open("/opt/homebrew/opt/gmp/lib/libgmp.10.dylib")
init = lib.func("v","__gmpz_init_set_si","pl")
multiply = lib.func("v","__gmpz_mul_ui","ppL")
add = lib.func("v","__gmpz_add","ppp")
subtract = lib.func("v","__gmpz_sub","ppp")
divide = lib.func("v","__gmpz_fdiv_q","ppp")
compare = lib.func("i","__gmpz_cmp","pp")
value = lib.func("l","__gmpz_get_si","p")
clear = lib.func("v","__gmpz_clear","p")
def integer(value):
    result = bytearray(16)
    init(result,value)
    return result
q = integer(1)
r = integer(0)
t = integer(1)
a = integer(0)
b = integer(0)
temporary = integer(0)
k = 1
count = 0
limit = int(sys.argv[1])
line = ""
while count < limit:
    multiply(temporary,q,3)
    add(temporary,temporary,r)
    divide(a,temporary,t)
    multiply(temporary,q,4)
    add(temporary,temporary,r)
    divide(b,temporary,t)
    if compare(a,b) == 0:
        digit = value(a)
        line += str(digit)
        count += 1
        if count%10 == 0 or count == limit:
            print(line+" "*(10-len(line))+"\t:"+str(count))
            line = ""
        multiply(temporary,t,digit)
        subtract(r,r,temporary)
        multiply(r,r,10)
        multiply(q,q,10)
    else:
        multiply(temporary,q,2)
        add(r,temporary,r)
        multiply(r,r,2*k+1)
        multiply(q,q,k)
        multiply(t,t,2*k+1)
        k += 1
for number in [q,r,t,a,b,temporary]:
    clear(number)
