pub use self::actual::map_reduce;
pub use self::actual::parallel_collect;
pub use self::actual::parallel_into_collect;
pub use self::actual::ThreadPool;

#[cfg(not(target_arch = "wasm32"))]
mod actual {
    extern crate rayon;
    use self::rayon::{ThreadPool as RayonThreadPool, ThreadPoolBuilder};
    use rayon::prelude::*;

    /// Represents a thread pool.
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

    /// Maps collection and collects results into vector in parallel.
    pub fn parallel_collect<T, F, R>(source: &[T], map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(&T) -> R + Sync + Send,
        R: Send,
    {
        source.par_iter().map(map_op).collect()
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
    pub fn map_reduce<T, FM, FR, FD, R>(source: &[T], map_op: FM, default_op: FD, reduce_op: FR) -> R
    where
        T: Send + Sync,
        FM: Fn(&T) -> R + Sync + Send,
        FR: Fn(R, R) -> R + Sync + Send,
        FD: Fn() -> R + Sync + Send,
        R: Send,
    {
        source.par_iter().map(map_op).reduce(default_op, reduce_op)
    }
}

#[cfg(target_arch = "wasm32")]
mod actual {
    use crate::utils::ParallelismDegree;

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

    /// Map collections and collects results into vector synchronously.
    pub fn parallel_collect<T, F, R>(source: &[T], _degree: ParallelismDegree, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(&T) -> R + Sync + Send,
        R: Send,
    {
        source.iter().map(map_op).collect()
    }

    /// Map collections and collects results into vector synchronously.
    pub fn parallel_into_collect<T, F, R>(source: Vec<T>, _degree: ParallelismDegree, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        source.into_iter().map(map_op).collect()
    }

    /// Performs map reduce operations synchronously.
    pub fn map_reduce<T, FM, FR, FD, R>(
        source: &[T],
        _degree: ParallelismDegree,
        map_op: FM,
        default_op: FD,
        reduce_op: FR,
    ) -> R
    where
        T: Send + Sync,
        FM: Fn(&T) -> R + Sync + Send,
        FR: Fn(R, R) -> R + Sync + Send,
        FD: Fn() -> R + Sync + Send,
        R: Send,
    {
        source.iter().map(map_op).fold(default_op(), reduce_op)
    }
}
