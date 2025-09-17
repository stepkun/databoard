// Copyright © 2025 Stephan Kunz
#![no_std]
#![doc = include_str!("../README.md")]

#[cfg(doctest)]
doc_comment::doctest!("../README.md");

#[doc(hidden)]
extern crate alloc;

mod database;
mod databoard;
mod entry;
mod error;
mod remappings;

// flatten
pub use databoard::{Databoard, DataboardPtr};
pub use error::Error;
pub use remappings::{
	Remappings, check_board_pointer, check_local_key, check_local_pointer, check_top_level_key, check_top_level_pointer,
	is_board_pointer, is_const_assignment, is_local_pointer, is_top_level_pointer, strip_board_pointer, strip_local_pointer,
	strip_top_level_pointer,
};

/// An immutable thread safe `String` type
/// see: [Logan Smith](https://www.youtube.com/watch?v=A4cKi7PTJSs).
type ConstString = alloc::sync::Arc<str>;
