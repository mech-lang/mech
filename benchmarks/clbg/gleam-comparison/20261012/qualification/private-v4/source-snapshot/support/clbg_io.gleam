// Counted generic byte I/O and numeric formatting; no benchmark kernel.
@external(erlang, "clbg_io_ffi", "argument")
pub fn argument() -> Int

@external(erlang, "clbg_io_ffi", "read_all")
pub fn read_all() -> BitArray

@external(erlang, "clbg_io_ffi", "write")
pub fn write(value: BitArray) -> Nil

@external(erlang, "clbg_io_ffi", "text")
pub fn text(value: String) -> Nil

@external(erlang, "clbg_io_ffi", "decimal")
pub fn decimal(value: Float, precision: Int) -> String

@external(erlang, "erlang", "byte_size")
pub fn size(value: BitArray) -> Int

@external(erlang, "binary", "at")
pub fn byte(value: BitArray, index: Int) -> Int

@external(erlang, "binary", "part")
pub fn slice(value: BitArray, index: Int, length: Int) -> BitArray

@external(erlang, "clbg_io_ffi", "join")
pub fn join(values: List(BitArray)) -> BitArray

@external(erlang, "clbg_io_ffi", "bytes")
pub fn bytes(values: List(Int)) -> BitArray

@external(erlang, "erlang", "integer_to_binary")
pub fn integer(value: Int) -> String

@external(erlang, "math", "sqrt")
pub fn sqrt(value: Float) -> Float

@external(erlang, "erlang", "float")
pub fn float(value: Int) -> Float
