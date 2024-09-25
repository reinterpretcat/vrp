//! A tweaked version of bloom filter implementation from the `probabilistic-collections` crate.

use crate::utils::bit_vec::BitVec;
use crate::utils::hasher::{DefaultHasherBuilder, DoubleHasher};
use std::borrow::Borrow;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;

/// A space-efficient probabilistic data structure to test for membership in a set.
///
/// At its core, a bloom filter is a bit array, initially all set to zero. `K` hash functions
/// map each element to `K` bits in the bit array. An element definitely does not exist in the
/// bloom filter if any of the `K` bits are unset. An element is possibly in the set if all of the
/// `K` bits are set. This particular implementation of a bloom filter uses two hash functions to
/// simulate `K` hash functions. Additionally, it operates on only one "slice" in order to have
/// predictable memory usage.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BloomFilter<T, B = DefaultHasherBuilder> {
    bit_vec: BitVec,
    hasher: DoubleHasher<T, B>,
    hasher_count: usize,
    _marker: PhantomData<T>,
}

impl<T> BloomFilter<T> {
    /// Constructs a new, empty `BloomFilter` with an estimated max capacity of `item_count` items,
    /// and a maximum false positive probability of `fpp`.
    #[allow(dead_code)]
    pub fn new(item_count: usize, fpp: f64) -> Self {
        Self::with_hashers(
            item_count,
            fpp,
            [DefaultHasherBuilder::from_entropy(), DefaultHasherBuilder::from_entropy()],
        )
    }

    /// Constructs a new, empty `BloomFilter` with an estimated max capacity of `item_count` items,
    /// and a maximum false positive probability of `fpp` using provided two seeds.
    pub fn new_with_seed(item_count: usize, fpp: f64, seed: (u64, u64)) -> Self {
        Self::with_hashers(
            item_count,
            fpp,
            [DefaultHasherBuilder::from_seed(seed.0), DefaultHasherBuilder::from_seed(seed.1)],
        )
    }

    pub fn union(&mut self, other: &Self) {
        self.bit_vec.union(&other.bit_vec);
    }
}

impl<T, B> BloomFilter<T, B>
where
    B: BuildHasher,
{
    /// Constructs a new, empty `BloomFilter` with an estimated max capacity of `item_count` items,
    /// a maximum false positive probability of `fpp`, and two hasher builders for double hashing.
    pub fn with_hashers(item_count: usize, fpp: f64, hash_builders: [B; 2]) -> Self {
        let bit_count = (-fpp.log2() * (item_count as f64) / 2f64.ln()).ceil() as usize;
        BloomFilter {
            bit_vec: BitVec::new(bit_count),
            hasher: DoubleHasher::with_hashers(hash_builders),
            hasher_count: Self::get_hasher_count(bit_count, item_count),
            _marker: PhantomData,
        }
    }

    /// Inserts an element into the bloom filter.
    pub fn insert<U>(&mut self, item: &U)
    where
        T: Borrow<U>,
        U: Hash + ?Sized,
    {
        self.hasher.hash(item).take(self.hasher_count).for_each(|hash| {
            let offset = hash % self.bit_vec.len() as u64;
            self.bit_vec.set(offset as usize, true);
        })
    }

    fn get_hasher_count(bit_count: usize, item_count: usize) -> usize {
        ((bit_count as f64) / (item_count as f64) * 2f64.ln()).ceil() as usize
    }

    /// Checks if an element is possibly in the bloom filter.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        T: Borrow<U>,
        U: Hash + ?Sized,
    {
        self.hasher.hash(item).take(self.hasher_count).all(|hash| {
            let offset = hash % self.bit_vec.len() as u64;
            self.bit_vec[offset as usize]
        })
    }
}
