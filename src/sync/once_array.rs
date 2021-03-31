// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    math::NonMaxU32,
    sync::{
        Arc,
        AtomicArc,
        AtomicBox,
        AtomicU32,
        Ordering,
    },
    util::mem::make_static_array,
};

// NOTE: This limits us to 2048 * 50 (102400) indices.
/// The number of elements a single OnceArrayNode can hold.
const NODE_SIZE: usize = 2048;
/// The number of OnceArrayNodes a OnceArray can hold.
const NODE_COUNT: usize = 50;
/// The total number of values that can be stored in a OnceArray.
const MAX_VALUES: usize = NODE_SIZE * NODE_COUNT;

/// A grow-only array that can initialize indexes individually.
///
/// This array uses [AtomicArc]s to initialize individual elements.
/// As such, each element of a OnceArray must be contained in an Arc.
/// A thread can reserve an index and fill it later (or have another thread
/// fill it).
///
/// A thread could also reserve an index and not fill it in. However,
/// this is discouraged.
///
/// # Max Size
/// Currently, the array can not grow beyond 102,400 indices.
/// Growing beyond that will result in a panic when the slot is set.
pub struct OnceArray<T> {
    nodes: [AtomicBox<OnceArrayNode<T>>; NODE_COUNT],
    accum: AtomicU32,
}

impl<T> OnceArray<T> {
    pub fn new() -> Self {
        OnceArray {
            nodes: make_static_array::<_, NODE_COUNT>(&|| AtomicBox::default()),
            accum: 0.into(),
        }
    }
    /// Reserves an index to be set later. This index is guaranteed to be unique.
    pub fn reserve(&self) -> Option<NonMaxU32> {
        // NOTE: We use a u32 because the maximum number of indices can fit within u32.
        // Also, we know outside code will use u32 more than usize.
        let index = self.accum.fetch_add(1, Ordering::SeqCst) as u32;
        if index < MAX_VALUES as u32 {
            // SAFETY: MAX_VALUES is less than the maximum of a u32.
            Some(unsafe { NonMaxU32::new_unchecked(index) })
        } else {
            // Subtract 1 to ensure that index accumulator never wraps.
            self.accum.fetch_sub(1, Ordering::SeqCst);
            None
        }
    }
    /// Tries to get the value at a specific index. If that index has not been initialized,
    /// it will return None.
    pub fn get(&self, index: NonMaxU32) -> Option<&T> {
        self.get_node(index)?.get(index)
    }
    /// Tries to get the Arc of the value at a specific index. If that index has not been
    /// initialized, it will return None.
    pub fn get_arc(&self, index: NonMaxU32) -> Option<Arc<T>> {
        self.get_node(index)?.get_arc(index)
    }
    /// Adds a value onto the array and returns the index the value is at.
    pub fn push(&self, val: Arc<T>) -> NonMaxU32 {
        let index = self.reserve().unwrap();
        self.set_or_panic(index, val);
        index
    }
    /// Sets the value at the given index. Since this method has exclusive mutability,
    /// it can also set a value to None.
    pub fn set_mut(&mut self, index: NonMaxU32, val: Option<Arc<T>>) {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        if self.accum.load(Ordering::SeqCst) <= index.get() {
            panic!("Cannot set a value in a non-reserved index.")
        }
        self.ensure_node_for_index_mut(index).set_mut(index, val);
    }
    /// Sets the value at the given index.
    /// # Panics
    /// Panics if this index has already been set.
    pub fn set_or_panic(&self, index: NonMaxU32, val: Arc<T>) {
        if !self.set_if_none(index, val) {
            panic!("Cannot set a value in a once-array that has already been initialized.");
        }
    }
    /// Tries to set the value at the given index. If that value has been already set,
    /// the value is discarded. Returns whether the value was set or not.
    pub fn set_if_none(&self, index: NonMaxU32, val: Arc<T>) -> bool {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        if self.accum.load(Ordering::SeqCst) <= index.get() {
            panic!("Cannot set a value in a non-reserved index.")
        }
        self.ensure_node_for_index(index).try_set(index, val)
    }

