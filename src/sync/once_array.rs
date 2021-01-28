use crate::{
    sync::{
        AtomicBox,
        AtomicUsize,
        Ordering,
    },
    util::mem::make_static_array,
};

// NOTE: This limits us to 2048 * 50 (102400) indices.
/// The number of elements a single OnceArrayNode can hold.
const NODE_SIZE: usize = 2048;
/// The number of OnceArrayNodes a OnceArray can hold.
const NODE_COUNT: usize = 50;

/// A grow-only array that can initialize indexes individually.
///
/// This array uses [AtomicBox]s to initialize individual elements.
/// A thread can reserve and index and fill it later (or have another thread
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
    accum: AtomicUsize,
}

impl<T> OnceArray<T> {
    pub fn new() -> Self {
        OnceArray {
            nodes: make_static_array::<_, NODE_COUNT>(&|| AtomicBox::empty()),
            accum: AtomicUsize::new(0),
        }
    }

    /// Reserves an index to be set later. This index is guaranteed to be unique.
    pub fn reserve(&self) -> u32 {
        // NOTE: We use a u32 because the maximum number of indices can fit within u32.
        // Also, we know outside code will use u32 more than usize.
        self.accum.fetch_add(1, Ordering::SeqCst) as u32
    }

    /// Tries to get the value at a specific index. If that index has not been initialized,
    /// it will return None.
    pub fn get(&self, index: u32) -> Option<&T> {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        if let Some(node) = self.nodes[node_index(index)].load(Ordering::SeqCst) {
            return node.get(index);
        }

        None
    }

    /// Sets the value at the given index. This assumes the index has not been previously set.
    /// # Panics
    /// Panics if this index has already been set.
    pub fn set(&self, index: u32, val: T) {
        if !self.try_set(index, val) {
            panic!("Cannot set a value in a once-array that has already been initialized.");
        }
    }

    /// Tries to set the value at the given index. If that value has been already set,
    /// the value is discarded. Returns whether the value was set or not.
    pub fn try_set(&self, index: u32, val: T) -> bool {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        if self.accum.load(Ordering::SeqCst) <= index as usize {
            panic!("Cannot set a value in a non-reserved index.")
        }
        return self.ensure_node_for_index(index).try_set(index, val);
    }

    /// Adds a value onto the array and returns the index the value is at.
    pub fn push(&self, val: T) -> u32 {
        let index = self.reserve();
        self.set(index, val);
        index
    }

    /// Ensures that the node exists. It will then return either the existing node or
    /// the newly created node.
    fn ensure_node_for_index(&self, index: u32) -> &OnceArrayNode<T> {
        let node_slot = &self.nodes[node_index(index)];
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        if let Some(node) = node_slot.load(Ordering::SeqCst) {
            return node;
        }

        // OPTIMIZATION: Can different orderings work here?
        node_slot.set_if_none(
            Box::new(OnceArrayNode::default()),
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
    }
}
impl<T> std::ops::Index<u32> for OnceArray<T> {
    type Output = T;

    /// Gets the value at the given index.
    /// # Panics
    /// Panics if the index is out of bounds or if that index has not been set.
    fn index(&self, index: u32) -> &T {
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
    values: [AtomicBox<T>; NODE_SIZE],
}
impl<T> OnceArrayNode<T> {
    fn get(&self, index: u32) -> Option<&T> {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        self.values[self.val_index(index)].load(Ordering::SeqCst)
    }

    fn try_set(&self, index: u32, val: T) -> bool {
        // OPTIMIZATION: Can different orderings work here?
        let new_val = self.values[self.val_index(index)].try_set_if_null(
            Box::new(val),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        return matches!(new_val, Ok(_));
    }

    fn val_index(&self, index: u32) -> usize {
        index as usize % NODE_SIZE
    }
}
impl<T> Default for OnceArrayNode<T> {
    fn default() -> Self {
        OnceArrayNode::<T> {
            values: make_static_array::<_, NODE_SIZE>(&|| AtomicBox::empty()),
        }
    }
}

fn node_index(index: u32) -> usize {
    index as usize / NODE_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_indexes_are_sequential() {
        let arr = OnceArray::<usize>::default();
        assert_eq!(arr.reserve(), 0);
        assert_eq!(arr.reserve(), 1);
        assert_eq!(arr.reserve(), 2);
    }

    #[test]
    fn can_set_reserved_index() {
        let arr = OnceArray::<usize>::default();
        let index = arr.reserve();
        assert!(arr.try_set(index, 10));
        assert_eq!(arr[index], 10);
    }

    #[test]
    fn try_set_returns_false_when_already_set() {
        let arr = OnceArray::<usize>::default();
        let index = arr.reserve();
        arr.set(index, 10);
        assert!(!arr.try_set(index, 11));
    }

    #[test]
    #[should_panic]
    fn panic_on_double_set() {
        let arr = OnceArray::<usize>::default();
        let index = arr.reserve();
        arr.set(index, 10);
        arr.set(index, 11);
    }

    #[test]
    #[should_panic]
    fn cannot_set_arbitrary_index() {
        let arr = OnceArray::<usize>::default();
        // Size this array hasn't had any reservations, there isn't a 0-index.
        arr.set(0, 10);
    }

    #[test]
    fn can_get_arbitrary_indexes() {
        let arr = OnceArray::<usize>::default();
        // Since this array hasn't had any reservations, there should be nothing in 0.
        assert_eq!(arr.get(0), None);
    }

    #[test]
    fn can_get_valid_indexes() {
        let arr = OnceArray::<usize>::default();
        // We've reserved an index but not put anything in it. So there is nothing there.
        let index = arr.reserve();
        assert_eq!(arr.get(index), None);
        // Now that the index has been set, there should be some value there.
        arr.set(index, 10);
        assert_eq!(arr.get(index), Some(&10));
    }

    #[test]
    #[should_panic]
    fn index_panics_on_empty_index() {
        let arr = OnceArray::<usize>::default();
        let index = arr.reserve();
        // Since the index hasn't been set, this should panic.
        let _ = arr[index];
    }
}
