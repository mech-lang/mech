//! Reservation-backed canonical payload ownership.
//!
//! The allocator and managed container implementations are introduced with
//! the canonical-payload cutover. This module exists at the core ownership
//! boundary so payload authority cannot migrate into the engine or a host.
