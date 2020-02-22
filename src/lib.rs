#![allow(unused, non_upper_case_globals)]
use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

use wyhash::WyHash;

const AtomicWrite: Ordering = Ordering::Release;
const AtomicRead: Ordering = Ordering::Acquire;

#[derive(Copy, Clone, Debug)]
struct Node<V> {
    hash: u64,
    value: V,
}

#[derive(Clone, Debug)]
enum Slot<V: Clone> {
    Empty,
    Deleted,
    Occupied(Node<V>),
}

#[derive(Debug)]
pub struct StampedeMap<K: Hash, V: Clone, S = BuildHasherDefault<WyHash>>
where
    S: BuildHasher,
{
    data: Vec<Slot<V>>,
    counter: AtomicUsize,
    len: usize,
    capacity: usize,
    pow: u8,
    _phantom: PhantomData<(K, S)>,
}

impl<K: Hash, V: Clone + std::fmt::Debug, S: BuildHasher + Default> StampedeMap<K, V, S> {
    pub fn new() -> Self {
        Self {
            data: vec![Slot::Empty; 8],
            counter: AtomicUsize::new(0),
            pow: 3,
            capacity: 8,
            _phantom: PhantomData,
            len: 0,
        }
    }

    #[inline(always)]
    pub fn get(&self, key: K) -> Option<&V> {
        let hash = self.hash(&key);
        let mut slot = self.modulo(hash);
        loop {
            match self.data[slot] {
                Slot::Empty => return None,
                Slot::Occupied(ref node) if node.hash == hash => return Some(&node.value),
                Slot::Occupied(_) | Slot::Deleted => slot = self.modulo(slot as u64 + 1),
            }
        }
    }

    #[inline(always)]
    fn exceeded_load_factor(&self) -> bool {
        self.capacity * 3 < self.len * 4
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
                Slot::Empty | Slot::Deleted => break,
                // I'd love to make this have the same match arm as the line above
                Slot::Occupied(ref slot) if slot.hash == hash => break,
                _ => idx = self.modulo(idx as u64 + 1),
            }
        }
        self.len += 1;
        self.data[idx] = Slot::Occupied(Node { hash, value });
    }

    pub fn delete(&mut self, key: K) {
        let hash = self.hash(&key);
        let mut idx = self.modulo(hash);
        loop {
            match self.data[idx] {
                Slot::Occupied(ref node) if node.hash == hash => break,
                Slot::Empty => return,
                _ => idx = self.modulo(idx as u64 + 1),
            }
        }
        self.data[idx] = Slot::Deleted;
        self.len -= 1;
    }

    #[inline(always)]
    fn resize(&mut self) {
        while 2usize.pow(self.pow.into()) < 3 * self.len {
            self.pow += 1;
        }
        // recalculate new cap
        self.capacity = 2usize.pow(self.pow.into());
        let mut old = self.data.clone();
        self.data.clear();
        self.data.resize(self.capacity, Slot::Empty);
        for slot in old.iter_mut() {
            match slot {
                Slot::Occupied(node) => {
                    let mut idx = self.modulo(node.hash);
                    // find the next place to insert
                    loop {
                        match self.data[idx] {
                            Slot::Empty | Slot::Deleted => break,
                            // duplicate hashes are impossible in a bijective map
                            _ => idx = self.modulo(idx as u64 + 1),
                        }
                    }
                    self.data[idx] = std::mem::replace(slot, Slot::Empty);
                }
                // we don't need to preserve deleted values and empty is a no-op
                _ => (),
            }
        }
    }

    #[inline(always)]
    fn hash(&self, key: &K) -> u64 {
        let mut hasher = S::default().build_hasher();
        key.hash(&mut hasher);
        hasher.finish()
    }

    #[inline(always)]
    fn modulo(&self, offset: u64) -> usize {
        (offset & ((self.capacity - 1) as u64)) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
