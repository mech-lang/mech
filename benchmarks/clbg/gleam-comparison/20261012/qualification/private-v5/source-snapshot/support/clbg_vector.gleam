// Reusable immutable tuple storage; all indexing and algorithms stay in Gleam.
pub type Vector(value)

@external(erlang, "erlang", "list_to_tuple")
pub fn from_list(values: List(value)) -> Vector(value)

@external(erlang, "erlang", "tuple_to_list")
pub fn to_list(values: Vector(value)) -> List(value)

@external(erlang, "clbg_vector_ffi", "get")
pub fn get(values: Vector(value), index: Int) -> value

@external(erlang, "clbg_vector_ffi", "set")
pub fn set(values: Vector(value), index: Int, value: value) -> Vector(value)
