use core::fmt::{Display, Formatter};

use alloc::vec::Vec;

/// A sparse, generational map keyed by [`Key`].
///
/// `SparseMap` provides stable keys with versioning to prevent
/// use-after-free bugs. Internally, it reuses vacant slots while
/// incrementing a generation counter to invalidate stale keys.
///
/// ## Guarantees
///
/// - Insertion is **O(1)**.
/// - Removal is **O(1)**.
/// - Lookup is **O(1)**.
/// - Keys are invalidated once their value is removed.
#[derive(Debug)]
pub struct SparseMap<T> {
    buffer: Vec<Item<T>>,
    empty_slots: Vec<usize>,
}

impl<T> SparseMap<T> {
    /// Creates a new empty sparse map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a value into the map and returns a unique [`Key`].
    ///
    /// Vacant slots are reused when possible. If a slot is reused,
    /// its generation counter is incremented to invalidate old keys.
    #[must_use = "The returned key is the only way to reference back the inserted value!"]
    pub fn insert(&mut self, value: T) -> Key {
        self.alloc_slot(
            value,
            |value, item| item.replace(value),
            |value| Item::new(value),
        )
    }

    /// Similar to [`Self::insert()`] but provides a [`Key`] before
    /// inserting the value.
    pub fn insert_with_key<F>(&mut self, f: F) -> Key
    where
        F: FnOnce(&mut Self, Key) -> T,
    {
        let key = self.alloc_slot(
            (),
            |_, item| item.replace_empty(),
            |_| Item::new_empty(),
        );

        let value = f(self, key);
        self.buffer[key.index].inner = Some(value);

        key
    }

    /// Returns [`Key`], reusing a vacant slot or allocating a new one.
    /// The caller decides how the slot is initialized.
    ///
    /// `value`: The inner value to be inserted.
    /// `replace`: Determine how a vacant slot will be replaced.
    /// `create`: Determine how a new item will be created.
    fn alloc_slot<V, R, C>(
        &mut self,
        value: V,
        replace: R,
        create: C,
    ) -> Key
    where
        R: FnOnce(V, &mut Item<T>) -> u32,
        C: FnOnce(V) -> Item<T>,
    {
        if let Some(index) = self.empty_slots.pop() {
            let version = replace(value, &mut self.buffer[index]);
            Key::new(index, version)
        } else {
            let index = self.buffer.len();
            let item = create(value);
            let version = item.version;
            self.buffer.push(item);
            Key::new(index, version)
        }
    }

    /// Removes a value associated with the given key.
    ///
    /// Returns `None` if the key is invalid or already removed.
    /// The slot is marked for reuse.
    pub fn remove(&mut self, key: &Key) -> Option<T> {
        let item = self.buffer.get_mut(key.index)?;
        self.empty_slots.push(key.index);
        item.take()
    }

    /// Returns an immutable reference to the value for the given
    /// key if present.
    pub fn get(&self, key: &Key) -> Option<&T> {
        let item = self.buffer.get(key.index)?;
        if item.version == key.version {
            return item.inner.as_ref();
        }

        None
    }

    /// Returns a mutable reference to the value for the given key
    /// if present.
    pub fn get_mut(&mut self, key: &Key) -> Option<&mut T> {
        let item = self.buffer.get_mut(key.index)?;
        if item.version == key.version {
            return item.inner.as_mut();
        }

        None
    }

    pub fn scope<F, R>(&mut self, key: &Key, f: F) -> Option<R>
    where
        F: FnOnce(&mut Self, &mut T) -> R,
    {
        if !self.contains(key) {
            return None;
        }

        // SAFETY: We already checked that the key contains a value.
        let mut value = self.buffer[key.index].take().unwrap();
        let result = f(self, &mut value);
        self.buffer[key.index].inner = Some(value);

        Some(result)
    }

    /// Returns `true` if the key currently refers to a live value.
    pub fn contains(&self, key: &Key) -> bool {
        self.buffer.get(key.index).is_some_and(|item| {
            item.inner.is_some() && item.version == key.version
        })
    }

    // FIXME: This is incorrect as newly added values may make old
    // keys vaild again when they shouldn't be.
    // TODO: Should we just not support clearing a map?
    // /// Removes all values and invalidates all keys.
    // pub fn clear(&mut self) {
    //     self.buffer.clear();
    //     self.empty_slots.clear();
    // }

    pub fn len(&self) -> usize {
        self.buffer.len() - self.empty_slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // FIXME: Make this more robust? Or we don't need it?
    // pub fn iter<'a>(&'a self) -> SparseMapIter<'a, T> {
    //     SparseMapIter {
    //         buffer: &self.buffer,
    //         curr_index: 0,
    //         len: self.len(),
    //     }
    // }
}

// Helper default impl for tests
impl<T> Default for SparseMap<T> {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            empty_slots: Vec::new(),
        }
    }
}

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Key {
    index: usize,
    version: u32,
}