    /// Gets the node that includes the given index.
    /// If that node does not exist, None will be returned.
    fn get_node(&self, index: NonMaxU32) -> Option<&OnceArrayNode<T>> {
        self.nodes.get(node_index(index))?.load()
    }
    /// Ensures that the node exists. It will then return either the existing node or
    /// the newly created node.
    fn ensure_node_for_index(&self, index: NonMaxU32) -> &OnceArrayNode<T> {
        self.nodes[node_index(index)].load_or_else(|| Box::new(OnceArrayNode::default()))
    }
    /// Ensures that the node exists. It will then return a mutable reference to
    /// the existing or newly-created node.
    fn ensure_node_for_index_mut(&mut self, index: NonMaxU32) -> &mut OnceArrayNode<T> {
        self.nodes[node_index(index)].get_or_else(|| Box::new(OnceArrayNode::default()))
    }
}

impl<T> std::ops::Index<NonMaxU32> for OnceArray<T> {
    type Output = T;

    /// Gets the value at the given index.
    /// # Panics
    /// Panics if the index is out of bounds or if that index has not been set.
    fn index(&self, index: NonMaxU32) -> &T {
        return self.get(index).unwrap();
    }
}

impl<T> Default for OnceArray<T> {
    fn default() -> Self {
        OnceArray::new()
    }
}

/// Contains a limited number of values for a once array.
struct OnceArrayNode<T> {
    values: [AtomicArc<T>; NODE_SIZE],
}
impl<T> OnceArrayNode<T> {
    fn get(&self, index: NonMaxU32) -> Option<&T> {
        self.values[self.val_index(index)].load()
    }

    fn get_arc(&self, index: NonMaxU32) -> Option<Arc<T>> {
        self.values[self.val_index(index)].load_arc()
    }

    fn try_set(&self, index: NonMaxU32, v: Arc<T>) -> bool {
        let slot = &self.values[self.val_index(index)];
        return matches!(slot.try_set_if_none(v), Ok(_));
    }

    fn set_mut(&mut self, index: NonMaxU32, v: Option<Arc<T>>) {
        self.values[self.val_index(index)].set(v);
    }

    fn val_index(&self, index: NonMaxU32) -> usize {
        index.get() as usize % NODE_SIZE
    }
}

impl<T> Default for OnceArrayNode<T> {
    fn default() -> Self {
        OnceArrayNode::<T> {
            values: make_static_array::<_, NODE_SIZE>(&|| AtomicArc::default()),
        }
    }
}

fn node_index(index: NonMaxU32) -> usize {
    index.get() as usize / NODE_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_indexes_are_sequential() {
        let arr = OnceArray::<usize>::default();
        assert_eq!(arr.reserve().unwrap(), 0.into());
        assert_eq!(arr.reserve().unwrap(), 1.into());
        assert_eq!(arr.reserve().unwrap(), 2.into());
    }

    #[test]
    fn can_set_reserved_index() {
        let arr = OnceArray::<usize>::default();
        let index = arr.reserve().unwrap();
        assert!(arr.set_if_none(index, 10.into()));
        assert_eq!(arr[index], 10);
    }

    #[test]
    fn try_set_returns_false_when_already_set() {
        let arr = OnceArray::<usize>::default();
        let index = arr.reserve().unwrap();
        arr.set_or_panic(index, 10.into());
        assert!(!arr.set_if_none(index, 11.into()));
    }

    #[test]
    #[should_panic]
    fn panic_on_double_set() {
        let arr = OnceArray::<usize>::default();
        let index = arr.reserve().unwrap();
        arr.set_or_panic(index, 10.into());
        arr.set_or_panic(index, 11.into());
    }

    #[test]
    #[should_panic]
    fn cannot_set_arbitrary_index() {
        let arr = OnceArray::<usize>::default();
        // Size this array hasn't had any reservations, there isn't a 0-index.
        arr.set_or_panic(0.into(), 10.into());
    }

    #[test]
    fn can_get_arbitrary_indexes() {
        let arr = OnceArray::<usize>::default();
        // Since this array hasn't had any reservations, there should be nothing in 0.
        assert_eq!(arr.get(0.into()), None);
    }

    #[test]
    fn can_get_valid_indexes() {
        let arr = OnceArray::<usize>::default();
        // We've reserved an index but not put anything in it. So there is nothing there.
        let index = arr.reserve().unwrap();
        assert_eq!(arr.get(index), None);
        // Now that the index has been set, there should be some value there.
        arr.set_or_panic(index, 10.into());
        assert_eq!(arr.get(index), Some(&10));
    }

    #[test]
    #[should_panic]
    fn index_panics_on_empty_index() {
        let arr = OnceArray::<usize>::default();
        let index = arr.reserve().unwrap();
        // Since the index hasn't been set, this should panic.
        let _ = arr[index];
    }
}
