#![feature(prelude_import)]
#![no_std]
/*!# databoard
under construction

## License

Licensed with the fair use "NGMC" license, see [license file](https://github.com/stepkun/behaviortree/blob/main/LICENSE)

## Contribution

Any contribution intentionally submitted for inclusion in the work by you,
shall be licensed with the same "NGMC" license, without any additional terms or conditions.
*/
#[prelude_import]
use core::prelude::rust_2024::*;
#[macro_use]
extern crate core;
#[doc(hidden)]
extern crate alloc;
mod database {
    //! Implementation of the [`DataboardData`].
    #![allow(dead_code, unused)]
    use crate::{
        ConstString, Error, entry::{EntryData, EntryGuard, EntryPtr},
        error::Result, remappings::Remappings,
    };
    use alloc::{
        borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String,
        sync::Arc,
    };
    use core::{any::Any, ops::{Deref, DerefMut}};
    use spin::RwLock;
    /// Holds all [`Databoard`](crate::databoard::Databoard) data.
    pub struct Database {
        storage: BTreeMap<ConstString, EntryPtr>,
    }
    #[automatically_derived]
    impl ::core::default::Default for Database {
        #[inline]
        fn default() -> Database {
            Database {
                storage: ::core::default::Default::default(),
            }
        }
    }
    impl Database {
        /// Returns `true` if a certain `key` is available, otherwise `false`.
        pub fn contains_key(&self, key: &str) -> bool {
            self.storage.contains_key(key)
        }
        /// Returns  a result of `true` if a certain `key` is available, otherwise a result of `false`.
        /// # Errors
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn contains<T: 'static>(&self, key: &str) -> Result<bool> {
            if let Some(entry) = self.storage.get(key) {
                let en = &*entry.read().data;
                if en.downcast_ref::<T>().is_none() {
                    return Err(Error::WrongType {
                        key: key.into(),
                    });
                }
                return Ok(true);
            }
            Ok(false)
        }
        /// Creates a value of type `T` under `key`.
        /// # Errors
        /// - [`Error::AlreadyExists`] if `key` already exists
        pub fn create<T: Send + Sync + 'static>(
            &mut self,
            key: impl Into<ConstString>,
            value: T,
        ) -> Result<()> {
            let key = key.into();
            if self.storage.contains_key(&key) {
                return Err(Error::AlreadyExists { key });
            }
            let entry = Arc::new(RwLock::new(EntryData::new(value)));
            if self.storage.insert(key, entry).is_some() {
                return Err(Error::Unexpected("src/database.rs".into(), 56u32));
            }
            Ok(())
        }
        /// Returns a value of type `T` stored under `key` and deletes it from storage.
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn delete<T: Clone + Send + Sync + 'static>(
            &mut self,
            key: &str,
        ) -> Result<T> {
            if let Some(entry) = self.storage.get(key) {
                let en = &*entry.read().data;
                if en.downcast_ref::<T>().is_none() {
                    return Err(Error::WrongType {
                        key: key.into(),
                    });
                }
            } else {
                return Err(Error::NotFound { key: key.into() });
            }
            if let Some(old) = self.storage.remove(key) {
                let en = &*old.read().data;
                let t = en.downcast_ref::<T>();
                return t
                    .cloned()
                    .map_or_else(
                        || Err(Error::WrongType {
                            key: key.into(),
                        }),
                        |v| Ok(v),
                    );
            }
            Err(Error::Unexpected("src/database.rs".into(), 84u32))
        }
        /// Returns a read/write guard to the `T` for the `key`.
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn get_ref<T: 'static>(&self, key: &str) -> Result<EntryGuard<T>> {
            if let Some(entry) = self.storage.get(key) {
                if let Some(guard) = EntryGuard::create(entry.clone()) {
                    return Ok(guard);
                } else {
                    return Err(Error::WrongType {
                        key: key.into(),
                    });
                }
            }
            Err(Error::NotFound { key: key.into() })
        }
        /// Returns a copy of the value of type `T` stored under `key`.
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn read<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<T> {
            self.storage
                .get(key)
                .map_or_else(
                    || Err(Error::NotFound { key: key.into() }),
                    |entry| {
                        let en = &*entry.read().data;
                        let t = en.downcast_ref::<T>();
                        t.cloned()
                            .map_or_else(
                                || Err(Error::WrongType {
                                    key: key.into(),
                                }),
                                |v| Ok(v),
                            )
                    },
                )
        }
        /// Returns the sequence id of an entry.
        /// The sequence id starts with '1' and is increased at every change of an entry.
        /// The sequence wraps around to '1' after reaching [`usize::MAX`] .
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        pub fn sequence_id(&self, key: &str) -> Result<usize> {
            self.storage
                .get(key)
                .map_or_else(
                    || Err(Error::NotFound { key: key.into() }),
                    |entry| Ok(entry.read().sequence_id),
                )
        }
        /// Updates a value of type `T` stored under `key` and returns the old value.
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn update<T: Clone + Send + Sync + 'static>(
            &self,
            key: &str,
            value: T,
        ) -> Result<T> {
            self.storage
                .get(key)
                .map_or_else(
                    || Err(Error::NotFound { key: key.into() }),
                    |mut entry| {
                        let en = &mut *entry.write();
                        let t = en.data.downcast_ref::<T>();
                        t.cloned()
                            .map_or_else(
                                || Err(Error::WrongType {
                                    key: key.into(),
                                }),
                                |v| {
                                    en.data = Box::new(value);
                                    if en.sequence_id < usize::MAX {
                                        en.sequence_id += 1;
                                    } else {
                                        en.sequence_id = usize::MIN + 1;
                                    }
                                    Ok(v)
                                },
                            )
                    },
                )
        }
    }
}
mod databoard {
    //! Implementation of the [`Databoard`].
    #![allow(dead_code, unused)]
    use crate::{
        ConstString, Error, database::Database,
        entry::{EntryData, EntryGuard, EntryPtr, EntryRef},
        error::Result, remappings::Remappings,
    };
    use alloc::{
        borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String,
        sync::Arc,
    };
    use core::{any::Any, ops::{Deref, DerefMut}};
    use spin::RwLock;
    /// Convenience type for a thread safe pointer to a [`Databoard`].
    pub type DataboardPtr = Arc<Databoard>;
    /// The Databoard implementation.
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
        pub fn with(
            parent: Option<DataboardPtr>,
            remappings: Option<Remappings>,
            autoremap: bool,
        ) -> DataboardPtr {
            if !!(remappings.is_some() && parent.is_none()) {
                {
                    ::core::panicking::panic_fmt(
                        format_args!(
                            "invalid usage of Databoard::with(...) giving some remappings but no parent",
                        ),
                    );
                }
            }
            let remappings = remappings
                .map_or_else(Remappings::default, |remappings| remappings);
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
            if let Some(key_stripped) = key.strip_prefix('@') {
                return self.root().contains_key(key_stripped);
            }
            if self.database.read().contains_key(key) {
                return true;
            }
            let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
            if let Some(parent) = &self.parent
                && (has_remapping || (autoremap && parent.contains_key(&parent_key)))
            {
                return parent.contains_key(&parent_key);
            }
            false
        }
        /// Returns  a result of `true` if a certain `key` is available, otherwise a result of `false`.
        /// # Errors
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn contains<T: 'static>(&self, key: &str) -> Result<bool> {
            if let Some(key_stripped) = key.strip_prefix('@') {
                return self.root().contains::<T>(key_stripped);
            }
            if self.database.read().contains::<T>(key)? {
                return Ok(true);
            }
            let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
            if let Some(parent) = &self.parent
                && (has_remapping || (autoremap && parent.contains::<T>(&parent_key)?))
            {
                return Ok(true);
            }
            Ok(false)
        }
        /// Returns a value of type `T` stored under `key` and deletes it from database.
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn delete<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<T> {
            if let Some(key_stripped) = key.strip_prefix('@') {
                return self.root().delete(key_stripped);
            }
            if self.database.read().contains_key(key) {
                return self.database.write().delete(key);
            }
            let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
            if let Some(parent) = &self.parent
                && (has_remapping || (autoremap && parent.contains_key(&parent_key)))
            {
                return parent.delete(&parent_key);
            }
            Err(Error::NotFound { key: key.into() })
        }
        /// Returns a copy of the raw [`EntryData`] stored under `key`.
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        fn entry(&self, key: &str) -> Result<EntryData> {
            if let Some(key_stripped) = key.strip_prefix('@') {
                return self.root().entry(key_stripped);
            }
            if self.database.read().contains_key(key) {
                ::core::panicking::panic("not yet implemented");
            }
            let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
            if let Some(parent) = &self.parent
                && (has_remapping || (autoremap && parent.contains_key(&parent_key)))
            {
                return parent.entry(&parent_key);
            }
            Err(Error::NotFound { key: key.into() })
        }
        /// Returns a copy of the value of type `T` stored under `key`.
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<T> {
            if let Some(key_stripped) = key.strip_prefix('@') {
                return self.root().get(key_stripped);
            }
            if self.database.read().contains_key(key) {
                return self.database.read().read(key);
            }
            let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
            if let Some(parent) = &self.parent
                && (has_remapping || (autoremap && parent.contains_key(&parent_key)))
            {
                return parent.get(&parent_key);
            }
            Err(Error::NotFound { key: key.into() })
        }
        /// Returns an [`RwLock`] guarded reference to the stored `T` for the `key`.
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        /// - [`Error::WrongType`] if the entry has not the expected type `T`
        pub fn get_ref<T: 'static>(&self, key: &str) -> Result<EntryGuard<T>> {
            if let Some(key_stripped) = key.strip_prefix('@') {
                return self.root().get_ref(key_stripped);
            }
            if self.database.read().contains_key(key) {
                return self.database.read().get_ref::<T>(key);
            }
            let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
            if let Some(parent) = &self.parent
                && (has_remapping || (autoremap && parent.contains_key(&parent_key)))
            {
                return parent.get_ref(&parent_key);
            }
            Err(Error::NotFound { key: key.into() })
        }
        /// Returns to the root [`Databoard`] of the hierarchy.
        fn root(&self) -> &Self {
            self.parent.as_ref().map_or(self, |board| board.root())
        }
        /// Read needed remapping information to parent.
        fn remapping_info(&self, key: &str) -> (ConstString, bool, bool) {
            let (remapped_key, has_remapping) = self
                .remappings
                .find(key)
                .map_or_else(|| (key.into(), false), |remapped| (remapped, true));
            (remapped_key, has_remapping, self.autoremap)
        }
        /// Returns the sequence id of an entry.
        /// The sequence id starts with '1' and is increased at every change of an entry.
        /// The sequence wraps around to '1' after reaching [`usize::MAX`] .
        /// # Errors
        /// - [`Error::NotFound`] if `key` is not contained
        pub fn sequence_id(&self, key: &str) -> Result<usize> {
            if let Some(key_stripped) = key.strip_prefix('@') {
                return self.root().sequence_id(key_stripped);
            }
            if let Ok(value) = self.database.read().sequence_id(key) {
                return Ok(value);
            }
            let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
            if let Some(parent) = &self.parent
                && (has_remapping || (autoremap && parent.contains_key(&parent_key)))
            {
                return parent.sequence_id(&parent_key);
            }
            Err(Error::NotFound { key: key.into() })
        }
        /// Stores a value of type `T` under `key` and returns an eventually existing value of type `T`.
        /// # Errors
        /// - [`Error::WrongType`] if `key` already exists with a different type
        pub fn set<T: Clone + Send + Sync + 'static>(
            &self,
            key: &str,
            value: T,
        ) -> Result<Option<T>> {
            if let Some(key_stripped) = key.strip_prefix('@') {
                return self.root().set(key_stripped, value);
            }
            if self.database.read().contains_key(key) {
                let old = self.database.read().update(key, value)?;
                return Ok(Some(old));
            }
            let (parent_key, has_remapping, autoremap) = self.remapping_info(key);
            if let Some(parent) = &self.parent
                && (has_remapping || (autoremap && parent.contains_key(&parent_key)))
            {
                return parent.set(&parent_key, value);
            }
            self.database.write().create(key, value)?;
            Ok(None)
        }
    }
}
mod entry {
    //! Implementation of the entry for a [`Databoard`](crate::databoard::Databoard).
    #![allow(dead_code, unused)]
    use crate::{error::Result, remappings::Remappings};
    use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc};
    use core::{any::Any, marker::PhantomData, ops::{Deref, DerefMut}};
    use ouroboros::self_referencing;
    use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};
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
    ///Encapsulates implementation details for a self-referencing struct. This module is only visible when using --document-private-items.
    mod ouroboros_impl_entry_guard_inner {
        use super::*;
        ///The self-referencing struct.
        #[repr(transparent)]
        pub(super) struct EntryGuardInner<T: 'static> {
            actual_data: ::core::mem::MaybeUninit<EntryGuardInnerInternal<T>>,
        }
        struct EntryGuardInnerInternal<T: 'static> {
            #[doc(hidden)]
            value: &'static T,
            #[doc(hidden)]
            guard: ::ouroboros::macro_help::AliasableBox<
                RwLockWriteGuard<'static, EntryData>,
            >,
            #[doc(hidden)]
            entry: ::ouroboros::macro_help::AliasableBox<EntryPtr>,
        }
        impl<T: 'static> ::core::ops::Drop for EntryGuardInner<T> {
            fn drop(&mut self) {
                unsafe { self.actual_data.assume_init_drop() };
            }
        }
        fn check_if_okay_according_to_checkers<T: 'static>(
            entry: EntryPtr,
            guard_builder: impl for<'this> ::core::ops::FnOnce(
                &'this mut EntryPtr,
            ) -> RwLockWriteGuard<'this, EntryData>,
            value_builder: impl for<'this> ::core::ops::FnOnce(
                &'this RwLockWriteGuard<'this, EntryData>,
            ) -> &'this T,
        ) {
            let mut entry = entry;
            let guard = guard_builder(&mut entry);
            let guard = guard;
            let value = value_builder(&guard);
            let value = value;
            BorrowedFields::<'_, '_, T> {
                guard: &guard,
                value: &value,
                _consume_template_type_t: ::core::marker::PhantomData,
            };
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`build()`](Self::build) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> guard: _` |
| `value_builder` | Use a function or closure: `(guard: &_) -> value: _` |
*/
        pub(super) struct EntryGuardInnerBuilder<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> RwLockWriteGuard<'this, EntryData>,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> &'this T,
        > {
            pub(super) entry: EntryPtr,
            pub(super) guard_builder: GuardBuilder_,
            pub(super) value_builder: ValueBuilder_,
        }
        impl<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> RwLockWriteGuard<'this, EntryData>,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> &'this T,
        > EntryGuardInnerBuilder<T, GuardBuilder_, ValueBuilder_> {
            ///Calls [`EntryGuardInner::new()`](EntryGuardInner::new) using the provided values. This is preferable over calling `new()` directly for the reasons listed above.
            pub(super) fn build(self) -> EntryGuardInner<T> {
                EntryGuardInner::new(self.entry, self.guard_builder, self.value_builder)
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`build()`](Self::build) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> guard: _` |
| `value_builder` | Use a function or closure: `(guard: &_) -> value: _` |
*/
        pub(super) struct EntryGuardInnerAsyncBuilder<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = RwLockWriteGuard<'this, EntryData>,
                            > + 'this,
                        >,
                    >,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<Output = &'this T> + 'this,
                        >,
                    >,
        > {
            pub(super) entry: EntryPtr,
            pub(super) guard_builder: GuardBuilder_,
            pub(super) value_builder: ValueBuilder_,
        }
        impl<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = RwLockWriteGuard<'this, EntryData>,
                            > + 'this,
                        >,
                    >,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<Output = &'this T> + 'this,
                        >,
                    >,
        > EntryGuardInnerAsyncBuilder<T, GuardBuilder_, ValueBuilder_> {
            ///Calls [`EntryGuardInner::new()`](EntryGuardInner::new) using the provided values. This is preferable over calling `new()` directly for the reasons listed above.
            pub(super) async fn build(self) -> EntryGuardInner<T> {
                EntryGuardInner::new_async(
                        self.entry,
                        self.guard_builder,
                        self.value_builder,
                    )
                    .await
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`build()`](Self::build) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> guard: _` |
| `value_builder` | Use a function or closure: `(guard: &_) -> value: _` |
*/
        pub(super) struct EntryGuardInnerAsyncSendBuilder<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = RwLockWriteGuard<'this, EntryData>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = &'this T,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
        > {
            pub(super) entry: EntryPtr,
            pub(super) guard_builder: GuardBuilder_,
            pub(super) value_builder: ValueBuilder_,
        }
        impl<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = RwLockWriteGuard<'this, EntryData>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = &'this T,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
        > EntryGuardInnerAsyncSendBuilder<T, GuardBuilder_, ValueBuilder_> {
            ///Calls [`EntryGuardInner::new()`](EntryGuardInner::new) using the provided values. This is preferable over calling `new()` directly for the reasons listed above.
            pub(super) async fn build(self) -> EntryGuardInner<T> {
                EntryGuardInner::new_async_send(
                        self.entry,
                        self.guard_builder,
                        self.value_builder,
                    )
                    .await
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`try_build()`](Self::try_build) or [`try_build_or_recover()`](Self::try_build_or_recover) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
        pub(super) struct EntryGuardInnerTryBuilder<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::result::Result<RwLockWriteGuard<'this, EntryData>, Error_>,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::result::Result<&'this T, Error_>,
            Error_,
        > {
            pub(super) entry: EntryPtr,
            pub(super) guard_builder: GuardBuilder_,
            pub(super) value_builder: ValueBuilder_,
        }
        impl<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::result::Result<RwLockWriteGuard<'this, EntryData>, Error_>,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::result::Result<&'this T, Error_>,
            Error_,
        > EntryGuardInnerTryBuilder<T, GuardBuilder_, ValueBuilder_, Error_> {
            ///Calls [`EntryGuardInner::try_new()`](EntryGuardInner::try_new) using the provided values. This is preferable over calling `try_new()` directly for the reasons listed above.
            pub(super) fn try_build(
                self,
            ) -> ::core::result::Result<EntryGuardInner<T>, Error_> {
                EntryGuardInner::try_new(
                    self.entry,
                    self.guard_builder,
                    self.value_builder,
                )
            }
            ///Calls [`EntryGuardInner::try_new_or_recover()`](EntryGuardInner::try_new_or_recover) using the provided values. This is preferable over calling `try_new_or_recover()` directly for the reasons listed above.
            pub(super) fn try_build_or_recover(
                self,
            ) -> ::core::result::Result<EntryGuardInner<T>, (Error_, Heads<T>)> {
                EntryGuardInner::try_new_or_recover(
                    self.entry,
                    self.guard_builder,
                    self.value_builder,
                )
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`try_build()`](Self::try_build) or [`try_build_or_recover()`](Self::try_build_or_recover) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
        pub(super) struct EntryGuardInnerAsyncTryBuilder<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    RwLockWriteGuard<'this, EntryData>,
                                    Error_,
                                >,
                            > + 'this,
                        >,
                    >,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<&'this T, Error_>,
                            > + 'this,
                        >,
                    >,
            Error_,
        > {
            pub(super) entry: EntryPtr,
            pub(super) guard_builder: GuardBuilder_,
            pub(super) value_builder: ValueBuilder_,
        }
        impl<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    RwLockWriteGuard<'this, EntryData>,
                                    Error_,
                                >,
                            > + 'this,
                        >,
                    >,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<&'this T, Error_>,
                            > + 'this,
                        >,
                    >,
            Error_,
        > EntryGuardInnerAsyncTryBuilder<T, GuardBuilder_, ValueBuilder_, Error_> {
            ///Calls [`EntryGuardInner::try_new()`](EntryGuardInner::try_new) using the provided values. This is preferable over calling `try_new()` directly for the reasons listed above.
            pub(super) async fn try_build(
                self,
            ) -> ::core::result::Result<EntryGuardInner<T>, Error_> {
                EntryGuardInner::try_new_async(
                        self.entry,
                        self.guard_builder,
                        self.value_builder,
                    )
                    .await
            }
            ///Calls [`EntryGuardInner::try_new_or_recover()`](EntryGuardInner::try_new_or_recover) using the provided values. This is preferable over calling `try_new_or_recover()` directly for the reasons listed above.
            pub(super) async fn try_build_or_recover(
                self,
            ) -> ::core::result::Result<EntryGuardInner<T>, (Error_, Heads<T>)> {
                EntryGuardInner::try_new_or_recover_async(
                        self.entry,
                        self.guard_builder,
                        self.value_builder,
                    )
                    .await
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`try_build()`](Self::try_build) or [`try_build_or_recover()`](Self::try_build_or_recover) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
        pub(super) struct EntryGuardInnerAsyncSendTryBuilder<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    RwLockWriteGuard<'this, EntryData>,
                                    Error_,
                                >,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<&'this T, Error_>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            Error_,
        > {
            pub(super) entry: EntryPtr,
            pub(super) guard_builder: GuardBuilder_,
            pub(super) value_builder: ValueBuilder_,
        }
        impl<
            T: 'static,
            GuardBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    RwLockWriteGuard<'this, EntryData>,
                                    Error_,
                                >,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            ValueBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<&'this T, Error_>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            Error_,
        > EntryGuardInnerAsyncSendTryBuilder<T, GuardBuilder_, ValueBuilder_, Error_> {
            ///Calls [`EntryGuardInner::try_new()`](EntryGuardInner::try_new) using the provided values. This is preferable over calling `try_new()` directly for the reasons listed above.
            pub(super) async fn try_build(
                self,
            ) -> ::core::result::Result<EntryGuardInner<T>, Error_> {
                EntryGuardInner::try_new_async_send(
                        self.entry,
                        self.guard_builder,
                        self.value_builder,
                    )
                    .await
            }
            ///Calls [`EntryGuardInner::try_new_or_recover()`](EntryGuardInner::try_new_or_recover) using the provided values. This is preferable over calling `try_new_or_recover()` directly for the reasons listed above.
            pub(super) async fn try_build_or_recover(
                self,
            ) -> ::core::result::Result<EntryGuardInner<T>, (Error_, Heads<T>)> {
                EntryGuardInner::try_new_or_recover_async_send(
                        self.entry,
                        self.guard_builder,
                        self.value_builder,
                    )
                    .await
            }
        }
        ///A struct for holding immutable references to all [tail and immutably borrowed fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) in an instance of [`EntryGuardInner`](EntryGuardInner).
        pub(super) struct BorrowedFields<'outer_borrow, 'this, T: 'static>
        where
            'static: 'this,
            'this: 'outer_borrow,
        {
            pub(super) value: &'outer_borrow &'this T,
            pub(super) guard: &'this RwLockWriteGuard<'this, EntryData>,
            _consume_template_type_t: ::core::marker::PhantomData<T>,
        }
        ///A struct for holding mutable references to all [tail fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) in an instance of [`EntryGuardInner`](EntryGuardInner).
        pub(super) struct BorrowedMutFields<'outer_borrow, 'this1, 'this0, T: 'static>
        where
            'static: 'this0,
            'static: 'this1,
            'this1: 'this0,
        {
            pub(super) value: &'outer_borrow mut &'this0 T,
            pub(super) guard: &'this1 RwLockWriteGuard<'this1, EntryData>,
            _consume_template_type_t: ::core::marker::PhantomData<T>,
        }
        ///A struct which contains only the [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) of [`EntryGuardInner`](EntryGuardInner).
        pub(super) struct Heads<T: 'static> {
            pub(super) entry: EntryPtr,
            _consume_template_type_t: ::core::marker::PhantomData<T>,
        }
        impl<T: 'static> EntryGuardInner<T> {
            /**Constructs a new instance of this self-referential struct. (See also [`EntryGuardInnerBuilder::build()`](EntryGuardInnerBuilder::build)). Each argument is a field of the new struct. Fields that refer to other fields inside the struct are initialized using functions instead of directly passing their value. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> guard: _` |
| `value_builder` | Use a function or closure: `(guard: &_) -> value: _` |
*/
            pub(super) fn new(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> RwLockWriteGuard<'this, EntryData>,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> &'this T,
            ) -> EntryGuardInner<T> {
                let mut entry = ::ouroboros::macro_help::aliasable_boxed(entry);
                let entry_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime_mut(&mut *entry)
                };
                let guard = guard_builder(entry_illegal_static_reference);
                let guard = ::ouroboros::macro_help::aliasable_boxed(guard);
                let guard_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*guard)
                };
                let value = value_builder(guard_illegal_static_reference);
                unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(EntryGuardInnerInternal {
                            entry,
                            guard,
                            value,
                        }),
                    }
                }
            }
            /**Constructs a new instance of this self-referential struct. (See also [`EntryGuardInnerAsyncBuilder::build()`](EntryGuardInnerAsyncBuilder::build)). Each argument is a field of the new struct. Fields that refer to other fields inside the struct are initialized using functions instead of directly passing their value. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> guard: _` |
| `value_builder` | Use a function or closure: `(guard: &_) -> value: _` |
*/
            pub(super) async fn new_async(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = RwLockWriteGuard<'this, EntryData>,
                            > + 'this,
                        >,
                    >,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<Output = &'this T> + 'this,
                        >,
                    >,
            ) -> EntryGuardInner<T> {
                let mut entry = ::ouroboros::macro_help::aliasable_boxed(entry);
                let entry_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime_mut(&mut *entry)
                };
                let guard = guard_builder(entry_illegal_static_reference).await;
                let guard = ::ouroboros::macro_help::aliasable_boxed(guard);
                let guard_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*guard)
                };
                let value = value_builder(guard_illegal_static_reference).await;
                unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(EntryGuardInnerInternal {
                            entry,
                            guard,
                            value,
                        }),
                    }
                }
            }
            /**Constructs a new instance of this self-referential struct. (See also [`EntryGuardInnerAsyncSendBuilder::build()`](EntryGuardInnerAsyncSendBuilder::build)). Each argument is a field of the new struct. Fields that refer to other fields inside the struct are initialized using functions instead of directly passing their value. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> guard: _` |
| `value_builder` | Use a function or closure: `(guard: &_) -> value: _` |
*/
            pub(super) async fn new_async_send(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = RwLockWriteGuard<'this, EntryData>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = &'this T,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            ) -> EntryGuardInner<T> {
                let mut entry = ::ouroboros::macro_help::aliasable_boxed(entry);
                let entry_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime_mut(&mut *entry)
                };
                let guard = guard_builder(entry_illegal_static_reference).await;
                let guard = ::ouroboros::macro_help::aliasable_boxed(guard);
                let guard_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*guard)
                };
                let value = value_builder(guard_illegal_static_reference).await;
                unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(EntryGuardInnerInternal {
                            entry,
                            guard,
                            value,
                        }),
                    }
                }
            }
            /**(See also [`EntryGuardInnerTryBuilder::try_build()`](EntryGuardInnerTryBuilder::try_build).) Like [`new`](Self::new), but builders for [self-referencing fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) can return results. If any of them fail, `Err` is returned. If all of them succeed, `Ok` is returned. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
            pub(super) fn try_new<Error_>(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::result::Result<RwLockWriteGuard<'this, EntryData>, Error_>,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::result::Result<&'this T, Error_>,
            ) -> ::core::result::Result<EntryGuardInner<T>, Error_> {
                EntryGuardInner::try_new_or_recover(entry, guard_builder, value_builder)
                    .map_err(|(error, _heads)| error)
            }
            /**(See also [`EntryGuardInnerTryBuilder::try_build_or_recover()`](EntryGuardInnerTryBuilder::try_build_or_recover).) Like [`try_new`](Self::try_new), but all [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) are returned in the case of an error. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
            pub(super) fn try_new_or_recover<Error_>(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::result::Result<RwLockWriteGuard<'this, EntryData>, Error_>,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::result::Result<&'this T, Error_>,
            ) -> ::core::result::Result<EntryGuardInner<T>, (Error_, Heads<T>)> {
                let mut entry = ::ouroboros::macro_help::aliasable_boxed(entry);
                let entry_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime_mut(&mut *entry)
                };
                let guard = match guard_builder(entry_illegal_static_reference) {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                entry: ::ouroboros::macro_help::unbox(entry),
                                _consume_template_type_t: ::core::marker::PhantomData,
                            },
                        ));
                    }
                };
                let guard = ::ouroboros::macro_help::aliasable_boxed(guard);
                let guard_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*guard)
                };
                let value = match value_builder(guard_illegal_static_reference) {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                entry: ::ouroboros::macro_help::unbox(entry),
                                _consume_template_type_t: ::core::marker::PhantomData,
                            },
                        ));
                    }
                };
                ::core::result::Result::Ok(unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(EntryGuardInnerInternal {
                            entry,
                            guard,
                            value,
                        }),
                    }
                })
            }
            /**(See also [`EntryGuardInnerAsyncTryBuilder::try_build()`](EntryGuardInnerAsyncTryBuilder::try_build).) Like [`new`](Self::new), but builders for [self-referencing fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) can return results. If any of them fail, `Err` is returned. If all of them succeed, `Ok` is returned. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
            pub(super) async fn try_new_async<Error_>(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    RwLockWriteGuard<'this, EntryData>,
                                    Error_,
                                >,
                            > + 'this,
                        >,
                    >,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<&'this T, Error_>,
                            > + 'this,
                        >,
                    >,
            ) -> ::core::result::Result<EntryGuardInner<T>, Error_> {
                EntryGuardInner::try_new_or_recover_async(
                        entry,
                        guard_builder,
                        value_builder,
                    )
                    .await
                    .map_err(|(error, _heads)| error)
            }
            /**(See also [`EntryGuardInnerAsyncTryBuilder::try_build_or_recover()`](EntryGuardInnerAsyncTryBuilder::try_build_or_recover).) Like [`try_new`](Self::try_new), but all [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) are returned in the case of an error. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
            pub(super) async fn try_new_or_recover_async<Error_>(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    RwLockWriteGuard<'this, EntryData>,
                                    Error_,
                                >,
                            > + 'this,
                        >,
                    >,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<&'this T, Error_>,
                            > + 'this,
                        >,
                    >,
            ) -> ::core::result::Result<EntryGuardInner<T>, (Error_, Heads<T>)> {
                let mut entry = ::ouroboros::macro_help::aliasable_boxed(entry);
                let entry_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime_mut(&mut *entry)
                };
                let guard = match guard_builder(entry_illegal_static_reference).await {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                entry: ::ouroboros::macro_help::unbox(entry),
                                _consume_template_type_t: ::core::marker::PhantomData,
                            },
                        ));
                    }
                };
                let guard = ::ouroboros::macro_help::aliasable_boxed(guard);
                let guard_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*guard)
                };
                let value = match value_builder(guard_illegal_static_reference).await {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                entry: ::ouroboros::macro_help::unbox(entry),
                                _consume_template_type_t: ::core::marker::PhantomData,
                            },
                        ));
                    }
                };
                ::core::result::Result::Ok(unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(EntryGuardInnerInternal {
                            entry,
                            guard,
                            value,
                        }),
                    }
                })
            }
            /**(See also [`EntryGuardInnerAsyncSendTryBuilder::try_build()`](EntryGuardInnerAsyncSendTryBuilder::try_build).) Like [`new`](Self::new), but builders for [self-referencing fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) can return results. If any of them fail, `Err` is returned. If all of them succeed, `Ok` is returned. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
            pub(super) async fn try_new_async_send<Error_>(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    RwLockWriteGuard<'this, EntryData>,
                                    Error_,
                                >,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<&'this T, Error_>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            ) -> ::core::result::Result<EntryGuardInner<T>, Error_> {
                EntryGuardInner::try_new_or_recover_async_send(
                        entry,
                        guard_builder,
                        value_builder,
                    )
                    .await
                    .map_err(|(error, _heads)| error)
            }
            /**(See also [`EntryGuardInnerAsyncSendTryBuilder::try_build_or_recover()`](EntryGuardInnerAsyncSendTryBuilder::try_build_or_recover).) Like [`try_new`](Self::try_new), but all [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) are returned in the case of an error. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `entry` | Directly pass in the value this field should contain |
| `guard_builder` | Use a function or closure: `(entry: &mut _) -> Result<guard: _, Error_>` |
| `value_builder` | Use a function or closure: `(guard: &_) -> Result<value: _, Error_>` |
*/
            pub(super) async fn try_new_or_recover_async_send<Error_>(
                entry: EntryPtr,
                guard_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this mut EntryPtr,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    RwLockWriteGuard<'this, EntryData>,
                                    Error_,
                                >,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
                value_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this RwLockWriteGuard<'this, EntryData>,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<&'this T, Error_>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            ) -> ::core::result::Result<EntryGuardInner<T>, (Error_, Heads<T>)> {
                let mut entry = ::ouroboros::macro_help::aliasable_boxed(entry);
                let entry_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime_mut(&mut *entry)
                };
                let guard = match guard_builder(entry_illegal_static_reference).await {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                entry: ::ouroboros::macro_help::unbox(entry),
                                _consume_template_type_t: ::core::marker::PhantomData,
                            },
                        ));
                    }
                };
                let guard = ::ouroboros::macro_help::aliasable_boxed(guard);
                let guard_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*guard)
                };
                let value = match value_builder(guard_illegal_static_reference).await {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                entry: ::ouroboros::macro_help::unbox(entry),
                                _consume_template_type_t: ::core::marker::PhantomData,
                            },
                        ));
                    }
                };
                ::core::result::Result::Ok(unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(EntryGuardInnerInternal {
                            entry,
                            guard,
                            value,
                        }),
                    }
                })
            }
            ///Provides limited immutable access to `guard`. This method was generated because the contents of `guard` are immutably borrowed by other fields.
            #[inline(always)]
            pub(super) fn with_guard<'outer_borrow, ReturnType>(
                &'outer_borrow self,
                user: impl for<'this> ::core::ops::FnOnce(
                    &'outer_borrow RwLockWriteGuard<'this, EntryData>,
                ) -> ReturnType,
            ) -> ReturnType {
                let field = &unsafe { self.actual_data.assume_init_ref() }.guard;
                user(field)
            }
            ///Provides limited immutable access to `guard`. This method was generated because the contents of `guard` are immutably borrowed by other fields.
            #[inline(always)]
            pub(super) fn borrow_guard<'this>(
                &'this self,
            ) -> &'this RwLockWriteGuard<'this, EntryData> {
                &unsafe { self.actual_data.assume_init_ref() }.guard
            }
            ///Provides an immutable reference to `value`. This method was generated because `value` is a [tail field](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn with_value<'outer_borrow, ReturnType>(
                &'outer_borrow self,
                user: impl for<'this> ::core::ops::FnOnce(
                    &'outer_borrow &'this T,
                ) -> ReturnType,
            ) -> ReturnType {
                let field = &unsafe { self.actual_data.assume_init_ref() }.value;
                user(field)
            }
            ///Provides an immutable reference to `value`. This method was generated because `value` is a [tail field](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn borrow_value<'this>(&'this self) -> &'this &'this T {
                &unsafe { self.actual_data.assume_init_ref() }.value
            }
            ///Provides a mutable reference to `value`. This method was generated because `value` is a [tail field](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions). No `borrow_value_mut` function was generated because Rust's borrow checker is currently unable to guarantee that such a method would be used safely.
            #[inline(always)]
            pub(super) fn with_value_mut<'outer_borrow, ReturnType>(
                &'outer_borrow mut self,
                user: impl for<'this> ::core::ops::FnOnce(
                    &'outer_borrow mut &'this T,
                ) -> ReturnType,
            ) -> ReturnType {
                let field = &mut unsafe { self.actual_data.assume_init_mut() }.value;
                user(field)
            }
            ///This method provides immutable references to all [tail and immutably borrowed fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn with<'outer_borrow, ReturnType>(
                &'outer_borrow self,
                user: impl for<'this> ::core::ops::FnOnce(
                    BorrowedFields<'outer_borrow, 'this, T>,
                ) -> ReturnType,
            ) -> ReturnType {
                let this = unsafe { self.actual_data.assume_init_ref() };
                user(BorrowedFields {
                    value: &this.value,
                    guard: unsafe {
                        ::ouroboros::macro_help::change_lifetime(&*this.guard)
                    },
                    _consume_template_type_t: ::core::marker::PhantomData,
                })
            }
            ///This method provides mutable references to all [tail fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn with_mut<'outer_borrow, ReturnType>(
                &'outer_borrow mut self,
                user: impl for<'this0, 'this1> ::core::ops::FnOnce(
                    BorrowedMutFields<'outer_borrow, 'this1, 'this0, T>,
                ) -> ReturnType,
            ) -> ReturnType {
                let this = unsafe { self.actual_data.assume_init_mut() };
                user(BorrowedMutFields {
                    value: &mut this.value,
                    guard: unsafe {
                        ::ouroboros::macro_help::change_lifetime(&*this.guard)
                    },
                    _consume_template_type_t: ::core::marker::PhantomData,
                })
            }
            ///This function drops all internally referencing fields and returns only the [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) of this struct.
            #[allow(clippy::drop_ref)]
            #[allow(clippy::drop_copy)]
            #[allow(clippy::drop_non_drop)]
            pub(super) fn into_heads(self) -> Heads<T> {
                let this_ptr = &self as *const _;
                let this: EntryGuardInnerInternal<T> = unsafe {
                    ::core::mem::transmute_copy(&*this_ptr)
                };
                ::core::mem::forget(self);
                ::core::mem::drop(this.value);
                ::core::mem::drop(this.guard);
                let entry = this.entry;
                Heads {
                    entry: ::ouroboros::macro_help::unbox(entry),
                    _consume_template_type_t: ::core::marker::PhantomData,
                }
            }
        }
        fn type_asserts<T: 'static>() {}
    }
    use ouroboros_impl_entry_guard_inner::EntryGuardInner;
    use ouroboros_impl_entry_guard_inner::EntryGuardInnerBuilder;
    use ouroboros_impl_entry_guard_inner::EntryGuardInnerAsyncBuilder;
    use ouroboros_impl_entry_guard_inner::EntryGuardInnerAsyncSendBuilder;
    use ouroboros_impl_entry_guard_inner::EntryGuardInnerTryBuilder;
    use ouroboros_impl_entry_guard_inner::EntryGuardInnerAsyncTryBuilder;
    use ouroboros_impl_entry_guard_inner::EntryGuardInnerAsyncSendTryBuilder;
    /// A reference to the locked [`EntryData`].
    #[repr(transparent)]
    pub struct EntryRef(pub(crate) EntryPtr);
    #[automatically_derived]
    impl ::core::clone::Clone for EntryRef {
        #[inline]
        fn clone(&self) -> EntryRef {
            EntryRef(::core::clone::Clone::clone(&self.0))
        }
    }
    impl EntryRef {
        pub(crate) fn new<T: Send + Sync + 'static>(value: T) -> Self {
            let data = EntryData {
                data: Box::new(value),
                sequence_id: usize::MIN + 1,
            };
            Self(Arc::new(RwLock::new(data)))
        }
    }
    /// Write-Locked entry.
    /// Until this value is dropped, the lock is held on the entry.
    ///
    /// Implements [`Deref`], providing access to the locked `T`.
    pub struct EntryGuard<T: 'static>(EntryGuardInner<T>);
    impl<T: 'static> EntryGuard<T> {
        /// Attempts to downcast the `Box<dyn Any + Send>` to `T`. If downcasting
        /// succeeds, wraps the value and lock into [`EntryGuard`].
        pub fn create(entry: EntryPtr) -> Option<Self> {
            let is_valid = entry.read().downcast_ref::<T>().is_some();
            if is_valid {
                let inner = EntryGuardInner::new(
                    entry,
                    |entry| entry.write(),
                    |guard| {
                        guard
                            .downcast_ref::<T>()
                            .expect("downcasting should be possible")
                    },
                );
                Some(Self(inner))
            } else {
                None
            }
        }
    }
    impl<T: 'static> Deref for EntryGuard<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            self.0.borrow_value()
        }
    }
    impl<T: 'static> DerefMut for EntryGuard<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            ::core::panicking::panic("not yet implemented")
        }
    }
}
mod error {
    //! The [`Databoard`](crate::Databoard) error handling.
    use crate::ConstString;
    /// Shortcut for [`Databoard`](crate::Databoard)'s Result<T, E> type
    pub type Result<T> = core::result::Result<T, Error>;
    /// Things that may go wrong using the [`Databoard`](crate::Databoard).
    #[non_exhaustive]
    pub enum Error {
        /// Entry with `key` already exists.
        AlreadyExists {
            /// Key of the entry to create.
            key: ConstString,
        },
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
    impl core::error::Error for Error {}
    impl core::fmt::Debug for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::AlreadyExists { key } => {
                    f.write_fmt(format_args!("AlreadyExists(key: {0}", key))
                }
                Self::AlreadyRemapped { key, remapped } => {
                    f.write_fmt(
                        format_args!(
                            "AlreadyRemapped(key: {0}, remapped: {1}",
                            key,
                            remapped,
                        ),
                    )
                }
                Self::NotFound { key } => {
                    f.write_fmt(format_args!("NotFound(key: {0}", key))
                }
                Self::WrongType { key } => {
                    f.write_fmt(format_args!("WrongType(key: {0}", key))
                }
                Self::Unexpected(file, line) => {
                    f.write_fmt(
                        format_args!("Unexpected(file: {0}, line: {1}", file, line),
                    )
                }
            }
        }
    }
    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::AlreadyExists { key } => {
                    f.write_fmt(
                        format_args!(
                            "cannot create data with key {0} as they already exist",
                            key,
                        ),
                    )
                }
                Self::AlreadyRemapped { key, remapped } => {
                    f.write_fmt(
                        format_args!("key {0} is already remapped as {1}", key, remapped),
                    )
                }
                Self::NotFound { key } => {
                    f.write_fmt(
                        format_args!("an entry for the key {0} is not existing", key),
                    )
                }
                Self::WrongType { key } => {
                    f.write_fmt(
                        format_args!(
                            "the entry for the key {0} is stored with a different type",
                            key,
                        ),
                    )
                }
                Self::Unexpected(file, line) => {
                    f.write_fmt(
                        format_args!(
                            "an unexpected error occured in {0} at line {1}",
                            file,
                            line,
                        ),
                    )
                }
            }
        }
    }
}
mod remappings {
    //! [`Databoard`][`Remappings`] implementation.
    use super::error::{Error, Result};
    use crate::ConstString;
    use alloc::{borrow::ToOwned, string::String, vec::Vec};
    use core::ops::{Deref, DerefMut};
    /// An immutable remapping entry.
    type RemappingEntry = (ConstString, ConstString);
    /// A mutable remapping list.
    #[repr(transparent)]
    pub struct Remappings(Vec<RemappingEntry>);
    #[automatically_derived]
    impl ::core::clone::Clone for Remappings {
        #[inline]
        fn clone(&self) -> Remappings {
            Remappings(::core::clone::Clone::clone(&self.0))
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Remappings {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Remappings", &&self.0)
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for Remappings {
        #[inline]
        fn default() -> Remappings {
            Remappings(::core::default::Default::default())
        }
    }
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
        pub fn add(
            &mut self,
            key: impl Into<ConstString>,
            remap_to: impl Into<ConstString>,
        ) -> Result<()> {
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
                    *old_value = remapped_name.into();
                    return;
                }
            }
            self.0.push((name.into(), remapped_name.into()));
        }
        /// Lookup the remapped name.
        #[must_use]
        pub fn find(&self, name: &str) -> Option<ConstString> {
            for (original, remapped) in &self.0 {
                if original.as_ref() == name {
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
}
use alloc::sync::Arc;
pub use databoard::{Databoard, DataboardPtr};
pub use error::{Error, Result};
pub use remappings::Remappings;
/// An immutable thread safe `String` type
/// see: [Logan Smith](https://www.youtube.com/watch?v=A4cKi7PTJSs).
type ConstString = Arc<str>;
