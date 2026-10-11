"""General PCRE2 compile/count/global-substitute bindings."""
from std.ffi import external_call

def compile(imm pattern: String) -> UInt64:
    var bytes = pattern.as_bytes()
    return external_call["regex_new", UInt64](bytes.unsafe_ptr(), len(bytes))
def close(handle: UInt64):
    external_call["regex_close", NoneType](handle)
def count(handle: UInt64, imm bytes: List[UInt8]) -> UInt64:
    return external_call["regex_count", UInt64](handle, bytes.unsafe_ptr(), len(bytes))
def replace(handle: UInt64, imm bytes: List[UInt8], imm replacement: String) -> List[UInt8]:
    var text = replacement.as_bytes()
    var buffer = external_call["regex_replace", UInt64](
        handle, bytes.unsafe_ptr(), len(bytes), text.unsafe_ptr(), len(text))
    var size = external_call["buffer_size", Int](buffer)
    var address = external_call["buffer_data", UInt64](buffer)
    var result = List[UInt8](unsafe_uninit_length=size)
    _ = external_call["memcpy", UInt64](result.unsafe_ptr(), address, size)
    external_call["buffer_close", NoneType](buffer)
    return result^
