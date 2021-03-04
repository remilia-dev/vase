// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::marker::PhantomPinned;
use std::mem::swap;
use std::ptr::{
    null_mut,
    NonNull,
};

use crate::{
    sync::{
        Arc,
        AtomicArc,
        AtomicBool,
        AtomicPtr,
        AtomicU8,
        Ordering,
    },
    util::mem::{
        alloc,
        free,
        make_static_array,
    },
};

/// Caches strings to reduce allocations and improve comparisons.
///
/// Atomic instructions are used internally to allow multiple threads
/// to use StringCache at the same time. The type is lock free and
/// never spins.
///
/// This struct uses a trie, also known as a prefix tree, internally.
#[derive(Debug)]
pub struct StringCache {
    root: TrieNodeLimited<128>,
}
impl StringCache {
    /// Creates a new empty StringCache.
    pub fn new() -> StringCache {
        StringCache { root: TrieNodeLimited::new_empty() }
    }

    /// If the string value given is in the cache, it will return the cached string.
    /// If the string value is not in the cache, it will create a new [CachedString]
    /// and add it to the cache.
    /// # Single-Result Guarantee
    /// This function guarantees that only one [CachedString] with a given string value will
    /// ever exist for this string cache. This allows comparisons between cached strings
    /// to be a simple pointer comparison.
    pub fn get_or_cache(&self, value: &str) -> CachedString {
        let mut cache_request = CacheRequest { chars: value, depth: 0 };
        // NOTE: The code below is a manual form of a tail-call (to prevent stack overflows).
        let mut chain = match self.root.get_or_cache_string(&mut cache_request) {
            Ok(result) => return result,
            Err(chain) => chain,
        };
        loop {
            chain = match chain.get_or_cache_string(&mut cache_request) {
                Ok(result) => return result,
                Err(chain) => chain,
            }
        }
    }
}
impl Default for StringCache {
    fn default() -> Self {
        StringCache::new()
    }
}

/// Represents a string value that has been cached in a [StringCache].
/// See [CachedStringData] for details about this type.
pub type CachedString = Arc<CachedStringData>;
/// CachedStringData stores the string that has been cached.
///
/// In the future, more data (such as a hash) may be stored as well.
/// # Comparison
/// CachedStringDatas are compared by their pointers. If they
/// are from separate string caches, they will never be considered
/// equal (even if they contain the same string).
pub struct CachedStringData {
    // OPTIMIZATION: We could store the str in the struct rather than needlessly boxing it.
    // It's not like CachedStringData should *ever* be on the stack.
    string: Box<str>,
    _pin: PhantomPinned,
}
impl CachedStringData {
    fn new(value: &str) -> Self {
        CachedStringData {
            string: Box::from(value),
            _pin: PhantomPinned,
        }
    }
    /// Gets the string this data represents.
    pub fn string(&self) -> &str {
        self.string.as_ref()
    }
    /// Gets the length of the string this data represents.
    pub fn len(&self) -> usize {
        self.string.len()
    }
    /// Returns whether this is an empty string or not.
    /// There should only be one empty string per cache.
    pub fn is_empty(&self) -> bool {
        self.string.is_empty()
    }
    /// Returns the pointer to the string as a usize.
    /// This can be used as a unique identification until the string is freed.
    pub fn uniq_id(&self) -> usize {
        self.string.as_ptr() as usize
    }
}
impl std::hash::Hash for CachedStringData {
    /// Much like equality, hashes are computed based on the pointer.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_usize(self as *const Self as usize);
    }
}
impl PartialEq for CachedStringData {
    /// The benefits of a StringCache are that comparisons between
    /// strings are simplified down to a pointer comparison.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for CachedStringData {}
impl std::fmt::Display for CachedStringData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}
impl std::fmt::Debug for CachedStringData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CachedString(at {:?}: {})",
            self as *const Self, self.string
        )
    }
}
impl AsRef<[u8]> for CachedStringData {
    fn as_ref(&self) -> &[u8] {
        self.string.as_bytes()
    }
}
impl AsRef<str> for CachedStringData {
    fn as_ref(&self) -> &str {
        &self.string
    }
}

