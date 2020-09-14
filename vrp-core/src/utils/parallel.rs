pub use self::actual::map_reduce;
pub use self::actual::parallel_collect;
pub use self::actual::parallel_into_collect;
pub use self::actual::parallel_into_collect2;

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

    /// Maps collection and collects results into vector in parallel.
    pub fn parallel_into_collect<T, F, R>(source: Vec<T>, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        source.into_par_iter().map(map_op).collect()
    }

    /// Applies two different map operations on vector data.
    /// TODO is there better way to do this?
    pub fn parallel_into_collect2<T, F1, R1, F2, R2>(source: Vec<T>, map_op1: F1, map_op2: F2) -> Vec<R2>
    where
        T: Send + Sync,
        F1: Fn(T) -> R1 + Sync + Send,
        F2: Fn(R1) -> R2 + Sync + Send,
        R1: Send,
        R2: Send,
    {
        source.into_par_iter().map(map_op1).map(map_op2).collect()
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

    /// Map collections and collects results into vector synchronously.
    pub fn parallel_into_collect<T, F, R>(source: Vec<T>, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        source.into_iter().map(map_op).collect()
    }

    /// Applies two different map operations on vector data.
    pub fn parallel_into_collect2<T, F1, R1, F2, R2>(source: Vec<T>, map_op1: F1, map_op2: F2) -> Vec<R>
    where
        T: Send + Sync,
        F1: Fn(T) -> R1 + Sync + Send,
        F2: Fn(R1) -> R2 + Sync + Send,
        R1: Send,
        R2: Send,
    {
        source.into_iter().map(map_op1).map(map_op2).collect()
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
