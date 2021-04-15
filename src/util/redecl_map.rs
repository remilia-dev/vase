// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::{
    collections::{
        hash_map::{
            Entry,
            Keys,
        },
        HashMap,
    },
    hash::Hash,
};

use smallvec::{
    smallvec,
    SmallVec,
};

use crate::{
    math::NonMaxU32,
    util::Vec32,
};

/// An index into a [RedeclMap].
#[derive(Copy, Clone, Debug)]
pub struct RedeclMapIndex {
    /// The index the key's values are stored at.
    pub index: NonMaxU32,
    /// The index into the values.
    pub redecl_index: NonMaxU32,
}
/// A map where keys can be 'redeclared' to have a new value.
///
/// However, no old values are gotten rid of. In fact, old values can still be
/// accessed using the [RedeclMapIndex] returned when adding the value.
///
/// Values can also be unkeyed. These values have no corresponding key and are
/// *only* accessible from the [RedeclMapIndex] that was returned.
#[derive(Clone, Debug)]
pub struct RedeclMap<K: Hash + Eq, V> {
    by_name: HashMap<K, NonMaxU32>,
    items: Vec32<SmallVec<[V; 1]>>,
}

impl<K: Hash + Eq, V> RedeclMap<K, V> {
    /// Creates an empty RedeclMap.
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
            items: Vec32::new(),
        }
    }
    /// Adds a value to the map with a potential key. The index this value
    /// can be found at is returned.
    pub fn add(&mut self, k: Option<K>, v: V) -> RedeclMapIndex {
        if let Some(name) = k {
            self.add_keyed(name, v)
        } else {
            self.add_unkeyed(v)
        }
    }
    /// Adds a value that corresponds to the given key. If a value already exists
    /// for this key, this value is added to the list of redeclarations.
    pub fn add_keyed(&mut self, k: K, v: V) -> RedeclMapIndex {
        let index = match self.by_name.entry(k) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let index = self.items.len();
                self.items.push(SmallVec::new());
                entry.insert(index);
                index
            },
        };

        let redecl_index = NonMaxU32::new_usize(self.items[index].len()).unwrap();
        self.items[index].push(v);
        RedeclMapIndex { index, redecl_index }
    }
    /// Adds a value that has no corresponding key. Returns an index that
    /// corresponds to this un-keyed value. This value can never be redeclared.
    #[must_use]
    pub fn add_unkeyed(&mut self, v: V) -> RedeclMapIndex {
        let decl = self.items.len();
        self.items.push(smallvec![v]);
        RedeclMapIndex { index: decl, redecl_index: 0.into() }
    }
    /// Returns an index that represents the last value of the given key.
    pub fn get_index(&self, k: &K) -> Option<RedeclMapIndex> {
        let index = *self.by_name.get(k)?;
        let redecl_index = NonMaxU32::new_usize(self.items[index].len() - 1).unwrap();
        Some(RedeclMapIndex { index, redecl_index })
    }
    /// Returns a reference to the value corresponding to the index.
    pub fn get(&self, index: RedeclMapIndex) -> Option<&V> {
        let item_list = self.items.get(index.index.get())?;
        item_list.get(index.redecl_index.get() as usize)
    }
    /// Returns a mutable reference to the value corresponding to the index.
    pub fn get_mut(&mut self, index: RedeclMapIndex) -> Option<&mut V> {
        let item_list = self.items.get_mut(index.index.get())?;
        item_list.get_mut(index.redecl_index.get() as usize)
    }
    /// Returns an iterator over all the keys.
    pub fn keys(&self) -> Keys<K, NonMaxU32> {
        self.by_name.keys()
    }
}

impl<K: Hash + Eq, V> Default for RedeclMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Hash + Eq, V> std::ops::Index<RedeclMapIndex> for RedeclMap<K, V> {
    type Output = V;

    fn index(&self, index: RedeclMapIndex) -> &Self::Output {
        self.get(index).expect("Index out of range.")
    }
}

impl<K: Hash + Eq, V> std::ops::IndexMut<RedeclMapIndex> for RedeclMap<K, V> {
    fn index_mut(&mut self, index: RedeclMapIndex) -> &mut Self::Output {
        self.get_mut(index).expect("Index out of range.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unkeyed_cannot_be_redeclared() {
        let mut map = RedeclMap::new();
        assert_eq!(map.add_unkeyed("UNKEYED VALUE 1").redecl_index, 0.into());
        assert_eq!(map.add_unkeyed("UNKEYED VALUE 2").redecl_index, 0.into());
        assert_eq!(map.add_keyed("KEY 1", "KEYED 1").redecl_index, 0.into());
        assert_eq!(map.add_unkeyed("UNKEYED VALUE 3").redecl_index, 0.into());
    }

    #[test]
    fn redeclaration_occurs() {
        let mut map = RedeclMap::new();
        let index1 = map.add_keyed("KEY 1", "VALUE 1");
        let index2 = map.add_keyed("KEY 1", "VALUE 2");
        assert_eq!(index1.index, index2.index);
        assert_eq!(index2.redecl_index, 1.into());
    }

    #[test]
    fn can_get_with_returned_index() {
        let mut map = RedeclMap::new();
        let index1 = map.add_keyed("DUMMY KEYED", "DUMMY 1");
        assert_eq!(map[index1], "DUMMY 1");
        let index2 = map.add_unkeyed("DUMMY 2");
        assert_eq!(map[index2], "DUMMY 2");
    }

    #[test]
    fn can_add_with_option_key() {
        let mut map = RedeclMap::new();
        // Unkeyed:
        assert_eq!(map.add(None, "VALUE 1").index, 0.into());
        assert_eq!(map.add(None, "VALUE 2").index, 1.into());
        // Keyed:
        assert_eq!(map.add(Some("KEY 1"), "VALUE 3").index, 2.into());
        assert_eq!(map.add(Some("KEY 1"), "VALUE 4").index, 2.into());
    }
}
