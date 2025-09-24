// Copyright © 2025 Stephan Kunz
//! Implements the [`Databoard`].

#[cfg(feature = "std")]
extern crate std;

use crate::{
	ConstString, Error, check_board_pointer, check_top_level_key,
	database::Database,
	entry::{EntryPtr, EntryReadGuard, EntryWriteGuard},
	error::Result,
	remappings::{Remappings, check_local_key},
	strip_board_pointer,
};
use alloc::sync::Arc;
use core::{any::Any, ops::Deref};
use spin::RwLock;

/// A thread safe data board.
pub struct Databoard(Arc<DataboardInner>);

impl Clone for Databoard {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl core::fmt::Debug for Databoard {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Databoard {{ ")?;
		write!(f, "autoremap: {:?}", &self.0.autoremap)?;
		write!(f, ", {:?}", &*self.0.database.read())?;
		write!(f, ", {:?}", &self.0.remappings)?;
		write!(f, ", parent: ")?;
		if let Some(parent) = &self.0.parent {
			write!(f, "{parent:?}",)
		} else {
			write!(f, "None")
		}?;
		write!(f, " }}")
	}
}

impl Deref for Databoard {
	type Target = DataboardInner;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Default for Databoard {
	fn default() -> Self {
		Self(Arc::new(DataboardInner {
			database: RwLock::new(Database::default()),
			parent: None,
			remappings: Remappings::default(),
			autoremap: false,
		}))
	}
}

impl Databoard {
	/// Creates a [`Databoard`].
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	/// Creates a [`Databoard`] with given parameters.
	pub fn with(parent: Option<Self>, remappings: Option<Remappings>, autoremap: bool) -> Self {
		let remappings = remappings.map_or_else(Remappings::default, |remappings| remappings);
		let database = RwLock::new(Database::default());
		Self(Arc::new(DataboardInner {
			database,
			parent,
			remappings,
			autoremap,
		}))
	}