#[derive(Copy, Clone)]
struct CacheRequest<'a> {
    depth: usize,
    chars: &'a str,
}
impl<'a> CacheRequest<'a> {
    /// Returns the string of the request as bytes.
    fn bytes(&self) -> &[u8] {
        self.chars.as_bytes()
    }
    /// Returns the bytes of the request string starting at the current depth.
    fn byte_tail(&self) -> &[u8] {
        &self.bytes()[self.depth..]
    }
    /// Returns the value of the current byte as a usize.
    fn byte_val(&self) -> usize {
        self.bytes()[self.depth] as usize
    }
    /// Returns the length (in bytes) of the requested string.
    fn len(&self) -> usize {
        self.chars.len()
    }
    /// Creates a new cached string for this request.
    fn new_cached(&self) -> CachedString {
        Arc::new(CachedStringData::new(self.chars))
    }
    /// Returns how far into the request one must go till a difference exists with a cached string.
    /// This starts from the current depth and assumes all previous bytes are the same.
    /// Returns None if the request is the same as the cached string.
    fn difference_from(&self, cached: &CachedString) -> Option<usize> {
        let b1 = &self.bytes()[self.depth..];
        let b2 = &cached.string.as_bytes()[self.depth..];

        let test_length = b1.len().min(b2.len());
        for i in 0..test_length {
            if b1[i] != b2[i] {
                return Some(i);
            }
        }

        if b1.len() == b2.len() {
            None
        } else {
            Some(test_length)
        }
    }
}
/// The node value that represents an unused slot.
const EMPTY_SLOT_VAL: u8 = 255;

#[repr(C)]
#[derive(Debug)]
struct TrieNodeLimited<const NODE_COUNT: usize> {
    size: u8,
    is_end_node: AtomicBool,
    children: [AtomicU8; NODE_COUNT],
    nodes: [TrieNodePtr; NODE_COUNT],
    chain: TrieNodePtr,
    end_value: AtomicArc<CachedStringData>,
    node_value: AtomicArc<CachedStringData>,
}
impl<const NODE_COUNT: usize> TrieNodeLimited<NODE_COUNT> {
    fn new_empty() -> Self {
        TrieNodeLimited {
            size: NODE_COUNT as u8,
            is_end_node: AtomicBool::new(false),
            end_value: AtomicArc::empty(),
            node_value: AtomicArc::empty(),
            children: make_static_array::<_, NODE_COUNT>(&|| AtomicU8::new(EMPTY_SLOT_VAL)),
            nodes: make_static_array::<_, NODE_COUNT>(&|| TrieNodePtr::new_null()),
            chain: TrieNodePtr::new_null(),
        }
    }

    fn new_end(value: CachedString, depth: usize) -> Self {
        let mut new_node = TrieNodeLimited::new_empty();
        if value.string.len() != depth {
            *new_node.is_end_node.get_mut() = true;
            new_node.end_value.set(value);
        } else {
            new_node.node_value.set(value);
        }
        new_node
    }

    fn new_mid(node_val: usize, node: TrieNodePtr) -> Self {
        let mut new_node = TrieNodeLimited::new_empty();
        let node_index = node_val % NODE_COUNT;
        *new_node.children[node_index].get_mut() = node_val as u8;
        new_node.nodes[node_index].set(node);
        new_node
    }

