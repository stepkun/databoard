// Copyright © 2025 Stephan Kunz
//! Implementation of the entry for a [`Databoard`](crate::databoard::Databoard).

#![allow(dead_code, unused)]

use crate::{Error, error::Result, remappings::Remappings};
use alloc::{boxed::Box, collections::btree_map::BTreeMap, fmt::Debug, string::String, sync::Arc};
use core::{
	any::Any,
	marker::PhantomData,
	ops::{Deref, DerefMut},
};
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Convenience type for the Arc around the [`EntryData`]
pub type EntryPtr = Arc<RwLock<EntryData>>;

// region:		--- EntryData
/// The data stored in a [`Databoard`](crate::databoard::Databoard) entry.
pub struct EntryData {
	pub(crate) sequence_id: usize,
	pub(crate) data: Box<dyn Any + Send + Sync>,
}

impl Deref for EntryData {
	type Target = Box<dyn Any + Send + Sync>;

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

impl DerefMut for EntryData {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.data
	}
}

impl EntryData {
	/// Creates a new `EntryData`.
	pub fn new<T: Any + Send + Sync>(value: T) -> Self {
		Self {
			data: Box::new(value),
			sequence_id: usize::MIN + 1,
		}
	}

	/// Returns a reference to the stored data.
	pub fn data(&self) -> &Box<dyn Any + Send + Sync> {
		&self.data
	}

	/// Returns the current change iteration value.
	pub const fn sequence_id(&self) -> usize {
		self.sequence_id
	}
}
// endregion:	--- EntryData

// region:		--- EntryReadGuard
/// Read-Locked entry guard.
/// Until this value is dropped, a read lock is held on the entry.
///
/// Implements [`Deref`], providing read access to the locked `T`.
pub struct EntryReadGuard<T: Any + Send + Sync> {
	entry: EntryPtr,
	ptr_t: *const T,
}

impl<T: Any + Send + Sync> Deref for EntryReadGuard<T> {
	type Target = T;

	#[allow(unsafe_code)]
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.ptr_t }
	}
}

impl<T: Any + Send + Sync> Drop for EntryReadGuard<T> {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		// manually decrementing lock because entry is permanently locked in new()
		unsafe {
			self.entry.force_read_decrement();
		}
	}
}

impl<T: Any + Send + Sync> EntryReadGuard<T> {
	/// Returns a read guard to a &T.
	/// # Errors
	/// - [`Error::WrongType`] if the entry has not the expected type `T`.
	#[allow(unsafe_code)]
	pub fn new(key: &str, entry: EntryPtr) -> Result<Self> {
		// we know this pointer is valid since the guard owns the EntryPtr
		let ptr_t = {
			let mut guard = entry.read();
			// leak returns &'rwlock mut EntryData but locks RwRLock forewer
			let x = &RwLockReadGuard::leak(guard).data;
			if let Some(t) = x.downcast_ref::<T>() {
				let ptr_t: *const T = unsafe { t };
				ptr_t
			} else {
				return Err(Error::WrongType { key: key.into() });
			}
		};

		Ok(Self { entry, ptr_t })
	}

	/// Returns a read guard to a &mut T.
	/// # Errors
	/// - [`Error::IsLocked`]  if the entry is locked by someone else.
	/// - [`Error::WrongType`] if the entry has not the expected type `T`.
	#[allow(unsafe_code)]
	pub fn try_new(key: &str, entry: &EntryPtr) -> Result<Self> {
		// we know this pointer is valid since the guard owns the EntryPtr
		let ptr_t = {
			if let Some(mut guard) = entry.try_read() {
				// leak returns &'rlock EntryData but locks RwLock forewer
				let x = &RwLockReadGuard::leak(guard).data;
				if let Some(t) = x.downcast_ref::<T>() {
					let ptr_t: *const T = unsafe { t };
					ptr_t
				} else {
					return Err(Error::WrongType { key: key.into() });
				}
			} else {
				return Err(Error::IsLocked { key: key.into() });
			}
		};

		Ok(Self {
			entry: entry.clone(),
			ptr_t,
		})
	}
}
// endregion:	--- EntryReadGuard

