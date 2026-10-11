"""Bindings to the shared general GMP arithmetic library."""
from std.ffi import external_call, c_int, CStringSpan

def integer(n: UInt64) -> UInt64:
    return external_call["z_new", UInt64](n)
def close(h: UInt64):
    external_call["z_close", NoneType](h)
def add(dst: UInt64, a: UInt64, b: UInt64):
    external_call["z_add", NoneType](dst, a, b)
def subtract(dst: UInt64, a: UInt64, b: UInt64):
    external_call["z_sub", NoneType](dst, a, b)
def multiply(dst: UInt64, a: UInt64, b: UInt64):
    external_call["z_mul_ui", NoneType](dst, a, b)
def quotient(dst: UInt64, a: UInt64, b: UInt64):
    external_call["z_div", NoneType](dst, a, b)
def small(a: UInt64) -> UInt64:
    return external_call["z_ui", UInt64](a)
def compare(a: UInt64, b: UInt64) -> Int:
    return Int(external_call["z_cmp", c_int](a, b))

def multiply_add(dst: UInt64, a: UInt64, factor: UInt64, b: UInt64):
    external_call["z_mul_add_ui", NoneType](dst, a, factor, b)

struct BigInt(Copyable, Movable):
    """Owned, signed unbounded integers with ordinary arithmetic operators."""
    var handle: UInt64

    def __init__(out self, value: Int = 0):
        self.handle = integer(UInt64(0) - UInt64(value) if value < 0 else UInt64(value))
        if value < 0:
            external_call["z_neg", NoneType](self.handle, self.handle)

    def __init__(out self, *, copy: Self):
        self.handle = integer(0)
        external_call["z_copy", NoneType](self.handle, copy.handle)

    def __init__(out self, imm text: String):
        self.handle = integer(0)
        var bytes = text.as_bytes()
        external_call["z_set_decimal", NoneType](self.handle, bytes.unsafe_ptr(), len(bytes))

    def __deinit__(deinit self):
        close(self.handle)

    def __add__(self, other: Self) -> Self:
        var result = Self()
        add(result.handle, self.handle, other.handle)
        return result^

    def __sub__(self, other: Self) -> Self:
        var result = Self()
        subtract(result.handle, self.handle, other.handle)
        return result^

    def __mul__(self, factor: Int) -> Self:
        var result = Self()
        external_call["z_mul_si", NoneType](result.handle, self.handle, factor)
        return result^

    def __floordiv__(self, other: Self) -> Self:
        var result = Self()
        quotient(result.handle, self.handle, other.handle)
        return result^

    def __gt__(self, other: Self) -> Bool:
        return compare(self.handle, other.handle) > 0

    def to_int(self) -> Int:
        return external_call["z_i64", Int](self.handle)

    def to_string(self) -> String:
        var size = external_call["z_decimal_size", Int](self.handle)
        var buffer = List[Int8](unsafe_uninit_length=size)
        external_call["z_decimal", NoneType](self.handle, buffer.unsafe_ptr())
        return String(StringSlice(unsafe_from_utf8=CStringSpan(unsafe_from_ptr=buffer.unsafe_ptr())))
