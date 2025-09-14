// Copyright © 2025 Stephan Kunz
//! Implementation of the entry for a [`Databoard`](crate::databoard::Databoard).

#![allow(dead_code, unused)]

use crate::{error::Result, remappings::Remappings};
use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
use core::{
	any::Any,
	marker::PhantomData,
	ops::{Deref, DerefMut},
};
use spin::{RwLock, RwLockWriteGuard};

/// The data stored in a [`Databoard`](crate::databoard::Databoard) entry.
pub struct EntryData {
	pub(crate) data: Box<dyn Any + Send + Sync>,
	pub(crate) sequence_id: usize,
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
	pub fn new<T: 'static + Send + Sync>(value: T) -> Self {
		Self {
			data: Box::new(value),
			sequence_id: usize::MIN + 1,
		}
	}
}

/// Convenience type for the Arc around the [`EntryData`]
pub type EntryPtr = Arc<RwLock<EntryData>>;

/// Write-Locked entry guard.
/// Until this value is dropped, a write lock is held on the entry.
///
/// Implements [`Deref`] & [`DerefMut`], providing access to the locked `T`.
pub struct EntryGuard<T: 'static> {
	inner: EntryGuardInner<T>,
}

impl<T: 'static> Deref for EntryGuard<T> {
	type Target = T;

	#[allow(unsafe_code)]
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.inner.ptr }
	}
}

impl<T: 'static> DerefMut for EntryGuard<T> {
	#[allow(unsafe_code)]
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.inner.modified = true;
		unsafe { &mut *self.inner.ptr }
	}
}

impl<T: 'static> EntryGuard<T> {
	pub fn new(entry: EntryPtr) -> Self {
		let inner = EntryGuardInner::new(entry);
		Self { inner }
	}
}

/// Inner data for the [`EntryGuard`].
/// Implements [`Deref`] and [`DerefMut`], providing read and write access to the `T`.
pub struct EntryGuardInner<T: 'static> {
	entry: EntryPtr,
	ptr: *mut T,
	modified: bool,
}

impl<T: 'static> Deref for EntryGuardInner<T> {
	type Target = T;

	#[allow(unsafe_code)]
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.ptr }
	}
}

impl<T: 'static> DerefMut for EntryGuardInner<T> {
	#[allow(unsafe_code)]
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.modified = true;
		unsafe { &mut *self.ptr }
	}
}

impl<T: 'static> Drop for EntryGuardInner<T> {
	fn drop(&mut self) {
		if self.modified {
			self.entry.write().sequence_id += 1;
		}
	}
}

impl<T: 'static> EntryGuardInner<T> {
	#[allow(unsafe_code)]
	#[allow(clippy::coerce_container_to_any)]
	#[allow(clippy::expect_used)]
	pub fn new(entry: EntryPtr) -> Self {
		let ptr = {
			// @TODO: holding the lock is missing!!
			let mut guard: RwLockWriteGuard<'_, EntryData> = entry.write();
			// we know this pointer is valid since the guard owns the EntryPtr
			let mut x = &mut guard.data;
			let t = x
				.downcast_mut::<T>()
				.expect("downcast should be possible");

			let ptr: *mut T = unsafe { t };
			ptr
		};

		Self {
			entry,
			ptr,
			modified: false,
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
		is_normal::<EntryPtr>();
		is_normal::<EntryData>();
	}
}
