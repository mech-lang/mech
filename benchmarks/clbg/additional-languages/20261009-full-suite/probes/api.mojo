from std.collections import Dict
from std.memory import UnsafePointer

def main() raises:
    var data: List[UInt8] = [65, 67, 10]
    var text = String(from_utf8=data)
    print(text.find("C"))
    for line in text.splitlines():
        print(line)
    var d = Dict[UInt64, UInt64]()
    d.setdefault(3, 0) += 1
    print(d[3])
    var p = data.unsafe_ptr()
    var v = p.load[width=2]()
    print(v.reversed())
