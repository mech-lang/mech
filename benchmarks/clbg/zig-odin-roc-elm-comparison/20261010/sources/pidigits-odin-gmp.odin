package main
import "core:fmt"
import "core:os"
import "core:strconv"
foreign import gmp "external:libgmp.10.dylib"
Integer :: struct { allocated, size: i32, limbs: rawptr }
@(default_calling_convention="c", link_prefix="__gmpz_")
foreign gmp {
    init :: proc(x: ^Integer) ---
    clear :: proc(x: ^Integer) ---
    set_ui :: proc(x: ^Integer, value: u64) ---
    mul_ui :: proc(x, a: ^Integer, value: u64) ---
    add :: proc(x,a,b: ^Integer) ---
    sub :: proc(x,a,b: ^Integer) ---
    fdiv_q :: proc(x,a,b: ^Integer) ---
    cmp :: proc(a,b: ^Integer) -> i32 ---
    get_ui :: proc(a: ^Integer) -> u64 ---
}
main :: proc() {
    n := strconv.parse_int(os.args[1]) or_else 30
    values: [7]Integer
    for &value in values { init(&value) }
    defer for &value in values { clear(&value) }
    q := &values[0]
    r := &values[1]
    t := &values[2]
    a := &values[3]
    b := &values[4]
    first := &values[5]
    second := &values[6]
    set_ui(q,1)
    set_ui(t,1)
    k := 0
    produced := 0
    line: [10]u8
    for produced < n {
        mul_ui(a,q,3)
        add(a,a,r)
        fdiv_q(first,a,t)
        mul_ui(b,q,4)
        add(b,b,r)
        fdiv_q(second,b,t)
        if cmp(first,second) == 0 {
            digit := get_ui(first)
            line[produced%10] = u8(digit)+'0'
            produced += 1
            if produced%10 == 0 || produced == n {
                used := 10 if produced%10 == 0 else produced%10
                for i in used..<10 { line[i] = ' ' }
                fmt.printf("%s\t:%d\n", transmute(string)line[:], produced)
            }
            mul_ui(a,t,digit)
            sub(r,r,a)
            mul_ui(r,r,10)
            mul_ui(q,q,10)
        } else {
            k += 1
            multiplier := u64(2*k+1)
            mul_ui(a,q,2)
            add(r,r,a)
            mul_ui(r,r,multiplier)
            mul_ui(q,q,u64(k))
            mul_ui(t,t,multiplier)
        }
    }
}
