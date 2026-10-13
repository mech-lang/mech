// Generic typed task ownership and join, not a benchmark executor.
pub type Task(value)

@external(erlang, "clbg_process_ffi", "start")
pub fn start(work: fn() -> value) -> Task(value)

@external(erlang, "clbg_process_ffi", "join")
pub fn join(task: Task(value)) -> value
