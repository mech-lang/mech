from std.sys import argv
from integer import BigInt, multiply, multiply_add
from uintmap import UIntMap
from common import wrap, write_bytes, write_string
from regex import compile, close, count, replace

def unsigned(imm text: String) -> UInt64:
    var n: UInt64 = 0
    for c in text.as_bytes():
        n = n * 10 + UInt64(c - 48)
    return n

def main() raises:
    var args = argv()
    if args[1] == "arithmetic":
        var a = BigInt(args[2])
        var b = BigInt(args[3])
        var factor = Int(args[4])
        write_string((a + b).to_string() + "\n")
        write_string((a - b).to_string() + "\n")
        write_string((a * factor).to_string() + "\n")
        write_string((a // b).to_string() + "\n")
        var duplicate = a.copy()
        multiply(duplicate.handle, duplicate.handle, 7)
        write_string(a.to_string() + "\n")
        write_string(duplicate.to_string() + "\n")
        var fused = BigInt()
        multiply_add(fused.handle, a.handle, UInt64(factor), b.handle)
        write_string(fused.to_string() + "\n")
        var alias_a = a.copy()
        multiply_add(alias_a.handle, alias_a.handle, UInt64(factor), b.handle)
        write_string(alias_a.to_string() + "\n")
        var alias_b = b.copy()
        multiply_add(alias_b.handle, a.handle, UInt64(factor), alias_b.handle)
        write_string(alias_b.to_string() + "\n")
        var alias_both = a.copy()
        multiply_add(alias_both.handle, alias_both.handle, UInt64(factor), alias_both.handle)
        write_string(alias_both.to_string() + "\n")
    elif args[1] == "map":
        var table = UIntMap()
        for i in range(2, len(args)):
            table.increment(unsigned(args[i]))
        for i in range(2, len(args)):
            write_string(String(table.get(unsigned(args[i]))) + "\n")
        table.set(0, 123)
        table.set(18446744073709551615, 456)
        write_string(String(table.get(0)) + "\n")
        write_string(String(table.get(18446744073709551615)) + "\n")
    elif args[1] == "wrap":
        var bytes = List[UInt8](args[3].as_bytes())
        write_bytes(wrap(Int(args[2]), bytes))
    elif args[1] == "regex":
        var bytes = List[UInt8](args[3].as_bytes())
        var rx = compile(args[2])
        write_string(String(count(rx, bytes)) + "\n")
        write_bytes(replace(rx, bytes, args[4]))
        close(rx)
