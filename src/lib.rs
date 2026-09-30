//! Crypto portfolio manager — backend library.
//!
//! Layers (dependencies only point inwards):
//! - `domain`: plain data and the traits ("ports") for storage and prices.
//! - `usecases`: the business rules, depending only on `domain` traits.
//! - `infra`: SQLite and HTTP price-provider implementations of those traits.
//! - `api`: Axum routes/handlers — thin glue over the use cases.
//!
//! `main.rs` and the tools in `src/bin/` are small programs built on this
//! library.

pub mod api;
pub mod config;
pub mod domain;
pub mod infra;
pub mod usecases;

#[cfg(test)]
mod tests;
