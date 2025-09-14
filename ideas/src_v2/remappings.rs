// Copyright © 2025 Stephan Kunz
//! [`Databoard`][`Remappings`] implementation.

use super::error::{Error, Result};
use crate::ConstString;
use alloc::{borrow::ToOwned, string::String, vec::Vec};
use core::ops::{Deref, DerefMut};

/// An immutable remapping entry.
type RemappingEntry = (ConstString, ConstString);

/// A mutable remapping list.
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
	/// - if entry already exists
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
	pub fn overwrite(&mut self, name: &str, remapped_name: impl Into<ConstString>) {
		for (original, old_value) in &mut self.0 {
			if original.as_ref() == name {
				// replace value
				*old_value = remapped_name.into();
				return;
			}
		}
		// create if not existent
		self.0.push((name.into(), remapped_name.into()));
	}

	/// Lookup the remapped name.
	#[must_use]
	pub fn find(&self, name: &str) -> Option<ConstString> {
		for (original, remapped) in &self.0 {
			if original.as_ref() == name {
				// is the shortcut '{=}' used?
				return if remapped.as_ref() == "{=}" {
					Some((String::from("{") + name + "}").into())
				} else {
					Some(remapped.clone())
				};
			}
		}
		None
	}

	/// Optimize for size
	pub fn shrink(&mut self) {
		self.0.shrink_to_fit();
	}
}

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
