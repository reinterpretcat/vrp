pub use self::actual::map_reduce;
pub use self::actual::parallel_collect;

#[cfg(not(target_arch = "wasm32"))]
mod actual {
    extern crate rayon;
    use rayon::prelude::*;

    /// Maps collection and collects results into vector in parallel.
    pub fn parallel_collect<T, F, R>(source: &[T], map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(&T) -> R + Sync + Send,
        R: Send,
    {
        source.par_iter().map(map_op).collect()
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
    /// Map collections and collects results into vector synchronously.
    pub fn parallel_collect<T, F, R>(source: &[T], map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(&T) -> R + Sync + Send,
        R: Send,
    {
        source.iter().map(map_op).collect()
    }

    /// Performs map reduce operations synchronously.
    pub fn map_reduce<T, FM, FR, FD, R>(source: &[T], map_op: FM, default_op: FD, reduce_op: FR) -> R
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
