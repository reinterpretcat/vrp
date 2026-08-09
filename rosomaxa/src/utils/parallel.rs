#[cfg(test)]
#[path = "../../tests/unit/utils/parallel_test.rs"]
mod parallel_test;

pub use self::actual::cartesian_product;
pub use self::actual::fold_reduce;
pub use self::actual::map_reduce;
pub use self::actual::parallel_collect;
pub use self::actual::parallel_foreach_mut;
pub use self::actual::parallel_into_collect;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use self::actual::get_current_parallelism;

/// Specifies whether mapped items represent local work or coarse operations which can launch
/// nested parallel work.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ParallelismScope {
    /// Use for local work which does not launch nested parallel work.
    #[default]
    Local,
    /// Use when each mapped item is an independent coarse operation which can launch nested work.
    Coarse,
}

/// Specifies how a fine-grained indexed iterator should be split.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ParallelismPolicy {
    /// Uses Rayon's default splitting strategy.
    #[default]
    Default,
    /// Adapts the minimum chunk size to the active pool and enclosing coarse operations.
    Adaptive(std::num::NonZeroUsize),
}

impl ParallelismPolicy {
    /// Creates an adaptive policy with the desired amount of stealable fine-grained tasks per
    /// worker across all coarse operations.
    pub const fn adaptive(tasks_per_worker: usize) -> Self {
        match std::num::NonZeroUsize::new(tasks_per_worker) {
            Some(tasks_per_worker) => Self::Adaptive(tasks_per_worker),
            None => panic!("tasks per worker must be greater than zero"),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod actual {
    use super::{ParallelismPolicy, ParallelismScope};
    use rayon::prelude::*;
    use std::cell::Cell;
    use std::sync::OnceLock;

    thread_local! {
        static COARSE_PARALLELISM: Cell<usize> = const { Cell::new(1) };
    }

    static PHYSICAL_CPUS: OnceLock<usize> = OnceLock::new();

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

    /// Creates a cartesian product returning a parallel iterator using the given splitting policy.
    pub fn cartesian_product<'a, A, B>(
        a: &'a [A],
        b: &'a [B],
        policy: ParallelismPolicy,
    ) -> impl IndexedParallelIterator<Item = (&'a A, &'a B)>
    where
        A: Send + Sync + 'a,
        B: Send + Sync + 'a,
    {
        let b_len = b.len();
        let product_len = a.len().checked_mul(b_len).expect("cartesian product size overflow");
        let min_len = get_min_len(product_len, policy);

        // A single indexed range lets rayon split the complete product directly. Nested parallel
        // iterators create a task tree per item in `a`, which adds work-stealing overhead when the
        // caller itself is already running multiple searches in parallel.
        (0..product_len).into_par_iter().with_min_len(min_len.max(1)).map(move |idx| (&a[idx / b_len], &b[idx % b_len]))
    }

    /// Maps an indexed collection and collects results in parallel using the given scope and policy.
    pub fn parallel_collect<T, S, FM, R>(
        source: S,
        scope: ParallelismScope,
        policy: ParallelismPolicy,
        map_op: FM,
    ) -> Vec<R>
    where
        T: Send + Sync,
        S: IntoParallelIterator<Item = T>,
        S::Iter: IndexedParallelIterator,
        FM: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        let source = source.into_par_iter();
        let source_len = source.len();
        let coarse_parallelism = get_coarse_parallelism(scope, source_len);

        match policy {
            ParallelismPolicy::Default => collect(source, coarse_parallelism, map_op),
            ParallelismPolicy::Adaptive(tasks_per_worker) => {
                let min_len = get_fine_grained_min_len(source_len, tasks_per_worker.get());
                collect(source.with_min_len(min_len), coarse_parallelism, map_op)
            }
        }
    }

    #[inline]
    fn collect<T, S, FM, R>(source: S, coarse_parallelism: Option<usize>, map_op: FM) -> Vec<R>
    where
        T: Send,
        S: ParallelIterator<Item = T>,
        FM: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        match coarse_parallelism {
            Some(parallelism) => source
                .map(|item| {
                    let _guard = CoarseParallelismGuard::set(parallelism);
                    map_op(item)
                })
                .collect(),
            None => source.map(map_op).collect(),
        }
    }

    /// Maps collection and collects results into vector in parallel using the given scope.
    pub fn parallel_into_collect<T, F, R>(source: Vec<T>, scope: ParallelismScope, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        parallel_collect(source, scope, ParallelismPolicy::Default, map_op)
    }

