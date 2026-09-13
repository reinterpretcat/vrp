use super::{RelaxedSolutionProgress, RosomaxaContext, RosomaxaSolution};
use crate::algorithms::gsom::{Storage, StorageFactory};
use crate::algorithms::math::relative_distance;
use crate::evolution::objectives::HeuristicObjective;
use crate::population::HeuristicPopulation;
use crate::population::elitism::{Alternative, DedupFn, Elitism};
use crate::utils::{Float, Random};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Bound, Range, RangeBounds};
use std::sync::Arc;

pub(super) struct IndividualStorageFactory<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    pub(super) node_size: usize,
    pub(super) random: Arc<dyn Random>,
    pub(super) objective: Arc<O>,
}

impl<C, O, S> StorageFactory<C, S, IndividualStorage<C, O, S>> for IndividualStorageFactory<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    fn eval(&self, _: &C) -> IndividualStorage<C, O, S> {
        let mut regular = Elitism::new_with_dedup(
            self.objective.clone(),
            self.random.clone(),
            self.node_size,
            self.node_size,
            create_dedup_fn(0.1),
        );
        regular.maybe_change();

        IndividualStorage { regular, relaxed_objective: self.objective.clone(), relaxed: RelaxedStorage::default() }
    }
}

pub(super) fn compare_relaxed<O, S>(objective: &O, left: &S, right: &S) -> Ordering
where
    O: HeuristicObjective<Solution = S>,
    S: RosomaxaSolution,
{
    left.relaxed_violation()
        .unwrap_or_default()
        .total_cmp(&right.relaxed_violation().unwrap_or_default())
        .then_with(|| objective.total_order(left, right))
}

pub(super) fn is_relaxed_selectable<S: RosomaxaSolution>(solution: &S) -> bool {
    solution.is_relaxed_continuation() || solution.relaxed_progress().is_none()
}

/// Keeps complementary representatives of a node's relaxed-search neighborhood.
///
/// The boundary records the best repair progress. The continuation records a checkpoint which is still being
/// educated, even when that checkpoint is temporarily worse under boundary ordering.
pub(super) struct RelaxedStorage<S> {
    pub(super) boundary: Option<Box<S>>,
    pub(super) continuation: Option<Box<S>>,
}

impl<S> Default for RelaxedStorage<S> {
    fn default() -> Self {
        Self { boundary: None, continuation: None }
    }
}

impl<S: RosomaxaSolution> RelaxedStorage<S> {
    pub(super) fn add<O: HeuristicObjective<Solution = S>>(&mut self, objective: &O, input: S) {
        let mut candidates = [self.boundary.take(), self.continuation.take(), Some(Box::new(input))];
        let (boundary_idx, continuation_idx) = select_relaxed_roles(objective, &candidates);

        self.boundary = candidates[boundary_idx].take();
        self.continuation = continuation_idx.and_then(|index| candidates[index].take());
    }

    /// Replaces an updated checkpoint and prevents an older step of its episode from remaining active after migration.
    pub(super) fn supersede_continuation(&mut self, progress: RelaxedSolutionProgress) {
        if self.boundary.as_deref().and_then(S::relaxed_progress) == Some(progress) {
            self.boundary = None;
        } else if let Some(boundary) = self.boundary.as_deref_mut()
            && boundary
                .relaxed_progress()
                .is_some_and(|known| known.episode() == progress.episode() && known < progress)
        {
            boundary.end_relaxed_continuation();
        }

        if self.continuation.as_deref().and_then(S::relaxed_progress) == Some(progress)
            || self
                .continuation
                .as_deref()
                .and_then(S::relaxed_progress)
                .is_some_and(|known| known.episode() == progress.episode() && known < progress)
        {
            self.continuation = None;
        }
    }

    pub(super) fn remove_episode(&mut self, episode: usize) {
        if self.boundary.as_deref().and_then(S::relaxed_progress).is_some_and(|known| known.episode() == episode) {
            self.boundary = None;
        }
        if self.continuation.as_deref().and_then(S::relaxed_progress).is_some_and(|known| known.episode() == episode) {
            self.continuation = None;
        }
    }

