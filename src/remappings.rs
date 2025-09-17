// Copyright © 2025 Stephan Kunz
//! Implements [`databoard`][`Remappings`] and helper functions handling the remapping rules.

use super::error::{Error, Result};
use crate::ConstString;
use alloc::{borrow::ToOwned, string::String, vec::Vec};
use core::ops::{Deref, DerefMut};

// region:		--- helpers
/// Checks whether the given key is a constant assignment.
#[must_use]
pub fn is_const_assignment(key: &str) -> bool {
	!key.starts_with('{') && !key.ends_with('}')
}

/// Checks whether the given key is a pointer into a [`Databoard`](crate::databoard).
#[must_use]
pub fn is_board_pointer(key: &str) -> bool {
	key.starts_with('{') && key.ends_with('}')
}

/// Returns Some(literal) of the [`Databoard`](crate::databoard) pointer if it is one, otherwise `None`.
#[must_use]
pub fn strip_board_pointer(key: &str) -> Option<ConstString> {
	Some(key.strip_prefix('{')?.strip_suffix('}')?.into())
}

/// Returns the literal of the [`Databoard`](crate::databoard) pointer if it is one.
/// # Errors
/// - if is not a [`Databoard`](crate::databoard) pointer, the error contains the unchanged key.
pub fn check_board_pointer(key: &str) -> core::result::Result<ConstString, &str> {
	key.strip_prefix('{').map_or_else(
		|| Err(key),
		|v| {
			v.strip_suffix('}')
				.map_or_else(|| Err(key), |v| Ok(v.into()))
		},
	)
}

/// Returns the literal of the current/local [`Databoard`](crate::databoard) key if it is one.
/// # Errors
/// - if is not a current/local [`Databoard`](crate::databoard) `key`, the error contains the unchanged `key`.
pub fn check_local_key(key: &str) -> core::result::Result<ConstString, &str> {
	key.strip_prefix("_")
		.map_or_else(|| Err(key), |v| Ok(v.into()))
}

/// Checks whether the given key is a pointer into current/local [`Databoard`](crate::databoard).
#[must_use]
pub fn is_local_pointer(key: &str) -> bool {
	key.starts_with("{_") && key.ends_with('}')
}

/// Returns Some(literal) of the current/local [`Databoard`](crate::databoard) pointer if it is one, otherwise `None`.
/// The leading `_` is removed from the literal.
#[must_use]
pub fn strip_local_pointer(key: &str) -> Option<ConstString> {
	Some(key.strip_prefix("{_")?.strip_suffix('}')?.into())
}

/// Returns the literal of the current/local [`Databoard`](crate::databoard) pointer if it is one.
/// # Errors
/// - if is not a current/local [`Databoard`](crate::databoard) pointer, the error contains the unchanged key.
pub fn check_local_pointer(key: &str) -> core::result::Result<ConstString, &str> {
	key.strip_prefix("{_").map_or_else(
		|| Err(key),
		|v| {
			v.strip_suffix('}')
				.map_or_else(|| Err(key), |v| Ok(v.into()))
		},
	)
}

/// Returns the literal of the top level [`Databoard`](crate::databoard) key if it is one.
/// # Errors
/// - if is not a top level [`Databoard`](crate::databoard) `key`, the error contains the unchanged `key`.
pub fn check_top_level_key(key: &str) -> core::result::Result<ConstString, &str> {
	key.strip_prefix("@")
		.map_or_else(|| Err(key), |v| Ok(v.into()))
}

/// Checks whether the given key is a pointer into top level [`Databoard`](crate::databoard).
#[must_use]
pub fn is_top_level_pointer(key: &str) -> bool {
	key.starts_with("{@") && key.ends_with('}')
}

/// Returns Some(literal) of the top level [`Databoard`](crate::databoard) pointer if it is one, otherwise `None`.
/// The leading `@` is removed from the literal.
#[must_use]
pub fn strip_top_level_pointer(key: &str) -> Option<ConstString> {
	Some(key.strip_prefix("{@")?.strip_suffix('}')?.into())
}