	/// Creates a [`Databoard`] using the given parent.
	/// The parents entries are automatically remapped into the new databoard.
	#[must_use]
	pub fn with_parent(parent: Self) -> Self {
		let database = RwLock::new(Database::default());
		Self(Arc::new(DataboardInner {
			database,
			parent: Some(parent),
			remappings: Remappings::default(),
			autoremap: true,
		}))
	}
}

/// Implements a hierarchical databoard.
#[derive(Default)]
pub struct DataboardInner {
	/// database of this `Databoard`.
	/// It is behind an `RwLock` to protect against data races.
	database: RwLock<Database>,
	/// An optional reference to a parent `Databoard`.
	parent: Option<Databoard>,
	/// Manual remapping rules from this `Databoard` to the parent.
	remappings: Remappings,
	/// Whether to use automatic remapping to parents content.
	autoremap: bool,
}

impl DataboardInner {
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
						if let Some(board_pointer) = strip_board_pointer(&parent_key)
							&& let Some(parent) = &self.parent
						{
							parent.contains_key(board_pointer)
						} else {
							false
						}
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.contains_key(&parent_key)
					} else {
						// No remapping, use local database
						self.database.read().contains_key(original_key)
					}
				}
			},
		}
	}

	/// Returns a result of `true` if a certain `key` is available, otherwise a result of `false`.
	/// # Errors
	/// - [`Error::NoParent`]  if `key` is remapped to a parent without having a parent.
	/// - [`Error::WrongType`] if the entry has not the expected type `T`.
	pub fn contains<T: Any + Send + Sync>(&self, key: &str) -> Result<bool> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().contains::<T>(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().contains::<T>(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						check_board_pointer(&parent_key).map_or(Ok(false), |board_pointer| {
							self.parent.as_ref().map_or_else(
								|| {
									Err(Error::NoParent {
										key: key.into(),
										remapped: board_pointer.into(),
									})
								},
								|parent| parent.contains::<T>(board_pointer),
							)
						})
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.contains::<T>(&parent_key)
					} else {
						// No remapping, use local database
						self.database.read().contains::<T>(original_key)
					}
				}
			},
		}
	}

	/// Prints the content of the [`Databoard`] for debugging purpose.
	#[cfg(feature = "std")]
	pub fn debug_message(&self) {
		let _ = self.parent;
		std::println!("not yet implemented");
	}

	/// Returns the value of type `T` stored under `key` and deletes it from database.
	/// # Errors
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::NotFound`]   if `key` is not contained.
	/// - [`Error::WrongType`]  if the entry has not the expected type `T`.
	pub fn delete<T: Any + Send + Sync>(&self, key: &str) -> Result<T> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().delete(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.write().delete(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.delete(board_pointer),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.delete(&parent_key)
					} else {
						// No remapping, use local database
						self.database.write().delete(original_key)
					}
				}
			},
		}
	}

	/// Returns a clone of the [`EntryPtr`] stored under `key`.
	/// # Errors
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::NotFound`]   if `key` is not contained.
	pub fn entry(&self, key: &str) -> Result<EntryPtr> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().entry(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().entry(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.entry(board_pointer),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.entry(&parent_key)
					} else {
						// No remapping, use local database
						self.database.read().entry(original_key)
					}
				}
			},
		}
	}

	/// Returns a copy of the value of type `T` stored under `key`.
	/// # Errors
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::NotFound`]   if `key` is not contained.
	/// - [`Error::WrongType`]  if the entry has not the expected type `T`.
	pub fn get<T: Any + Clone + Send + Sync>(&self, key: &str) -> Result<T> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().get(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().read(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.get(board_pointer),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.get(&parent_key)
					} else {
						// No remapping, use local database
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
	/// You need to drop the received [`EntryWriteGuard`] before using `delete`, `get`, `set` or `sequence_id`.
	/// # Errors
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::NotFound`]   if `key` is not contained.
	/// - [`Error::WrongType`]  if the entry has not the expected type `T`.
	pub fn get_mut_ref<T: Any + Send + Sync>(&self, key: &str) -> Result<EntryWriteGuard<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().get_mut_ref(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().get_mut_ref(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.get_mut_ref(board_pointer),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.get_mut_ref(&parent_key)
					} else {
						// No remapping, use local database
						self.database.read().get_mut_ref(original_key)
					}
				}
			},
		}
	}

	/// Returns a read guard to the `T` of the `entry` stored under `key`.
	/// The entry is locked for write while this reference is held.
	///
	/// You need to drop the received [`EntryReadGuard`] before using `delete` or `set`.
	/// # Errors
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::NotFound`]   if `key` is not contained.
	/// - [`Error::WrongType`]  if the entry has not the expected type `T`.
	pub fn get_ref<T: Any + Send + Sync>(&self, key: &str) -> Result<EntryReadGuard<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().get_ref(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().get_ref(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.get_ref(board_pointer),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.get_ref(&parent_key)
					} else {
						// No remapping, use local database
						self.database.read().get_ref(original_key)
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
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::NotFound`]   if `key` is not contained.
	pub fn sequence_id(&self, key: &str) -> Result<usize> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().sequence_id(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().sequence_id(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.sequence_id(board_pointer),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.sequence_id(&parent_key)
					} else {
						// No remapping, use local database
						self.database.read().sequence_id(original_key)
					}
				}
			},
		}
	}

	/// Stores the value of type `T` under `key` and returns an eventually existing value of type `T`.
	/// # Errors
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::WrongType`]  if `key` already exists with a different type.
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
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.set(board_pointer, value),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.set(&parent_key, value)
					} else {
						// No remapping, use local database
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
	/// You need to drop the received [`EntryWriteGuard`] before using `delete`, `get...`, `set` or `sequence_id`.
	/// # Errors
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::IsLocked`]   if the entry is locked by someone else.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::NotFound`]   if `key` is not contained.
	/// - [`Error::WrongType`]  if the entry has not the expected type `T`.
	pub fn try_get_mut_ref<T: Any + Send + Sync>(&self, key: &str) -> Result<EntryWriteGuard<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().try_get_mut_ref(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().try_get_mut_ref(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.try_get_mut_ref(board_pointer),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.try_get_mut_ref(&parent_key)
					} else {
						// No remapping, use local database
						self.database.read().try_get_mut_ref(original_key)
					}
				}
			},
		}
	}

	/// Returns a read guard to the `T` of the `entry` stored under `key`.
	/// The entry is locked for write while this reference is held.
	///
	/// You need to drop the received [`EntryReadGuard`] before using `delete` or `set`.
	/// # Errors
	/// - [`Error::Assignment`] if the remapping contains an assignment of a `str` value.
	/// - [`Error::IsLocked`]   if the entry is locked by someone else.
	/// - [`Error::NoParent`]   if `key` is remapped to a parent without having a parent.
	/// - [`Error::NotFound`]   if `key` is not contained.
	/// - [`Error::WrongType`]  if the entry has not the expected type `T`.
	pub fn try_get_ref<T: Any + Send + Sync>(&self, key: &str) -> Result<EntryReadGuard<T>> {
		match check_top_level_key(key) {
			Ok(stripped_key) => self.root().try_get_ref(stripped_key),
			Err(original_key) => match check_local_key(original_key) {
				Ok(local_key) => self.database.read().try_get_ref(local_key),
				Err(original_key) => {
					let (parent_key, has_remapping) = self.remapping_info(original_key);
					if has_remapping {
						strip_board_pointer(&parent_key).map_or_else(
							|| {
								Err(Error::Assignment {
									key: original_key.into(),
									value: parent_key.clone(),
								})
							},
							|board_pointer| {
								self.parent.as_ref().map_or_else(
									|| {
										Err(Error::NoParent {
											key: key.into(),
											remapped: board_pointer.into(),
										})
									},
									|parent| parent.try_get_ref(board_pointer),
								)
							},
						)
					} else if self.autoremap
						&& let Some(parent) = &self.parent
					{
						parent.get_ref(&parent_key)
					} else {
						// No remapping, use local database
						self.database.read().try_get_ref(original_key)
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
		is_normal::<DataboardInner>();
		is_normal::<Databoard>();
	}
}
