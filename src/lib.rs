//! dermestes — finds Python abstractions that have not earned their keep.
//!
//! The binary is a thin shell over [`cli`]. [`guide`] serves the agent-facing
//! pages from `docs/src/guide/`. A run goes [`config`] → [`discovery`] →
//! [`index`] → [`one_impl`] (through [`resolve`]) → optional [`git`] filter.
//! This library exists so tests can reach the modules; it is not a public API.

pub mod cli;
pub mod config;
pub mod discovery;
pub mod git;
pub mod guide;
pub mod index;
pub mod one_impl;
pub mod resolve;