/// Returns the literal of the top level [`Databoard`](crate::databoard) pointer if it is one.
/// # Errors
/// - if is not a top level [`Databoard`](crate::databoard) pointer, the error contains the unchanged pointer.
pub fn check_top_level_pointer(key: &str) -> core::result::Result<ConstString, &str> {
	key.strip_prefix("{@").map_or_else(
		|| Err(key),
		|v| {
			v.strip_suffix('}')
				.map_or_else(|| Err(key), |v| Ok(v.into()))
		},
	)
}
// endregion:	--- helpers

// region:		--- remappings
/// An immutable remapping entry.
type RemappingEntry = (ConstString, ConstString);

/// A mutable remapping list.
///
/// The following rules between `key`and `value` are valid:
/// - `key`and `value` are literals.
/// - The `key`s may not start with the characters `@` and `_`, these are reserved.
/// - A `value` wrapped in brackets is a `remapped_key` to a parent [`Databoard`](crate::databoard), e.g. `{remapped_key}`.
///  - A `remapped_key` starting with `@` is a redirection to the top level [`Databoard`](crate::databoard), e.g. `{@remapped_key}`.
///  - A `remapped_key` starting with `_` is a restriction to the current level [`Databoard`](crate::databoard), e.g. `{_remapped_key}`.
/// - The `value` `{=}` is a shortcut for the redirection with the same name as in `key`, e.g. `{=}`.
/// - A `value` **NOT** wrapped in brackets is a constant assignment to the key, e.g. `literal`.
///   It does not access a [`Databoard`](crate::databoard).
///   It is helpful in combination with types that implement the trait [`FromStr`](core::str::FromStr) to create a distinct value.
#[derive(Clone, Debug, Default)]
#[repr(transparent)]
pub struct Remappings(Vec<RemappingEntry>);

impl Deref for Remappings {
	type Target = Vec<RemappingEntry>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for Remappings {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl Remappings {
	/// Adds an entry to the [`Remappings`] table.
	/// # Errors
	/// - [`Error::AlreadyRemapped`] if entry already exists
	pub fn add(&mut self, key: impl Into<ConstString>, remap_to: impl Into<ConstString>) -> Result<()> {
		let key = key.into();
		for (original, remapped) in &self.0 {
			if original == &key {
				return Err(Error::AlreadyRemapped {
					key,
					remapped: remapped.to_owned(),
				});
			}
		}
		self.0.push((key, remap_to.into()));
		Ok(())
	}

	/// Adds an entry to the [`Remappings`] table.
	/// Already existing values will be overwritten.
	pub fn overwrite(&mut self, key: &str, remapped: impl Into<ConstString>) {
		for (original, old_value) in &mut self.0 {
			if original.as_ref() == key {
				// replace value
				*old_value = remapped.into();
				return;
			}
		}
		// create if not existent
		self.0.push((key.into(), remapped.into()));
	}

	/// Returns the remapped value for `key`, if there is a remapping, otherwise `None`.
	#[must_use]
	pub fn find(&self, key: &str) -> Option<ConstString> {
		for (original, remapped) in &self.0 {
			if original.as_ref() == key {
				// is the shortcut '{=}' used?
				return if remapped.as_ref() == "{=}" {
					Some((String::from("{") + key + "}").into())
				} else {
					Some(remapped.clone())
				};
			}
		}
		None
	}

	/// Returns the remapped value for `key` if there is one, otherwise the original `key`.
	#[must_use]
	pub fn remap(&self, name: &str) -> ConstString {
		for (original, remapped) in &self.0 {
			if original.as_ref() == name {
				// is the shortcut '{=}' used?
				return if remapped.as_ref() == "{=}" {
					name.into()
				} else {
					remapped.clone()
				};
			}
		}
		name.into()
	}

	/// Optimize for size
	pub fn shrink(&mut self) {
		self.0.shrink_to_fit();
	}
}
// endregion:	--- remappings

#[cfg(test)]
mod tests {
	use super::*;

	// check, that the auto traits are available
	const fn is_normal<T: Sized + Send + Sync>() {}

	#[test]
	const fn normal_types() {
		is_normal::<Remappings>();
		is_normal::<RemappingEntry>();
	}
}
