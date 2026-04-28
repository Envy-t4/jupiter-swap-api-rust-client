//! Jupiter Swap API v2 client types.
//!
//! Implements `/order` (managed quote + transaction) and `/build` (raw swap
//! instructions, Metis-only). The `/execute` endpoint is intentionally not
//! covered — landing is expected to be handled by the caller via their own
//! RPC / Jito stack.

pub mod build;
pub mod common;
pub mod order;
