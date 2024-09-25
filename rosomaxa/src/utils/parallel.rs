#[cfg(test)]
#[path = "../../tests/unit/utils/parallel_test.rs"]
mod parallel_test;

pub use self::actual::cartesian_product;
pub use self::actual::fold_reduce;
pub use self::actual::map_reduce;
pub use self::actual::parallel_collect;
pub use self::actual::parallel_foreach_mut;
pub use self::actual::parallel_into_collect;
pub use self::actual::ThreadPool;

#[cfg(not(target_arch = "wasm32"))]
mod actual {
    use rayon::prelude::*;
    use rayon::{ThreadPool as RayonThreadPool, ThreadPoolBuilder};

    /// Represents a thread pool wrapper.
    pub struct ThreadPool {
        inner: RayonThreadPool,
    }

    impl ThreadPool {
        /// Creates a new instance of `ThreadPool`
        pub fn new(num_threads: usize) -> Self {
            Self {
                inner: ThreadPoolBuilder::new().num_threads(num_threads).build().expect("cannot build a thread pool"),
            }
        }

        /// Executes given operation on thread pool.
        pub fn execute<OP, R>(&self, op: OP) -> R
        where
            OP: FnOnce() -> R + Send,
            R: Send,
        {
            self.inner.install(op)
        }
    }

    /// Creates a cartesian product returning a parallel iterator.
    pub fn cartesian_product<'a, A, B>(a: &'a [A], b: &'a [B]) -> impl IntoParallelIterator<Item = (&'a A, &'a B)>
    where
        A: Send + Sync + 'a,
        B: Send + Sync + 'a,
    {
        a.par_iter().flat_map(|a| b.par_iter().map(move |b| (a, b)))
    }

    /// Maps collection and collects results into vector in parallel.
    pub fn parallel_collect<T, S, FM, R>(source: S, map_op: FM) -> Vec<R>
    where
        T: Send + Sync,
        S: IntoParallelIterator<Item = T>,
        FM: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        source.into_par_iter().map(map_op).collect()
    }

    /// Maps collection and collects results into vector in parallel.
    pub fn parallel_into_collect<T, F, R>(source: Vec<T>, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        source.into_par_iter().map(map_op).collect()
    }

    /// Performs map reduce operations in parallel.
    pub fn map_reduce<'a, T, S, FM, FR, FD, R>(source: &'a S, map_op: FM, default_op: FD, reduce_op: FR) -> R
    where
        T: Send + Sync,
        S: IntoParallelRefIterator<'a, Item = T> + ?Sized,
        FM: Fn(T) -> R + Sync + Send,
        FR: Fn(R, R) -> R + Sync + Send,
        FD: Fn() -> R + Sync + Send,
        R: Send,
    {
        source.par_iter().map(map_op).reduce(default_op, reduce_op)
    }

    /// Performs fold and then reduce operations in parallel.
    pub fn fold_reduce<T, S, FI, FF, FR, R>(source: S, identity: FI, fold: FF, reduce: FR) -> R
    where
        T: Send + Sync,
        S: IntoParallelIterator<Item = T>,
        FI: Fn() -> R + Clone + Sync + Send,
        FF: Fn(R, T) -> R + Sync + Send,
        FR: Fn(R, R) -> R + Sync + Send,
        R: Send,
    {
        source.into_par_iter().fold(identity.clone(), fold).reduce(identity, reduce)
    }

    /// Performs mutable foreach in parallel.
    pub fn parallel_foreach_mut<T, F>(source: &mut [T], action: F)
    where
        T: Send + Sync,
        F: Fn(&mut T) + Send + Sync,
    {
        source.par_iter_mut().for_each(action)
    }
}

#[cfg(target_arch = "wasm32")]
mod actual {
    /// Represents a thread pool wrapper.
    pub struct ThreadPool;

    impl ThreadPool {
        /// Creates a new instance of `ThreadPool`.
        pub fn new(_num_threads: usize) -> Self {
            Self {}
        }

        /// Executes given operation on thread pool (dummy).
        pub fn execute<OP, R>(&self, op: OP) -> R
        where
            OP: FnOnce() -> R + Send,
            R: Send,
        {
            op()
        }
    }

    /// Creates a cartesian product returning an iterator.
    pub fn cartesian_product<'a, A, B>(a: &'a [A], b: &'a [B]) -> impl Iterator<Item = (&'a A, &'a B)>
    where
        A: Send + Sync + 'a,
        B: Send + Sync + 'a,
    {
        a.iter().flat_map(|a| b.iter().map(move |b| (a, b)))
    }

    /// Map collections and collects results into vector synchronously.
    pub fn parallel_collect<T, F, R>(source: &[T], map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(&T) -> R + Sync + Send,
        R: Send,
    {
        source.iter().map(map_op).collect()
    }

    /// Map collections and collects results into vector synchronously.
    pub fn parallel_into_collect<T, F, R>(source: Vec<T>, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        source.into_iter().map(map_op).collect()
    }

    /// Performs map and reduce operations synchronously.
    pub fn map_reduce<T, S, FM, FR, FD, R>(source: S, map_op: FM, default_op: FD, reduce_op: FR) -> R
    where
        T: Send + Sync,
        S: IntoIterator<Item = T>,
        FM: Fn(T) -> R + Sync + Send,
        FR: Fn(R, R) -> R + Sync + Send,
        FD: Fn() -> R + Sync + Send,
        R: Send,
    {
        source.into_iter().map(map_op).fold(default_op(), reduce_op)
    }

    /// Performs fold and then reduce operations.
    /// NOTE it behaves differently from parallel implementation.
    pub fn fold_reduce<T, S, FI, FF, FR, R>(source: S, identity: FI, fold: FF, mut reduce: FR) -> R
    where
        T: Send + Sync,
        S: IntoIterator<Item = T>,
        FI: Fn() -> R + Sync + Send,
        FF: FnMut(R, T) -> R + Sync + Send,
        FR: FnMut(R, R) -> R + Sync + Send,
        R: Send,
    {
        reduce(identity(), source.into_iter().fold(identity(), fold))
    }

    /// Performs mutable foreach in parallel.
    pub fn parallel_foreach_mut<T, F>(source: &mut [T], action: F)
    where
        T: Send + Sync,
        F: Fn(&mut T) + Send + Sync,
    {
        source.iter_mut().for_each(action)
    }
}