impl Key {
    fn new(index: usize, version: u32) -> Self {
        Self { index, version }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn version(&self) -> u32 {
        self.version
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("#{}v{}", self.index, self.version))
    }
}

/// A versioned item holder used by [`SparseMap`].
#[derive(Debug)]
struct Item<T> {
    inner: Option<T>,
    version: u32,
}

impl<T> Item<T> {
    const fn new(value: T) -> Self {
        Self {
            inner: Some(value),
            version: 0,
        }
    }

    const fn new_empty() -> Self {
        Self {
            inner: None,
            version: 0,
        }
    }

    const fn take(&mut self) -> Option<T> {
        self.inner.take()
    }

    fn replace(&mut self, value: T) -> u32 {
        self.inner.replace(value);
        self.version = self.version.wrapping_add(1);
        self.version
    }

    fn replace_empty(&mut self) -> u32 {
        self.inner = None;
        self.version = self.version.wrapping_add(1);
        self.version
    }
}

// pub struct SparseMapIter<'a, T> {
//     buffer: &'a [SparseItem<T>],
//     curr_index: usize,
//     len: usize,
// }

// impl<'a, T> Iterator for SparseMapIter<'a, T> {
//     type Item = &'a T;

//     fn next(&mut self) -> Option<Self::Item> {
//         let res = self.buffer[self.curr_index..]
//             .iter()
//             .enumerate()
//             .filter_map(|(i, item)| item.inner.as_ref().map(|item| (i, item)))
//             .next();

//         if let Some((i, item)) = res {
//             self.curr_index += i + 1;
//             return Some(item);
//         }

//         None
//     }

//     fn last(self) -> Option<Self::Item>
//     where
//         Self: Sized,
//     {
//         self.buffer
//             .iter()
//             .rev()
//             .filter_map(|i| i.inner.as_ref())
//             .next()
//     }

//     fn count(self) -> usize
//     where
//         Self: Sized,
//     {
//         self.len
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut map = SparseMap::new();

        let key = map.insert(42);
        assert_eq!(map.get(&key), Some(&42));
        assert!(map.contains(&key));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn insert_with_key_receives_valid_key_before_insert() {
        let mut map = SparseMap::new();

        let key = map.insert_with_key(|map, key| {
            // Key must already be valid and point to the slot.
            assert_eq!(key.index, 0);
            assert!(map.buffer[key.index].inner.is_none());

            42
        });

        assert_eq!(map.get(&key), Some(&42));
    }

    #[test]
    fn insert_and_insert_with_key_behave_equivalently() {
        let mut map = SparseMap::new();

        let k1 = map.insert(1);
        let k2 = map.insert_with_key(|_, _| 2);

        assert_eq!(k1.index, 0);
        assert_eq!(k2.index, 1);

        assert_eq!(map.buffer[k1.index].inner, Some(1));
        assert_eq!(map.buffer[k2.index].inner, Some(2));
    }

    #[test]
    fn remove_invalidates_key() {
        let mut map = SparseMap::new();

        let key = map.insert(10);
        let removed = map.remove(&key);

        assert_eq!(removed, Some(10));
        assert_eq!(map.get(&key), None);
        assert!(!map.contains(&key));
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn insert_reuse_bumps_version() {
        let mut map = SparseMap::new();

        let k1 = map.insert(1);
        map.remove(&k1);

        let k2 = map.insert(2);

        assert_eq!(k1.index, k2.index);
        assert_ne!(k1.version, k2.version);

        assert_eq!(map.get(&k1), None);
        assert_eq!(map.get(&k2), Some(&2));
    }

    #[test]
    fn insert_with_key_reuse_bumps_version() {
        let mut map = SparseMap::new();

        let k1 = map.insert_with_key(|_, _| 1);
        map.remove(&k1);

        let k2 = map.insert_with_key(|_, _| 2);

        assert_eq!(k1.index, k2.index);
        assert_ne!(k1.version, k2.version);

        assert_eq!(map.get(&k1), None);
        assert_eq!(map.get(&k2), Some(&2));
    }

    #[test]
    fn get_mut_works() {
        let mut map = SparseMap::new();

        let key = map.insert(5);
        *map.get_mut(&key).unwrap() = 99;

        assert_eq!(map.get(&key), Some(&99));
    }

    #[test]
    fn removing_twice_is_safe() {
        let mut map = SparseMap::new();

        let key = map.insert(7);
        assert_eq!(map.remove(&key), Some(7));
        assert_eq!(map.remove(&key), None);
    }

    #[test]
    fn invalid_index_returns_none() {
        let mut map = SparseMap::<usize>::new();

        let fake_key = Key::new(999, 0);
        assert_eq!(map.get(&fake_key), None);
        assert_eq!(map.get_mut(&fake_key), None);
        assert!(!map.contains(&fake_key));
    }

    // #[test]
    // fn clear_resets_map() {
    //     let mut map = SparseMap::new();

    //     map.insert(1);
    //     map.insert(2);
    //     map.clear();

    //     assert!(map.is_empty());
    // }
}
