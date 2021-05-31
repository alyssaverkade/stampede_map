#![allow(non_upper_case_globals)]
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;
use std::mem;
use std::sync::atomic::AtomicUsize;

mod bitmask;

pub use bitmask::BitMask;

use ahash::CallHasher;
use lazy_static::lazy_static;

#[derive(Copy, Clone, Debug)]
struct Node<V> {
    hash: u64,
    value: V,
}

#[derive(Clone, Debug)]
enum Slot<V>
where
    V: Clone,
{
    Empty,
    Occupied(Node<V>),
}

#[inline(always)]
const fn ctrl_hash(hash: u64) -> u8 {
    let val = hash & 0x7F;
    val as u8
}

const Deleted: u8 = 0b1000_0000;
const Empty: u8 = 0b1111_1110;

#[derive(Debug)]
pub struct StampedeMap<K, V, S = ahash::RandomState>
where
    V: Clone,
{
    data: Vec<Slot<V>>,
    // extra_data:
    counter: AtomicUsize,
    len: usize,
    capacity: usize,
    ctrl: Vec<u8>,
    // keep track of tombstone count because they contribute to load factor
    deleted: usize,
    _phantom: PhantomData<(K, S)>,
}

#[inline(always)]
const fn bucket_size() -> usize {
    16
}