    fn move_or_get_end_value(&self, data: &CacheRequest) -> Option<CachedString> {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        if !self.is_end_node.load(Ordering::SeqCst) {
            return None;
        }
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        let end_value = self.end_value.load_arc(Ordering::SeqCst)?;
        let first_diff = match data.difference_from(&end_value) {
            // This was actually the value we're looking for
            None => return Some(end_value),
            Some(diff) => diff,
        };

        let target_spot = end_value.string.as_bytes()[data.depth] as usize;
        let reserved_spot = match self.find_or_reserve_node_index(target_spot) {
            // If there are no spots up for reservation, this node is no longer an end node.
            None => return None,
            Some(node_spot) => node_spot,
        };

        let mut chain_head = match first_diff {
            // In this case, the end value is being moved to a child node.
            0 => TrieNodePtr::new_end(end_value, data.depth + 1),
            // In this case, the end value will end up in a value-node.
            diff if data.depth + diff == end_value.len() => {
                TrieNodePtr::new_end(end_value, data.depth + first_diff)
            },
            // In this case, the end value is given its own node to differentiate from the requested value
            _ => {
                let diff_val = end_value.string().as_bytes()[data.depth + first_diff] as usize;
                let chain_diff = TrieNodePtr::new_end(end_value, data.depth + first_diff + 1);
                TrieNodePtr::new_mid(diff_val, chain_diff, data.depth + first_diff)
            },
        };

        for diff_pos in (1..first_diff).rev() {
            let node_val = data.byte_tail()[diff_pos] as usize;
            chain_head = TrieNodePtr::new_mid(node_val, chain_head, data.depth + diff_pos);
        }

        // If the node is not null, then this node is no longer an end node.
        // OPTIMIZATION: If the set succeeds, we could skip right to the end of the chain.
        // This optimization would apply to all but the first_diff = 0 case.
        let _ = self.nodes[reserved_spot].set_if_null(chain_head);
        // OPTIMIZATION: Could we use Ordering::Release here?
        self.is_end_node.store(false, Ordering::SeqCst);

        None
    }

    fn find_or_reserve_node_index(&self, start_val: usize) -> Option<usize> {
        // We start relative to start_val to hopefully make finding/reserving a slot quicker.
        let mut loop_index = start_val % NODE_COUNT;
        loop {
            let slot = &self.children[loop_index];
            // OPTIMIZATION: Could we use Ordering::Acquire here?
            let slot_val = slot.load(Ordering::SeqCst);
            if slot_val as usize == start_val {
                // There is a slot already reserved for this value.
                return Some(loop_index);
            } else if slot_val == EMPTY_SLOT_VAL {
                // OPTIMIZATION: Is a different ordering possible here?
                match slot.compare_exchange(
                    EMPTY_SLOT_VAL,
                    start_val as u8,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    // We've managed to reserve this slot for this value.
                    Ok(_) => return Some(loop_index),
                    Err(prev_val) => {
                        if prev_val == start_val as u8 {
                            // Another thread managed to reserve this slot for this same value after
                            // we loaded its value.
                            return Some(loop_index);
                        }
                    },
                }
                // At this point, the slot was taken by another thread for a different value.
            }

            loop_index = (loop_index + 1) % NODE_COUNT;
            // If our loop index equals the original start_val after an increment,
            // we've checked all of our reservation slots.
            if loop_index == start_val % NODE_COUNT {
                return None;
            }
        }
    }
}
impl<const NODE_COUNT: usize> TrieNode for TrieNodeLimited<NODE_COUNT> {
    fn get_or_cache_string(&self, data: &mut CacheRequest) -> Result<CachedString, &dyn TrieNode> {
        if data.len() == data.depth {
            let value = self.node_value.load_or_set_arc(&|| data.new_cached());
            Ok(value)
        } else if let Some(value) = self.move_or_get_end_value(data) {
            Ok(value)
        } else {
            self.find_next_node(data)
        }
    }

    fn find_next_node(&self, data: &mut CacheRequest) -> Result<CachedString, &dyn TrieNode> {
        if let Some(node_index) = self.find_or_reserve_node_index(data.byte_val()) {
            self.nodes[node_index].get_or_create_child(data)
        } else {
            self.chain.get_or_create_chain(data.depth).find_next_node(data)
        }
    }
}

