// Copyright © 2025 Stephan Kunz
#![no_std]
#![doc = include_str!("../README.md")]

#[cfg(doctest)]
doc_comment::doctest!("../README.md");

#[doc(hidden)]
extern crate alloc;

mod databoard;
mod databoard_data;
mod entry;
mod error;
mod remappings;

use alloc::sync::Arc;

// flatten
pub use databoard::Databoard;
pub use error::{Error, Result};
pub use remappings::Remappings;

/// An immutable thread safe `String` type
/// see: [Logan Smith](https://www.youtube.com/watch?v=A4cKi7PTJSs).
type ConstString = Arc<str>;

#[cfg(test)]
mod tests {}
