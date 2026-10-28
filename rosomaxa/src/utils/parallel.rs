#[cfg(test)]
#[path = "../../tests/unit/utils/parallel_test.rs"]
mod parallel_test;

pub use self::actual::cartesian_product;
pub use self::actual::fold_reduce;
pub use self::actual::map_reduce;
pub use self::actual::parallel_collect;
pub use self::actual::parallel_foreach_mut;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use self::actual::get_current_parallelism;

/// Specifies how an indexed iterator should be executed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ParallelismPolicy {
    /// Uses Rayon's default splitting strategy.
    #[default]
    Default,
    /// Runs independent operations and limits the task trees created by their nested parallel work.
    Coarse,
    /// Adapts the minimum chunk size to the active pool and enclosing coarse operations.
    Adaptive,
}

#[cfg(not(target_arch = "wasm32"))]
mod actual {
    use super::ParallelismPolicy;
    use rayon::prelude::*;
    use std::cell::Cell;
    use std::sync::OnceLock;

    thread_local! {
        static COARSE_PARALLELISM: Cell<usize> = const { Cell::new(1) };
    }

    static PHYSICAL_CPUS: OnceLock<usize> = OnceLock::new();

    // Extra tasks let rayon balance insertion candidates which take different time to evaluate.
    // Eight per worker is still small enough to avoid a large task tree for every concurrent search.
    const ADAPTIVE_TASKS_PER_WORKER: usize = 8;

    struct CoarseParallelismGuard(usize);

    impl CoarseParallelismGuard {
        fn set(value: usize) -> Self {
            Self(COARSE_PARALLELISM.replace(value))
        }
    }

    impl Drop for CoarseParallelismGuard {
        fn drop(&mut self) {
            COARSE_PARALLELISM.set(self.0);
        }
    }

    /// Creates a cartesian product returning an adaptively split parallel iterator.
    pub fn cartesian_product<'a, A, B>(a: &'a [A], b: &'a [B]) -> impl IndexedParallelIterator<Item = (&'a A, &'a B)>
    where
        A: Send + Sync + 'a,
        B: Send + Sync + 'a,
    {
        let b_len = b.len();
        let product_len = a.len().checked_mul(b_len).expect("cartesian product size overflow");
        let min_len = get_fine_grained_min_len(product_len);

        // A single indexed range lets rayon split the complete product directly. Nested parallel
        // iterators create a task tree per item in `a`, which adds work-stealing overhead when the
        // caller itself is already running multiple searches in parallel.
        (0..product_len).into_par_iter().with_min_len(min_len.max(1)).map(move |idx| (&a[idx / b_len], &b[idx % b_len]))
    }

    /// Maps an indexed collection and collects results in parallel using the given policy.
    pub fn parallel_collect<T, S, FM, R>(source: S, policy: ParallelismPolicy, map_op: FM) -> Vec<R>
    where
        T: Send + Sync,
        S: IntoParallelIterator<Item = T>,
        S::Iter: IndexedParallelIterator,
        FM: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        let source = source.into_par_iter();
        let source_len = source.len();

        match policy {
            ParallelismPolicy::Default => source.map(map_op).collect(),
            ParallelismPolicy::Coarse => {
                let parallelism = COARSE_PARALLELISM.get().saturating_mul(source_len).max(1);

                source
                    .map(|item| {
                        let _guard = CoarseParallelismGuard::set(parallelism);
                        map_op(item)
                    })
                    .collect()
            }
            ParallelismPolicy::Adaptive => {
                let min_len = get_fine_grained_min_len(source_len);
                source.with_min_len(min_len).map(map_op).collect()
            }
        }
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
    pub fn fold_reduce<T, S, FI, FF, FR, R>(source: S, identity: FI, fold_op: FF, reduce_op: FR) -> R
    where
        T: Send + Sync,
        S: IntoParallelIterator<Item = T>,
        FI: Fn() -> R + Clone + Sync + Send,
        FF: Fn(R, T) -> R + Sync + Send,
        FR: Fn(R, R) -> R + Sync + Send,
        R: Send,
    {
        source.into_par_iter().fold(identity.clone(), fold_op).reduce(identity, reduce_op)
    }

    /// Performs mutable foreach in parallel.
    pub fn parallel_foreach_mut<T, F>(source: &mut [T], action: F)
    where
        T: Send + Sync,
        F: Fn(&mut T) + Send + Sync,
    {
        source.par_iter_mut().for_each(action)
    }

    #[cfg(test)]
    pub(crate) fn get_current_parallelism() -> usize {
        COARSE_PARALLELISM.get()
    }

    /// Gets a minimum chunk size for fine-grained work nested under coarse parallel work.
    fn get_fine_grained_min_len(source_len: usize) -> usize {
        let worker_count = rayon::current_num_threads();
        let physical_cpus = *PHYSICAL_CPUS.get_or_init(|| num_cpus::get_physical().max(1));

        // Rayon's adaptive splitting performs well when workers map to physical cores. A wider
        // logical/SMT pool can over-split every inner iterator while several outer searches are
        // already active. Keep a small stealing reserve per worker across the complete coarse level.
        if worker_count <= physical_cpus {
            return 1;
        }

        let coarse_parallelism = COARSE_PARALLELISM.get().max(1);
        let max_tasks = worker_count.saturating_mul(ADAPTIVE_TASKS_PER_WORKER).div_ceil(coarse_parallelism).max(1);

        source_len.div_ceil(max_tasks).max(1)
    }
}

#[cfg(target_arch = "wasm32")]
mod actual {
    use super::ParallelismPolicy;

    /// Creates a cartesian product returning an iterator.
    pub fn cartesian_product<'a, A, B>(a: &'a [A], b: &'a [B]) -> impl Iterator<Item = (&'a A, &'a B)>
    where
        A: Send + Sync + 'a,
        B: Send + Sync + 'a,
    {
        a.iter().flat_map(|a| b.iter().map(move |b| (a, b)))
    }

    /// Maps an indexed collection and collects results synchronously. Policy is ignored on wasm.
    pub fn parallel_collect<T, S, F, R>(source: S, _policy: ParallelismPolicy, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        S: IntoIterator<Item = T>,
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