trait TrieNode {
    fn get_or_cache_string(&self, data: &mut CacheRequest) -> Result<CachedString, &dyn TrieNode>;
    fn find_next_node(&self, data: &mut CacheRequest) -> Result<CachedString, &dyn TrieNode>;
}

struct TrieNodePtr {
    ptr: AtomicPtr<u8>,
}
impl TrieNodePtr {
    const fn new_null() -> Self {
        TrieNodePtr { ptr: AtomicPtr::new(null_mut()) }
    }

    fn new_empty(depth: usize) -> TrieNodePtr {
        let new_ptr = match depth {
            0..=1 => alloc(TrieNodeLimited::<128>::new_empty()).cast::<u8>(),
            2..=4 => alloc(TrieNodeLimited::<64>::new_empty()).cast::<u8>(),
            5..=8 => alloc(TrieNodeLimited::<32>::new_empty()).cast::<u8>(),
            9..=13 => alloc(TrieNodeLimited::<24>::new_empty()).cast::<u8>(),
            _ => alloc(TrieNodeLimited::<16>::new_empty()).cast::<u8>(),
        }
        .as_ptr();
        TrieNodePtr { ptr: AtomicPtr::new(new_ptr) }
    }

    fn new_end(end_value: CachedString, depth: usize) -> TrieNodePtr {
        let new_ptr = match depth {
            0..=1 => alloc(TrieNodeLimited::<128>::new_end(end_value, depth)).cast::<u8>(),
            2..=4 => alloc(TrieNodeLimited::<64>::new_end(end_value, depth)).cast::<u8>(),
            5..=8 => alloc(TrieNodeLimited::<32>::new_end(end_value, depth)).cast::<u8>(),
            9..=13 => alloc(TrieNodeLimited::<24>::new_end(end_value, depth)).cast::<u8>(),
            _ => alloc(TrieNodeLimited::<16>::new_end(end_value, depth)).cast::<u8>(),
        }
        .as_ptr();
        TrieNodePtr { ptr: AtomicPtr::new(new_ptr) }
    }

    fn new_mid(node_val: usize, node: TrieNodePtr, depth: usize) -> TrieNodePtr {
        let new_ptr = match depth {
            0..=1 => alloc(TrieNodeLimited::<128>::new_mid(node_val, node)).cast::<u8>(),
            2..=4 => alloc(TrieNodeLimited::<64>::new_mid(node_val, node)).cast::<u8>(),
            5..=8 => alloc(TrieNodeLimited::<32>::new_mid(node_val, node)).cast::<u8>(),
            9..=13 => alloc(TrieNodeLimited::<24>::new_mid(node_val, node)).cast::<u8>(),
            _ => alloc(TrieNodeLimited::<16>::new_mid(node_val, node)).cast::<u8>(),
        }
        .as_ptr();
        TrieNodePtr { ptr: AtomicPtr::new(new_ptr) }
    }

    fn set(&mut self, mut val: TrieNodePtr) {
        // We swap the pointers. If this TrieNodePtr had a node,
        // it will be freed by val going out of scope.
        swap(self.ptr.get_mut(), val.ptr.get_mut());
    }