impl<K, V, S> StampedeMap<K, V, S>
where
    K: Hash + Sized + CallHasher,
    V: Clone + std::fmt::Debug,
    S: BuildHasher + Default,
{
    pub fn new() -> Self {
        Self {
            data: vec![Slot::Empty; bucket_size()],
            counter: AtomicUsize::new(0),
            ctrl: vec![Empty; bucket_size() * 2], // extra group for bookkeeping
            capacity: bucket_size(),
            deleted: 0,
            _phantom: PhantomData,
            len: 0,
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        let mut map = Self::new();
        let cap = cap.next_power_of_two();
        map.capacity = cap;
        map.data.resize(map.capacity, Slot::Empty);
        map.ctrl.resize(map.capacity + 16, Empty);
        map
    }

    pub fn clear(&mut self) {
        let mut vec = vec![Slot::Empty; self.capacity];
        let mut ctrl = vec![Empty; self.capacity + 16];
        mem::swap(&mut self.data, &mut vec);
        mem::swap(&mut self.ctrl, &mut ctrl);
        self.deleted = 0;
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline(never)]
    pub fn get(&self, key: K) -> Option<&V> {
        let hash = self.hash(&key);
        let ctrl = ctrl_hash(hash);
        let mut slot = self.modulo(hash);
        loop {
            // SAFETY: the `modulo` method ensures we cannot access out of bounds + the `set`
            // method ensures that reads from len - 15..max_capacity cannot read over the end of
            // the buffer
            let buffer = unsafe { self.ctrl.get_unchecked(slot..slot + 16) };
            let empty_mask = BitMask::matches(buffer, Empty);
            let ctrl_mask = BitMask::matches(buffer, ctrl);
            for item in ctrl_mask | empty_mask {
                let offset = self.modulo((slot + item as usize) as u64);
                // SAFETY: the `modulo` method ensures we cannot perform an out of bounds read
                match unsafe { *self.ctrl.get_unchecked(offset) } {
                    // SAFETY: the `modulo` method ensures we cannot perform an out of bounds read
                    val if val == ctrl => match unsafe { self.data.get_unchecked(offset) } {
                        // the ctrl byte should be set to Empty
                        Slot::Empty => unreachable!(),
                        Slot::Occupied(ref node) if node.hash == hash => return Some(&node.value),
                        // probe chain must continue
                        _ => (),
                    },
                    Empty => return None,
                    _ => (),
                }
            }
            slot = self.modulo(slot as u64 + 16);
        }
    }

    #[inline(always)]
    fn exceeded_load_factor(&self) -> bool {
        self.capacity * 3 < (self.len + self.deleted) * 4
    }

    #[inline(always)]
    pub fn set(&mut self, key: K, value: V) {
        if self.exceeded_load_factor() {
            self.resize();
        }
        let hash = self.hash(&key);
        let mut idx = self.modulo(hash);
        loop {
            match self.data[idx] {
                Slot::Occupied(ref slot) if slot.hash != hash => idx = self.modulo(idx as u64 + 1),
                _ => break,
            }
        }
        let ctrl = ctrl_hash(hash);
        // bookkeeping so that memcpy can acquire contiguous values
        if (0..16).contains(&idx) {
            self.ctrl[self.capacity + idx] = ctrl;
        }
        self.ctrl[idx] = ctrl;
        self.len += 1;
        self.data[idx] = Slot::Occupied(Node { hash, value });
    }

    pub fn delete(&mut self, key: K) {
        let hash = self.hash(&key);
        let mut idx = self.modulo(hash);
        loop {
            match &self.data[idx] {
                Slot::Occupied(ref node) if node.hash == hash => break,
                Slot::Empty => return,
                _ => idx = self.modulo(idx as u64 + 1),
            }
        }
        if (0..16).contains(&idx) {
            self.ctrl[self.capacity + idx] = Deleted;
        }
        self.ctrl[idx] = Deleted;
        self.data[idx] = Slot::Empty;
        self.deleted += 1;
        self.len -= 1;
    }

    #[inline(always)]
    fn resize(&mut self) {
        let mut old = Vec::with_capacity(self.capacity);
        self.capacity = (self.capacity() + 1).next_power_of_two();
        self.deleted = 0;
        mem::swap(&mut old, &mut self.data);
        self.ctrl.clear();
        self.ctrl.resize(self.capacity + 16, Empty);
        self.data.resize(self.capacity, Slot::Empty);
        for slot in old {
            // we don't need to preserve deleted values and empty is a no-op
            if let Slot::Occupied(node) = &slot {
                let mut idx = self.modulo(node.hash);
                // find the next place to insert
                loop {
                    match self.data[idx] {
                        Slot::Empty => break,
                        // duplicate hashes are impossible in a bijective map
                        _ => idx = self.modulo(idx as u64 + 1),
                    }
                }
                let ctrl = ctrl_hash(node.hash);
                if (0..16).contains(&idx) {
                    self.ctrl[self.capacity + idx] = ctrl;
                }
                self.ctrl[idx] = ctrl;
                self.data[idx] = slot;
            }
        }
    }

    #[inline(always)]
    fn hash(&self, key: &K) -> u64 {
        lazy_static! {
            static ref HASHER: ahash::RandomState = ahash::RandomState::new();
        }
        // static hash_builder: ahash::RandomState = ahash::RandomState::new();
        // let mut hasher = S::default().build_hasher();
        // key.hash(&mut hasher);
        // hasher.finish()
        K::get_hash(key, &*HASHER)
    }

    #[inline(always)]
    fn modulo(&self, offset: u64) -> usize {
        (offset & ((self.capacity - 1) as u64)) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[global_allocator]
    static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

    #[test]
    fn basic_set_and_get() {
        let mut map: StampedeMap<_, _> = StampedeMap::new();
        map.set(0, 1);
        assert_eq!(map.get(0), Some(&1));
        map.set(1, 10);
        map.set(2, 9);
        map.set(3, 8);
        map.set(4, 7);
        map.set(5, 6);
        map.set(6, 5);
        map.set(7, 4);
        map.set(8, 3);
        map.set(9, 2);
        map.set(10, 0);
        assert_eq!(map.get(10), Some(&0));
        assert_eq!(map.get(9), Some(&2));
        assert_eq!(map.get(1), Some(&10));
        assert_eq!(map.get(0), Some(&1));
        assert_eq!(map.get(2), Some(&9));
        assert_eq!(map.get(3), Some(&8));
        assert_eq!(map.get(4), Some(&7));
        assert_eq!(map.get(5), Some(&6));
        assert_eq!(map.get(6), Some(&5));
        assert_eq!(map.get(7), Some(&4));
        assert_eq!(map.get(8), Some(&3));
        assert_eq!(map.get(9), Some(&2));
    }

    #[test]
    fn regressions() {
        let mut map: StampedeMap<usize, usize> = StampedeMap::new();
        let mut input = vec![(0, 0), (882041908, 0), (201832565, 0)];
        for (k, v) in input.iter().copied() {
            map.set(k, v);
            assert_eq!(map.get(k), Some(&v));
            map.delete(k);
            assert_ne!(map.get(k), Some(&v));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { result_cache: proptest::test_runner::basic_result_cache, cases: 16, ..Default::default() })]
        #[test]
        fn prop_sets_and_deletes_always_work(v in prop::collection::vec((0usize..1 << 32, 0usize..1 << 32), 10..1_000_000)) {
            let mut map: StampedeMap<usize, usize> = StampedeMap::new();
            for (key, value) in v.iter().copied() {
                map.set(key, value);
                assert_eq!(map.get(key), Some(&value));
                map.delete(key);
                let hash = map.hash(&key);
                let ctrl = ctrl_hash(hash);
                assert_ne!(map.get(key), Some(&value));
            }
        }
    }
}
