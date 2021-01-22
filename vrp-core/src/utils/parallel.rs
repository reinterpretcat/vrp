pub use self::actual::map_reduce;
pub use self::actual::parallel_collect;
pub use self::actual::parallel_into_collect;

#[cfg(not(target_arch = "wasm32"))]
mod actual {
    extern crate rayon;
    use crate::utils::ParallelismDegree;
    use rayon::prelude::*;

    /// Maps collection and collects results into vector in parallel.
    pub fn parallel_collect<T, F, R>(source: &[T], degree: ParallelismDegree, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(&T) -> R + Sync + Send,
        R: Send,
    {
        if let Some(min_len) = get_min_len(source.len(), degree) {
            source.par_iter().with_min_len(min_len).map(map_op).collect()
        } else {
            source.par_iter().map(map_op).collect()
        }
    }

    /// Maps collection and collects results into vector in parallel.
    pub fn parallel_into_collect<T, F, R>(source: Vec<T>, degree: ParallelismDegree, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        if let Some(min_len) = get_min_len(source.len(), degree) {
            source.into_par_iter().with_min_len(min_len).map(map_op).collect()
        } else {
            source.into_par_iter().map(map_op).collect()
        }
    }

    /// Performs map reduce operations in parallel.
    pub fn map_reduce<T, FM, FR, FD, R>(
        source: &[T],
        degree: ParallelismDegree,
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
        if let Some(min_len) = get_min_len(source.len(), degree) {
            source.par_iter().with_min_len(min_len).map(map_op).reduce(default_op, reduce_op)
        } else {
            source.par_iter().map(map_op).reduce(default_op, reduce_op)
        }
    }

    fn get_min_len(items: usize, degree: ParallelismDegree) -> Option<usize> {
        let degree = match degree {
            ParallelismDegree::Full => return None,
            ParallelismDegree::Limited { max } => max,
        };

        Some((items as f64 / degree as f64).ceil() as usize)
    }
}

#[cfg(target_arch = "wasm32")]
mod actual {
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
