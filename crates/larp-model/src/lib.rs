#![doc = include_str!("../README.md")]

mod diagnostic;
mod id;
mod load;
pub mod model;
mod playthrough;
pub mod text;
pub mod time;
mod validate;

pub use diagnostic::{Diagnostic, Severity};
pub use id::{Id, IdError};
pub use load::{Checked, SourceFile, load};
pub use model::Game;
pub use playthrough::{MAX_STATES, Playthroughs};
