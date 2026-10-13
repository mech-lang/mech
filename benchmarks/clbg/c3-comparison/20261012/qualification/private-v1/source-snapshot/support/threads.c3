// Counted general worker-launch glue around the pinned C3 standard library.
// std::thread owns native thread metadata and joins; owners outlive all joins.
module clbg::threads;
import std::thread;

alias Handle = thread::Thread;
alias Worker = thread::ThreadFn;

fn void start(Handle* handle, Worker callback, void* argument)
{
    handle.create(callback, argument)!!;
}

fn void join(Handle handle)
{
    handle.join()!!;
}
