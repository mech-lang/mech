// General PCRE2 service in OTP re: compile and one indexed search only.
pub type Pattern

@external(erlang, "clbg_regex_ffi", "compile")
pub fn compile(pattern: String) -> Pattern

@external(erlang, "clbg_regex_ffi", "search")
pub fn search(pattern: Pattern, data: BitArray, offset: Int) -> Result(#(Int, Int), Nil)