    /// Performs map reduce operations in parallel using the given scope.
    pub fn map_reduce<'a, T, S, FM, FR, FD, R>(
        source: &'a S,
        scope: ParallelismScope,
        map_op: FM,
        default_op: FD,
        reduce_op: FR,
    ) -> R
    where
        T: Send + Sync,
        S: IntoParallelRefIterator<'a, Item = T> + ?Sized,
        FM: Fn(T) -> R + Sync + Send,
        FR: Fn(R, R) -> R + Sync + Send,
        FD: Fn() -> R + Sync + Send,
        R: Send,
    {
        let source = source.par_iter();
        match get_coarse_parallelism(scope, source.opt_len().unwrap_or(1)) {
            Some(parallelism) => source
                .map(|item| {
                    let _guard = CoarseParallelismGuard::set(parallelism);
                    map_op(item)
                })
                .reduce(default_op, reduce_op),
            None => source.map(map_op).reduce(default_op, reduce_op),
        }
    }

    /// Performs fold and then reduce operations in parallel using the given scope.
    pub fn fold_reduce<T, S, FI, FF, FR, R>(
        source: S,
        scope: ParallelismScope,
        identity: FI,
        fold_op: FF,
        reduce_op: FR,
    ) -> R
    where
        T: Send + Sync,
        S: IntoParallelIterator<Item = T>,
        FI: Fn() -> R + Clone + Sync + Send,
        FF: Fn(R, T) -> R + Sync + Send,
        FR: Fn(R, R) -> R + Sync + Send,
        R: Send,
    {
        let source = source.into_par_iter();
        match get_coarse_parallelism(scope, source.opt_len().unwrap_or(1)) {
            Some(parallelism) => source
                .fold(identity.clone(), |acc, item| {
                    let _guard = CoarseParallelismGuard::set(parallelism);
                    fold_op(acc, item)
                })
                .reduce(identity, reduce_op),
            None => source.fold(identity.clone(), fold_op).reduce(identity, reduce_op),
        }
    }

    /// Performs mutable foreach in parallel using the given scope.
    pub fn parallel_foreach_mut<T, F>(source: &mut [T], scope: ParallelismScope, action: F)
    where
        T: Send + Sync,
        F: Fn(&mut T) + Send + Sync,
    {
        match get_coarse_parallelism(scope, source.len()) {
            Some(parallelism) => source.par_iter_mut().for_each(|item| {
                let _guard = CoarseParallelismGuard::set(parallelism);
                action(item)
            }),
            None => source.par_iter_mut().for_each(action),
        }
    }

    fn get_coarse_parallelism(scope: ParallelismScope, source_len: usize) -> Option<usize> {
        let current = COARSE_PARALLELISM.get();

        match scope {
            ParallelismScope::Local => None,
            ParallelismScope::Coarse => Some(current.saturating_mul(source_len).max(1)),
        }
    }

    #[cfg(test)]
    pub(crate) fn get_current_parallelism() -> usize {
        COARSE_PARALLELISM.get()
    }

    fn get_min_len(source_len: usize, policy: ParallelismPolicy) -> usize {
        match policy {
            ParallelismPolicy::Default => 1,
            ParallelismPolicy::Adaptive(tasks_per_worker) => {
                get_fine_grained_min_len(source_len, tasks_per_worker.get())
            }
        }
    }

    /// Gets a minimum chunk size for fine-grained work nested under coarse parallel work.
    fn get_fine_grained_min_len(source_len: usize, tasks_per_worker: usize) -> usize {
        let worker_count = rayon::current_num_threads();
        let physical_cpus = *PHYSICAL_CPUS.get_or_init(|| num_cpus::get_physical().max(1));

        // Rayon's adaptive splitting performs well when workers map to physical cores. A wider
        // logical/SMT pool can over-split every inner iterator while several outer searches are
        // already active. Keep a small stealing reserve per worker across the complete coarse level.
        if worker_count <= physical_cpus {
            return 1;
        }

        let coarse_parallelism = COARSE_PARALLELISM.get().max(1);
        let max_tasks = worker_count.saturating_mul(tasks_per_worker).div_ceil(coarse_parallelism).max(1);

        source_len.div_ceil(max_tasks).max(1)
    }
}

#[cfg(target_arch = "wasm32")]
mod actual {
    use super::{ParallelismPolicy, ParallelismScope};

    /// Creates a cartesian product returning an iterator. Splitting policy is ignored on wasm.
    pub fn cartesian_product<'a, A, B>(
        a: &'a [A],
        b: &'a [B],
        _policy: ParallelismPolicy,
    ) -> impl Iterator<Item = (&'a A, &'a B)>
    where
        A: Send + Sync + 'a,
        B: Send + Sync + 'a,
    {
        a.iter().flat_map(|a| b.iter().map(move |b| (a, b)))
    }

    /// Maps an indexed collection and collects results synchronously. Scope and policy are ignored
    /// on wasm.
    pub fn parallel_collect<T, S, F, R>(
        source: S,
        _scope: ParallelismScope,
        _policy: ParallelismPolicy,
        map_op: F,
    ) -> Vec<R>
    where
        T: Send + Sync,
        S: IntoIterator<Item = T>,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        source.into_iter().map(map_op).collect()
    }

    /// Map collections and collects results into vector synchronously. Scope is ignored on wasm.
    pub fn parallel_into_collect<T, F, R>(source: Vec<T>, _scope: ParallelismScope, map_op: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        parallel_collect(source, _scope, ParallelismPolicy::Default, map_op)
    }

    /// Performs map and reduce operations synchronously. Scope is ignored on wasm.
    pub fn map_reduce<T, S, FM, FR, FD, R>(
        source: S,
        _scope: ParallelismScope,
        map_op: FM,
        default_op: FD,
        reduce_op: FR,
    ) -> R
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
    pub fn fold_reduce<T, S, FI, FF, FR, R>(
        source: S,
        _scope: ParallelismScope,
        identity: FI,
        fold: FF,
        mut reduce: FR,
    ) -> R
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
    pub fn parallel_foreach_mut<T, F>(source: &mut [T], _scope: ParallelismScope, action: F)
    where
        T: Send + Sync,
        F: Fn(&mut T) + Send + Sync,
    {
        source.iter_mut().for_each(action)
    }
}
