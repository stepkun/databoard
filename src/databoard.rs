// Copyright © 2025 Stephan Kunz
//! Implements the [`Databoard`].

#![allow(dead_code, unused)]

#[cfg(feature = "std")]
extern crate std;

use crate::{
	ConstString, Error, check_board_pointer, check_local_pointer, check_top_level_key, check_top_level_pointer,
	database::Database,
	entry::{EntryData, EntryPtr, EntryReadGuard, EntryWriteGuard},
	error::Result,
	is_const_assignment,
	remappings::{Remappings, check_local_key},
};
use alloc::{borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use core::{
	any::{Any, TypeId},
	fmt::Debug,
	ops::{Deref, DerefMut},
};
use spin::RwLock;

/// Convenience type for a thread safe pointer to a [`Databoard`].
pub type DataboardPtr = Arc<Databoard>;

/// Implements a hierarchical databoard.
#[derive(Default)]
pub struct Databoard {
	/// [`Database`] of this `Databoard`.
	/// It is behind an `RwLock` to protect against data races.
	database: RwLock<Database>,
	/// An optional reference to a parent `Databoard`.
	parent: Option<DataboardPtr>,
	/// Manual remapping rules from this `Databoard` to the parent.
	remappings: Remappings,
	/// Whether to use automatic remapping to parents content.
	autoremap: bool,
}

impl Databoard {
	/// Creates a [`DataboardPtr`] to a new [`Databoard`].
	#[must_use]
	pub fn new() -> DataboardPtr {
		Arc::new(Self {
			database: RwLock::new(Database::default()),
			parent: None,
			remappings: Remappings::default(),
			autoremap: false,
		})
	}

	/// Creates a [`DataboardPtr`] to a new [`Databoard`] with given parameters.
	/// # Panics
	/// - if called with remappings but without a parent
	pub fn with(parent: Option<DataboardPtr>, remappings: Option<Remappings>, autoremap: bool) -> DataboardPtr {
		assert!(
			!(remappings.is_some() && parent.is_none()),
			"invalid usage of Databoard::with(...) giving some remappings but no parent"
		);
		let remappings = remappings.map_or_else(Remappings::default, |remappings| remappings);
		let database = RwLock::new(Database::default());
		Arc::new(Self {
			database,
			parent,
			remappings,
			autoremap,
		})
	}

	/// Creates a [`DataboardPtr`] to a new [`Databoard`] using the given parent.
	/// The parents entries are automatically remapped into the new databoard.
	#[must_use]
	pub fn with_parent(parent: DataboardPtr) -> DataboardPtr {
		let database = RwLock::new(Database::default());
		Arc::new(Self {
			database,
			parent: Some(parent),
			remappings: Remappings::default(),
			autoremap: true,
		})
	}

	/// Returns `true` if a certain `key` is available, otherwise `false`.
	#[must_use]
	pub fn contains_key(&self, key: &str) -> bool {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().contains_key(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().contains_key(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.contains_key(board_pointer)
								} else {
									false
								}
							}
							Err(_) => false,
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.contains_key(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						self.database.read().contains_key(original_key)
					}
				}
			},
		}
	}

	/// Returns  a result of `true` if a certain `key` is available, otherwise a result of `false`.
	/// # Errors
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn contains<T: Any + Send + Sync>(&self, key: &str) -> Result<bool> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().contains::<T>(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().contains::<T>(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.contains::<T>(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Ok(false),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.contains::<T>(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						self.database.read().contains::<T>(original_key)
					}
				}
			},
		}
	}

	/// Prints the content of the [`Databoard`] for debugging purpose.
	#[cfg(feature = "std")]
	pub fn debug_message(&self) {
		std::println!("not yet implemented");
	}

	/// Returnsthe value of type `T` stored under `key` and deletes it from database.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn delete<T: Any + Send + Sync>(&self, key: &str) -> Result<T> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().delete(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.write().delete(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.delete(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.delete(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						self.database.write().delete(original_key)
					}
				}
			},
		}
	}

	/// Returns a clone of the [`EntryPtr`] stored under `key`.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	pub fn entry(&self, key: &str) -> Result<EntryPtr> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().entry(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().entry(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.entry(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.entry(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						self.database.read().entry(original_key)
					}
				}
			},
		}
	}

	/// Returns a copy of the value of type `T` stored under `key`.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn get<T: Any + Clone + Send + Sync>(&self, key: &str) -> Result<T> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().get(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().read(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.get(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.get(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						self.database.read().read(original_key)
					}
				}
			},
		}
	}

	/// Returns a read/write guard to the `T` of the `entry` stored under `key`.
	/// The entry is locked for read & write while this reference is held.
	/// Multiple changes during holding the reference are counted as a single change,
	/// so `sequence_id()`will only increase by 1.
	///
	/// You need to drop the received [`EntryGuardWrite`] before using `delete`, `get`, `set` or `sequence_id`.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn get_mut_ref<T: Any + Send + Sync>(&self, key: &str) -> Result<EntryWriteGuard<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().get_mut_ref(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().get_mut_ref(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.get_mut_ref(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.get_mut_ref(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						if self.database.read().contains_key(original_key) {
							self.database.read().get_mut_ref(original_key)
						} else {
							Err(Error::NotFound {
								key: original_key.into(),
							})
						}
					}
				}
			},
		}
	}

	/// Returns a read guard to the `T` of the `entry` stored under `key`.
	/// The entry is locked for write while this reference is held.
	///
	/// You need to drop the received [`EntryGuardRead`] before using `delete` or `set`.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	pub fn get_ref<T: Any + Send + Sync>(&self, key: &str) -> Result<EntryReadGuard<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().get_ref(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().get_ref(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.get_ref(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.get_ref(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						if self.database.read().contains_key(original_key) {
							self.database.read().get_ref(original_key)
						} else {
							Err(Error::NotFound {
								key: original_key.into(),
							})
						}
					}
				}
			},
		}
	}

	/// Returns a reference to the remappings, if there are any, otherwise `None`.
	pub fn remappings(&self) -> Option<&Remappings> {
		if self.remappings.is_empty() {
			None
		} else {
			Some(&self.remappings)
		}
	}

	/// Returns a reference to the root [`Databoard`] of the hierarchy.
	fn root(&self) -> &Self {
		self.parent
			.as_ref()
			.map_or(self, |board| board.root())
	}

	/// Read needed remapping information to parent.
	fn remapping_info(&self, key: &str) -> (ConstString, bool) {
		let (remapped_key, has_remapping) = self
			.remappings
			.find(key)
			.map_or_else(|| (key.into(), false), |remapped| (remapped, true));

		(remapped_key, has_remapping)
	}

	/// Returns the sequence id of an entry.
	/// The sequence id starts with '1' and is increased at every change of an entry.
	/// The sequence wraps around to '1' after reaching [`usize::MAX`] .
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	pub fn sequence_id(&self, key: &str) -> Result<usize> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().sequence_id(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().sequence_id(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.sequence_id(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.sequence_id(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						self.database.read().sequence_id(original_key)
					}
				}
			},
		}
	}

	/// Stores a value of type `T` under `key` and returns an eventually existing value of type `T`.
	/// # Errors
	/// - [`Error::WrongType`] if `key` already exists with a different type
	pub fn set<T: Any + Send + Sync>(&self, key: &str, value: T) -> Result<Option<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().set(stripped_key, value),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => {
					let old = self.database.read().update(local_key, value)?;
					Ok(Some(old))
				}
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.set(board_pointer, value)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.set(&parent_key, value)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						if self.contains_key(original_key) {
							let old = self.database.read().update(original_key, value)?;
							Ok(Some(old))
						} else {
							self.database
								.write()
								.create(original_key, value)?;
							Ok(None)
						}
					}
				}
			},
		}
	}

	/// Returns a read/write guard to the `T` of the `entry` stored under `key`.
	/// The entry is locked for read & write while this reference is held.
	/// Multiple changes during holding the reference are counted as a single change,
	/// so `sequence_id()`will only increase by 1.
	///
	/// You need to drop the received [`EntryGuardWrite`] before using `delete`, `get`, `set` or `sequence_id`.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	/// - [`Error::IsLocked`] if the entry is locked by someone else
	pub fn try_get_mut_ref<T: Any + Send + Sync>(&self, key: &str) -> Result<EntryWriteGuard<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().try_get_mut_ref(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().try_get_mut_ref(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.try_get_mut_ref(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.try_get_mut_ref(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						if self.database.read().contains_key(original_key) {
							self.database.read().try_get_mut_ref(original_key)
						} else {
							Err(Error::NotFound {
								key: original_key.into(),
							})
						}
					}
				}
			},
		}
	}

	/// Returns a read guard to the `T` of the `entry` stored under `key`.
	/// The entry is locked for write while this reference is held.
	///
	/// You need to drop the received [`EntryGuardRead`] before using `delete` or `set`.
	/// # Errors
	/// - [`Error::NotFound`] if `key` is not contained
	/// - [`Error::WrongType`] if the entry has not the expected type `T`
	/// - [`Error::IsLocked`] if the entry is locked by someone else
	pub fn try_get_ref<T: Any + Send + Sync>(&self, key: &str) -> Result<EntryReadGuard<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().try_get_ref(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().try_get_ref(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						#[allow(clippy::option_if_let_else)]
						match check_board_pointer(&parent_key) {
							Ok(board_pointer) => {
								if let Some(parent) = &self.parent {
									parent.try_get_ref(board_pointer)
								} else {
									Err(Error::Unexpected(file!().into(), line!()))
								}
							}
							Err(_) => Err(Error::Assignment {
								key: original_key.into(),
								value: parent_key,
							}),
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.get_ref(&parent_key)
					} else {
						// If it is not remapped anywhere in hierarchy, handle it in current `Blackboard`
						if self.database.read().contains_key(original_key) {
							self.database.read().try_get_ref(original_key)
						} else {
							Err(Error::NotFound {
								key: original_key.into(),
							})
						}
					}
				}
			},
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
		is_normal::<Databoard>();
		is_normal::<DataboardPtr>();
	}
}
