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
use spin::RwLock;

/// A pointer to the [`EntryData`].
#[repr(transparent)]
pub struct EntryPtr(pub(crate) Box<RwLock<EntryData>>);

impl Deref for EntryPtr {
	type Target = Box<RwLock<EntryData>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for EntryPtr {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl EntryPtr {
	pub(crate) fn new<T: Send + Sync + 'static>(value: T) -> Self {
		let data = EntryData {
			data: Box::new(value),
			// start above usize::MIN, so a value of usize::MIN indicates a wrap-around
			sequence_id: usize::MIN + 1,
		};
		Self(Box::new(RwLock::new(data)))
	}
}
/// The data behind an [`EntryPtr`].
pub struct EntryData {
	pub(crate) data: Box<dyn Any + Send + Sync>,
	pub(crate) sequence_id: usize,
}

/// Locked [`Databoard`](crate::databoard::Databoard) entry. Until this value is dropped,
/// the lock is held on the [`Databoard`](crate::databoard::Databoard) entry.
///
/// Implements [`Deref`] and [`DerefMut`], providing read and write access to the locked `T`.
pub struct EntryGuard<T: 'static>(EntryGuardInner<T>);

impl<T> Deref for EntryGuard<T>
where
	T: 'static,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		todo!()
	}
}

impl<T> DerefMut for EntryGuard<T>
where
	T: 'static,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		todo!()
	}
}

struct EntryGuardInner<T>
where
	T: 'static,
{
	entry: EntryPtr,
	marker: PhantomData<T>,
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
