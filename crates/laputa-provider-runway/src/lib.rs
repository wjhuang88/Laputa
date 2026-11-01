//! Runway adapter and project-owned output staging for Laputa.
//!
//! Vendor HTTP concerns remain isolated in this crate. The optional project wrapper
//! converts Runway's temporary HTTPS outputs into durable project-owned local files
//! before the existing Laputa generation publication gate sees them.

mod core;
mod project_output;

pub use core::*;
pub use project_output::*;
