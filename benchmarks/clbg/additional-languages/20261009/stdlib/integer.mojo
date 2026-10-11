"""Bindings to the shared general GMP arithmetic library."""
from std.ffi import external_call, c_int

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
