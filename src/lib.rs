//! dermestes — finds Python abstractions that have not earned their keep.
//!
//! The binary is a thin shell over [`cli`]. [`guide`] serves the agent-facing
//! pages from `docs/src/guide/`. A run goes [`config`] → [`discovery`] →
//! [`index`] (with [`calls`]) → [`one_impl`] and [`const_param`] (through
//! [`resolve`]), then, in diff mode, again on the base side [`git`] rebuilds.
//! This library exists so tests can reach the modules; it is not a public API.

pub mod calls;
pub mod cli;
pub mod config;
pub mod const_param;
pub mod discovery;
pub mod git;
pub mod guide;
pub mod index;
pub mod one_impl;
pub mod resolve;