    pub(super) fn select(&self, prefer_continuation: bool) -> Option<&S> {
        let continuation = self
            .iter()
            .filter(|solution| solution.is_relaxed_continuation())
            .max_by_key(|solution| solution.relaxed_progress());
        let boundary = self.boundary.as_deref().filter(|solution| is_relaxed_selectable(*solution));

        if prefer_continuation { continuation } else { boundary.or(continuation) }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &S> {
        self.boundary.iter().chain(&self.continuation).map(Box::as_ref)
    }

    pub(super) fn drain_all(&mut self) -> impl Iterator<Item = S> {
        self.boundary.take().into_iter().chain(self.continuation.take()).map(|solution| *solution)
    }

    pub(super) fn drain(&mut self, range: Range<usize>) -> Vec<S> {
        let mut result = Vec::with_capacity(range.len());
        let mut rank = 0;

        if self.boundary.is_some() {
            if range.contains(&rank)
                && let Some(solution) = self.boundary.take()
            {
                result.push(*solution);
            }
            rank += 1;
        }
        if self.continuation.is_some()
            && range.contains(&rank)
            && let Some(solution) = self.continuation.take()
        {
            result.push(*solution);
        }

        result
    }

    pub(super) fn clear(&mut self) {
        self.boundary = None;
        self.continuation = None;
    }

    pub(super) fn len(&self) -> usize {
        usize::from(self.boundary.is_some()) + usize::from(self.continuation.is_some())
    }
}

fn select_relaxed_roles<O, S>(objective: &O, candidates: &[Option<Box<S>>]) -> (usize, Option<usize>)
where
    O: HeuristicObjective<Solution = S>,
    S: RosomaxaSolution,
{
    let iter_candidates = || {
        candidates
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| candidate.as_deref().map(|candidate| (index, candidate)))
            // The later value carries the latest lease/recovery metadata for this checkpoint.
            .filter(|(index, candidate)| {
                !candidates[index + 1..]
                    .iter()
                    .flatten()
                    .any(|later| is_same_relaxed_checkpoint(*candidate, later.as_ref()))
            })
    };
    let boundary_idx = iter_candidates()
        .min_by(|(left_idx, left), (right_idx, right)| {
            compare_relaxed(objective, left, right)
                // Prefer the latest checkpoint when repair quality is equal. This also preserves metadata updates.
                .then_with(|| right.relaxed_progress().cmp(&left.relaxed_progress()))
                .then_with(|| right_idx.cmp(left_idx))
        })
        .map(|(index, _)| index)
        .expect("new relaxed candidate is present");
    let boundary = candidates[boundary_idx].as_deref().expect("boundary is present");
    let continuation_idx = iter_candidates()
        .filter(|(index, candidate)| {
            *index != boundary_idx && candidate.is_relaxed_continuation() && !candidate.is_same(boundary)
        })
        .max_by(|(left_idx, left), (right_idx, right)| {
            left.relaxed_progress().cmp(&right.relaxed_progress()).then_with(|| left_idx.cmp(right_idx))
        })
        .map(|(index, _)| index);

    (boundary_idx, continuation_idx)
}

fn is_same_relaxed_checkpoint<S: RosomaxaSolution>(left: &S, right: &S) -> bool {
    left.relaxed_progress().zip(right.relaxed_progress()).is_some_and(|(left, right)| left == right)
        || left.is_same(right)
}

pub(super) struct IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    pub(super) regular: Elitism<O, S>,
    pub(super) relaxed_objective: Arc<O>,
    pub(super) relaxed: RelaxedStorage<S>,
}

impl<C, O, S> IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    pub(super) fn select(&self, random: &dyn Random, alternative_probability: Float) -> Option<&S> {
        let rank = match self.regular.size() {
            0 => return None,
            size if size > 1 && random.is_hit(alternative_probability) => {
                random.uniform_int(1, size as i32 - 1) as usize
            }
            _ => 0,
        };

        self.regular.get(rank)
    }
}

impl<C, O, S> Storage for IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    type Item = S;

    fn add(&mut self, input: Self::Item) {
        if input.relaxed_violation().is_some() {
            let objective = self.relaxed_objective.as_ref();
            self.relaxed.add(objective, input);
        } else {
            self.regular.add(input);
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &'_ Self::Item> + '_> {
        Box::new(self.regular.ranked().chain(self.relaxed.iter()))
    }

    fn drain<R>(&mut self, range: R) -> Vec<Self::Item>
    where
        R: RangeBounds<usize>,
    {
        let regular_size = self.regular.size();
        let relaxed_size = self.relaxed.len();
        let range = resolve_range(range, regular_size + relaxed_size);
        let regular_range = range.start.min(regular_size)..range.end.min(regular_size);
        let relaxed_range = range.start.saturating_sub(regular_size).min(relaxed_size)
            ..range.end.saturating_sub(regular_size).min(relaxed_size);

        let relaxed = self.relaxed.drain(relaxed_range);

        self.regular.drain(regular_range).into_iter().chain(relaxed).collect()
    }

    fn resize(&mut self, size: usize) {
        self.regular.set_max_population_size(size);
    }

    fn size(&self) -> usize {
        self.regular.size() + self.relaxed.len()
    }
}

impl<C, O, S> Display for IndividualStorage<C, O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let relaxed = self
            .relaxed
            .iter()
            .map(|solution| solution.fitness().map(|value| format!("{value:.7}")).collect::<Vec<_>>().join(","))
            .map(|fitness| format!("[{fitness}]"))
            .collect::<Vec<_>>()
            .join(",");
        // Keep the regular population first: existing state consumers read its leading fitness vector.
        write!(f, "{}; relaxed: [{}]", self.regular, relaxed)
    }
}

fn resolve_range<R: RangeBounds<usize>>(range: R, len: usize) -> Range<usize> {
    let start = match range.start_bound() {
        Bound::Included(start) => *start,
        Bound::Excluded(start) => start.saturating_add(1),
        Bound::Unbounded => 0,
    }
    .min(len);
    let end = match range.end_bound() {
        Bound::Included(end) => end.saturating_add(1),
        Bound::Excluded(end) => *end,
        Bound::Unbounded => len,
    }
    .min(len)
    .max(start);

    start..end
}

pub(super) fn create_dedup_fn<C, O, S>(threshold: Float) -> DedupFn<O, S>
where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    // NOTE custom dedup rule to increase diversity property
    Box::new(move |objective, a, b| match objective.total_order(a, b) {
        Ordering::Equal => {
            let fitness_a = a.fitness();
            let fitness_b = b.fitness();

            fitness_a.zip(fitness_b).all(|(a, b)| a == b)
        }
        _ => {
            let distance = relative_distance(a.weights().iter(), b.weights().iter());

            distance < threshold
        }
    })
}
