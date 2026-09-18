//! dermestes — finds Python abstractions that have not earned their keep.
//!
//! The binary is a thin shell over [`cli`]. [`guide`] serves the agent-facing
//! pages from `docs/src/guide/`. The checks themselves arrive phase by phase;
//! see `docs/plans/CONSTITUTION.md` for what they are and the rules they obey.

pub mod cli;
pub mod guide;
