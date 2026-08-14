//! Runtime execution topology.
//!
//! Source execution, queries, reactive turns, context preflight, module
//! execution, live registration, host input, and input-driver lifecycle each
//! live in their owning module.

mod input_drivers;
mod query;

#[cfg(test)]
mod tests;