// region:		--- EntryWriteGuard
/// Write-Locked entry guard.
/// Until this value is dropped, a write lock is held on the entry.
///
/// Implements [`Deref`] & [`DerefMut`], providing access to the locked `T`.
pub struct EntryWriteGuard<T: Any + Send + Sync> {
	entry: EntryPtr,
	ptr_t: *mut T,
	ptr_seq_id: *mut usize,
	modified: bool,
}

impl<T: Any + Send + Sync> Deref for EntryWriteGuard<T> {
	type Target = T;

	#[allow(unsafe_code)]
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.ptr_t }
	}
}

impl<T: Any + Send + Sync> DerefMut for EntryWriteGuard<T> {
	#[allow(unsafe_code)]
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.modified = true;
		unsafe { &mut *self.ptr_t }
	}
}

impl<T: Any + Send + Sync> Drop for EntryWriteGuard<T> {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		// manually removing lock because entry is permanently locked in new()
		unsafe {
			if self.modified {
				*self.ptr_seq_id += 1;
			}
			self.entry.force_write_unlock();
		}
	}
}

impl<T: Any + Send + Sync> EntryWriteGuard<T> {
	/// Returns a write guard to a &mut T.
	/// # Errors
	/// - [`Error::WrongType`] if the entry has not the expected type `T`.
	#[allow(unsafe_code)]
	pub fn new(key: &str, entry: &EntryPtr) -> Result<Self> {
		// we know this pointer is valid since the guard owns the EntryPtr
		let (ptr_t, ptr_seq_id) = {
			let mut guard = entry.write();
			let ptr_seq_id: *mut usize = unsafe { &raw mut guard.sequence_id };
			// leak returns &'rwlock mut EntryData but locks RwLock forewer
			let x = &mut RwLockWriteGuard::leak(guard).data;
			if let Some(t) = x.downcast_mut::<T>() {
				let ptr_t: *mut T = unsafe { t };
				(ptr_t, ptr_seq_id)
			} else {
				return Err(Error::WrongType { key: key.into() });
			}
		};

		Ok(Self {
			entry: entry.clone(),
			ptr_t,
			ptr_seq_id,
			modified: false,
		})
	}

	/// Returns a write guard to a &mut T.
	/// # Errors
	/// - [`Error::IsLocked`]  if the entry is locked by someone else.
	/// - [`Error::WrongType`] if the entry has not the expected type `T`.
	#[allow(unsafe_code)]
	pub fn try_new(key: &str, entry: &EntryPtr) -> Result<Self> {
		// we know this pointer is valid since the guard owns the EntryPtr
		let (ptr_t, ptr_seq_id) = {
			if let Some(mut guard) = entry.try_write() {
				let ptr_seq_id: *mut usize = unsafe { &raw mut guard.sequence_id };
				// leak returns &'rwlock mut EntryData but locks RwLock forewer
				let x = &mut RwLockWriteGuard::leak(guard).data;
				if let Some(t) = x.downcast_mut::<T>() {
					let ptr_t: *mut T = unsafe { t };
					(ptr_t, ptr_seq_id)
				} else {
					return Err(Error::WrongType { key: key.into() });
				}
			} else {
				return Err(Error::IsLocked { key: key.into() });
			}
		};

		Ok(Self {
			entry: entry.clone(),
			ptr_t,
			ptr_seq_id,
			modified: false,
		})
	}
}
// endregion:	--- EntryWriteGuard

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone, Debug)]
	struct Dummy {
		data: i32,
	}

	// check, that the auto traits are available
	const fn is_normal<T: Sized + Send + Sync>() {}

	#[test]
	const fn normal_types() {
		is_normal::<Dummy>();
		is_normal::<EntryData>();
		is_normal::<EntryPtr>();
		// is_normal::<EntryReadGuard<Dummy>>();
		// is_normal::<EntryWriteGuard<Dummy>>();
	}
}