    fn set_if_null(&self, mut val: TrieNodePtr) -> Result<*mut u8, *mut u8> {
        // OPTIMIZATION: Is a different ordering possible here?
        let result = self.ptr.compare_exchange(
            null_mut(),
            *val.ptr.get_mut(),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        if result.is_ok() {
            std::mem::forget(val);
        }
        result
    }

    fn get_or_create_child(&self, data: &mut CacheRequest) -> Result<CachedString, &dyn TrieNode> {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        let node_ptr = match NonNull::new(self.ptr.load(Ordering::SeqCst)) {
            Some(node_ptr) => node_ptr.as_ptr(),
            None => {
                let new_cached = data.new_cached();
                let new_node = TrieNodePtr::new_end(new_cached.clone(), data.depth + 1);
                match self.set_if_null(new_node) {
                    Ok(_) => return Ok(new_cached),
                    Err(ptr) => ptr,
                }
            },
        };

        data.depth += 1;
        Err(TrieNodePtr::get_trait(node_ptr))
    }

    fn get_or_create_chain(&self, depth: usize) -> &dyn TrieNode {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        let node_ptr = match NonNull::new(self.ptr.load(Ordering::SeqCst)) {
            Some(node_ptr) => node_ptr.as_ptr(),
            None => {
                // TODO: Maybe make a new_chain so that depth can be weighted differently.
                let mut new_node = TrieNodePtr::new_empty(depth);
                let new_ptr = *new_node.ptr.get_mut();
                match self.set_if_null(new_node) {
                    Ok(_) => new_ptr,
                    Err(ptr) => ptr,
                }
            },
        };

        TrieNodePtr::get_trait(node_ptr)
    }

    fn get_trait<'a>(ptr: *mut u8) -> &'a dyn TrieNode {
        let raw = match NonNull::new(ptr) {
            Some(raw) => raw,
            None => panic!("Can't get TrieNode trait from a null pointer."),
        };
        unsafe {
            // SAFETY: The pointer is not null and there should be at least a byte to read.
            match raw.as_ref() {
                // SAFETY: TrieNodeLimited's first field should match its type.
                16 => &*raw.cast::<TrieNodeLimited<16>>().as_ptr(),
                24 => &*raw.cast::<TrieNodeLimited<24>>().as_ptr(),
                32 => &*raw.cast::<TrieNodeLimited<32>>().as_ptr(),
                64 => &*raw.cast::<TrieNodeLimited<64>>().as_ptr(),
                128 => &*raw.cast::<TrieNodeLimited<128>>().as_ptr(),
                size => panic!("Unknown TrieNode size {}", size),
            }
        }
    }
}
impl Drop for TrieNodePtr {
    fn drop(&mut self) {
        unsafe {
            if let Some(raw) = NonNull::new(*self.ptr.get_mut()) {
                // SAFETY: The pointer is not null and there should be at least a byte to read.
                match raw.as_ref() {
                    // SAFETY: TrieNodeLimited's first field should match its type.
                    // SAFETY: This TrieNodePtr owns this pointer and can free it.
                    16 => free(raw.cast::<TrieNodeLimited<16>>()),
                    24 => free(raw.cast::<TrieNodeLimited<24>>()),
                    32 => free(raw.cast::<TrieNodeLimited<32>>()),
                    64 => free(raw.cast::<TrieNodeLimited<64>>()),
                    128 => free(raw.cast::<TrieNodeLimited<128>>()),
                    size => panic!("Unknown TrieNode size {}", size),
                }
            }
        }
    }
}
impl std::fmt::Debug for TrieNodePtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(raw) = NonNull::new(self.ptr.load(Ordering::SeqCst)) {
            unsafe {
                // SAFETY: The pointer is not null and there should be at least a byte to read.
                return match raw.as_ref() {
                    // SAFETY: TrieNodeLimited's first field should match its type.
                    // SAFETY: This TrieNodePtr owns this pointer and can free it.
                    16 => raw.cast::<TrieNodeLimited<16>>().as_ref().fmt(f),
                    24 => raw.cast::<TrieNodeLimited<24>>().as_ref().fmt(f),
                    32 => raw.cast::<TrieNodeLimited<32>>().as_ref().fmt(f),
                    64 => raw.cast::<TrieNodeLimited<64>>().as_ref().fmt(f),
                    128 => raw.cast::<TrieNodeLimited<128>>().as_ref().fmt(f),
                    size => panic!("Unknown TrieNode size {}", size),
                };
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_cache_returns_same_string() {
        let test_case = "Test";
        let cache = StringCache::new();
        let cached_value = cache.get_or_cache(test_case);
        assert_eq!(cached_value.string(), test_case);
    }

    #[test]
    fn string_cache_returns_cached_value() {
        let cache = StringCache::new();
        let cache1 = cache.get_or_cache("Test");
        let cache2 = cache.get_or_cache("Test");
        assert_eq!(cache1, cache2);
    }

    #[test]
    fn string_cache_does_not_confuse_values() {
        let cache = StringCache::new();
        let cache_author = cache.get_or_cache("author");
        let cache_a = cache.get_or_cache("a");
        assert_ne!(cache_author, cache_a);
    }

    #[test]
    fn string_cache_handles_unicode() {
        let cache = StringCache::new();
        let cached = cache.get_or_cache("🏳️‍🌈");
        assert_eq!(cached.string(), "🏳️‍🌈");
        let cached_again = cache.get_or_cache("🏳️‍🌈");
        assert_eq!(cached, cached_again);
    }

    #[test]
    fn string_cache_conflicts_are_solved() {
        let cache = StringCache::new();
        // The third node only has 64 slots, so 0 and p end up mapping to the same slot.
        // This conflict is handled by taking the nearest available slot (or adding a chain node).
        let cache_aa0 = cache.get_or_cache("AA0");
        let cache_aap = cache.get_or_cache("AAp");
        assert_ne!(cache_aa0, cache_aap);
    }

    #[test]
    fn string_cache_end_node_chain_works() {
        let cache = StringCache::new();
        // When an end node needs to be moved, it will pre-emptively generate as many nodes as it
        // needs in the chain.
        // This generates an 'f' end node with a value foobar.
        let cache_foobar1 = cache.get_or_cache("foobar");
        // This will cause a o->o->b->a->r chain to be generated.
        let cache_foobaz1 = cache.get_or_cache("foobaz");
        // To verify that the chain wasn't malformed, we get the same values from cache.
        let cache_foobar2 = cache.get_or_cache("foobar");
        let cache_foobaz2 = cache.get_or_cache("foobaz");
        assert_eq!(cache_foobar1, cache_foobar2);
        assert_eq!(cache_foobaz1, cache_foobaz2);
        assert_ne!(cache_foobar1, cache_foobaz1);
    }

    #[test]
    fn string_cache_end_node_chain_works_2() {
        let cache = StringCache::new();
        // When an end node needs to be moved, it will pre-emptively generate as many nodes as it
        // needs in the chain.

        // This generates an 'i' end node with a value foobar.
        let cache_if1 = cache.get_or_cache("if");
        // This generates a ->f value-node to store the if.
        // It will put int in a ->n end-node.
        let cache_int1 = cache.get_or_cache("int");
        // This will move int into a ->t value-node
        // Inline will be put in a ->l end-node.
        let cache_inline1 = cache.get_or_cache("inline");

        // To verify that the chains weren't malformed, we get the same values from cache.
        let cache_if2 = cache.get_or_cache("if");
        let cache_int2 = cache.get_or_cache("int");
        let cache_inline2 = cache.get_or_cache("inline");
        assert_eq!(cache_if1, cache_if2);
        assert_eq!(cache_int1, cache_int2);
        assert_eq!(cache_inline1, cache_inline2);
        assert_ne!(cache_if1, cache_inline1);
        assert_ne!(cache_inline1, cache_int1);
    }

    #[test]
    fn cached_strings_are_only_equal_as_pointers() {
        let cache1 = CachedString::new(CachedStringData::new("test"));
        let cache2 = CachedString::new(CachedStringData::new("test"));
        assert_ne!(cache1, cache2);
        assert_eq!(cache1, cache1.clone());
    }

    #[test]
    fn cached_strings_len_matches_string_len() {
        let cached_str = CachedStringData::new("test");
        assert_eq!(cached_str.len(), "test".len());
    }
}
