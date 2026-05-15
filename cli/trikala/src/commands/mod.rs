//! CLI command modules. One file per verb keeps `--help` text close to
//! the code that implements it.
//!
//! Per axiom U6 the help output of each command reminds the user which
//! of the three phases (อดีต / ปัจจุบัน / อนาคต) the command lives in.

pub mod new;
pub mod dev;
pub mod build;
pub mod deploy;
pub mod claim;
pub mod doctor;
pub mod r#use;
pub mod upgrade;

// Re-export with a name `use_` so the parent module can write
// `commands::use_::Args` (the `use` keyword can't appear as an
// identifier without raw escapes).
pub use r#use as use_;
