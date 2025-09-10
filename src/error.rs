// Copyright © 2025 Stephan Kunz
//! The [`Databoard`](crate::Databoard) error handling.

use crate::ConstString;

/// Shortcut for [`Databoard`](crate::Databoard)'s Result<T, E> type
pub type Result<T> = core::result::Result<T, Error>;

/// Things that may go wrong using the [`Databoard`].
#[non_exhaustive]
pub enum Error {
	/// Key is already remapped.
	AlreadyRemapped {
		/// Key to be remapped.
		key: ConstString,
		/// The already existing remapping.
		remapped: ConstString,
	},
	/// Entry with `key` not stored.
	NotFound {
		/// Key of the wanted entry.
		key: ConstString,
	},
	/// Entry with `key` is stored with a different type.
	WrongType {
		/// Key of the wanted entry.
		key: ConstString,
	},

	/// Something impossible happened.
	Unexpected(ConstString, u32),
}

/// Currently the default implementation is sufficient.
impl core::error::Error for Error {
	// fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
	// 	None
	// }

	// fn cause(&self) -> Option<&dyn core::error::Error> {
	// 	self.source()
	// }

	// fn provide<'a>(&'a self, request: &mut core::error::Request<'a>) {}
}

impl core::fmt::Debug for Error {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::AlreadyRemapped { key, remapped } => {
				write!(f, "AlreadyRemapped(key: {key}, remapped: {remapped}")
			}
			Self::NotFound { key } => write!(f, "NotFound(key: {key}"),
			Self::WrongType { key } => write!(f, "WrongType(key: {key}"),
			Self::Unexpected(file, line) => write!(f, "Unexpected(file: {file}, line: {line}"),
		}
	}
}

impl core::fmt::Display for Error {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::AlreadyRemapped { key, remapped } => {
				write!(f, "key {key} is already remapped as {remapped}")
			}
			Self::NotFound { key } => write!(f, "an entry for the key {key} is not existing"),
			Self::WrongType { key } => write!(f, "the entry for the key {key} is stored with a different type"),
			Self::Unexpected(file, line) => write!(f, "an unexpected error occured in {file} at line {line}"),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// check, that the auto traits are available
	const fn is_normal<T: Sized + Send + Sync>() {}

	#[test]
	const fn normal_types() {
		is_normal::<Error>();
	}
}
