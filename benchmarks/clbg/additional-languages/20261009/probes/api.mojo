from std.ffi import external_call, c_int
from std.memory import alloc
from std.sys import argv

def main() raises:
    var data = List[UInt8](capacity=100)
    for i in range(100):
        data.append(UInt8(0))
    var got = external_call["read", Int](0, data.unsafe_ptr(), 100)
    _ = external_call["write", Int](1, data.unsafe_ptr(), got)
    _ = external_call["printf", c_int, num_fixed_args=1]("%.9f\n".as_c_string_span().ptr(), Float64(1.25))
