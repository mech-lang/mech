"""General native byte IO and fixed decimal output; no benchmark kernels."""
from std.ffi import external_call, c_int, CStringSpan

def print_fixed(x: Float64, digits: Int):
    write_string(fixed_string(x, digits) + "\n")

def fixed_string(x: Float64, digits: Int) -> String:
    var buffer = List[Int8](unsafe_uninit_length=128)
    _ = external_call["snprintf", c_int, num_fixed_args=3](
        buffer.unsafe_ptr(), 128, "%.*f".as_c_string_span().ptr(), c_int(digits), x)
    return String(StringSlice(unsafe_from_utf8=CStringSpan(unsafe_from_ptr=buffer.unsafe_ptr())))

def read_bytes() -> List[UInt8]:
    var bytes = List[UInt8]()
    var used = 0
    while True:
        bytes.resize(used + 65536, UInt8(0))
        var got = external_call["read", Int](0, bytes.unsafe_ptr() + used, 65536)
        if got <= 0:
            break
        used += got
    bytes.resize(used, UInt8(0))
    return bytes^

def write_bytes(imm bytes: List[UInt8]):
    var written = 0
    while written < len(bytes):
        written += external_call["write", Int](
            1, bytes.unsafe_ptr() + written, len(bytes) - written)

def write_string(imm text: String):
    _ = external_call["write", Int](1, text.as_bytes().unsafe_ptr(), len(text.as_bytes()))
