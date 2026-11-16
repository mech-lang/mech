//! Allocation policy for compiler executables, separate from library/runtime consumers.

#[global_allocator]
static COMPILER_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;
