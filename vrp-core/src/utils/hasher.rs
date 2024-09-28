use rand::random;
use std::borrow::Borrow;
use std::fmt;
use std::hash::{BuildHasher, DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

#[derive(Clone)]
pub(crate) struct DefaultHasherBuilder {
    seed: u64,
    hasher: DefaultHasher,
}

impl DefaultHasherBuilder {
    /// Constructs a new `SipHasherBuilder` that uses the thread-local RNG to seed itself.
    pub fn from_entropy() -> Self {
        Self::from_seed(random())
    }

    /// Constructs a new `DefaultHasherBuilder` that is seeded with the given keys.
    pub fn from_seed(seed: u64) -> Self {
        let mut hasher = DefaultHasher::new();

        // NOTE although DefaultHasher is actually SipHasher, it doesn't expose keys
        hasher.write_u64(seed);

        DefaultHasherBuilder { seed, hasher }
    }
}

impl fmt::Debug for DefaultHasherBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultHasherBuilder").field("seed", &self.seed).finish()
    }
}

impl PartialEq for DefaultHasherBuilder {
    fn eq(&self, other: &DefaultHasherBuilder) -> bool {
        self.seed == other.seed
    }
}

impl BuildHasher for DefaultHasherBuilder {
    type Hasher = DefaultHasher;

    #[inline]
    fn build_hasher(&self) -> DefaultHasher {
        self.hasher.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DoubleHasher<T, B = DefaultHasherBuilder> {
    hash_builders: [B; 2],
    _marker: PhantomData<T>,
}

impl<T, B> DoubleHasher<T, B>
where
    B: BuildHasher,
{
    pub fn with_hashers(hash_builders: [B; 2]) -> Self {
        DoubleHasher { hash_builders, _marker: PhantomData }
    }

    pub fn hash<U>(&self, item: &U) -> HashIter
    where
        T: Borrow<U>,
        U: Hash + ?Sized,
    {
        HashIter { a: self.hash_builders[0].hash_one(item), b: self.hash_builders[1].hash_one(item), c: 0 }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct HashIter {
    a: u64,
    b: u64,
    c: u64,
}

impl Iterator for HashIter {
    type Item = u64;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.a;
        self.a = self.a.wrapping_add(self.b);
        self.b = self.b.wrapping_add(self.c);
        self.c += 1;
        Some(ret)
    }
}
