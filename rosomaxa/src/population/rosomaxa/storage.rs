use super::{RosomaxaContext, RosomaxaSolution};
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

/// Keeps complementary representatives of a node's infeasible neighborhood.
///
/// The boundary stays close to feasibility. A bridge is allowed farther away only while it improves upon the
/// node's feasible solution under the original objective. During shuffled replay, before a feasible resident arrives,
/// the two slots provisionally retain the violation-best and objective-best candidates.
pub(super) struct RelaxedStorage<S> {
    pub(super) boundary: Option<Box<S>>,
    pub(super) bridge: Option<Box<S>>,
}

impl<S> Default for RelaxedStorage<S> {
    fn default() -> Self {
        Self { boundary: None, bridge: None }
    }
}

impl<S: RosomaxaSolution> RelaxedStorage<S> {
    pub(super) fn add<O: HeuristicObjective<Solution = S>>(&mut self, objective: &O, regular: Option<&S>, input: S) {
        match regular {
            Some(regular) => {
                let is_bridge = objective.total_order(&input, regular) == Ordering::Less;
                Self::update(objective, if is_bridge { &mut self.bridge } else { &mut self.boundary }, Box::new(input));
            }
            None => self.add_unclassified(objective, input),
        }
    }

    pub(super) fn on_regular_updated<O: HeuristicObjective<Solution = S>>(
        &mut self,
        objective: &O,
        regular: Option<&S>,
    ) {
        if let Some(regular) = regular {
            let boundary = self.boundary.take();
            let bridge = self.bridge.take();
            boundary.into_iter().chain(bridge).for_each(|input| {
                let is_bridge = objective.total_order(input.as_ref(), regular) == Ordering::Less;
                Self::update(objective, if is_bridge { &mut self.bridge } else { &mut self.boundary }, input);
            });
        }
    }

    pub(super) fn select<O: HeuristicObjective<Solution = S>>(&self, objective: &O, reference: &S) -> Option<&S> {
        self.bridge
            .as_deref()
            .filter(|bridge| objective.total_order(bridge, reference) == Ordering::Less)
            .or(self.boundary.as_deref())
            .or(self.bridge.as_deref())
    }

    pub(super) fn representative<O: HeuristicObjective<Solution = S>>(&self, objective: &O) -> Option<&S> {
        match (self.boundary.as_deref(), self.bridge.as_deref()) {
            (Some(boundary), Some(bridge)) => {
                Some(if compare_relaxed(objective, boundary, bridge) == Ordering::Greater { bridge } else { boundary })
            }
            (Some(boundary), None) => Some(boundary),
            (None, Some(bridge)) => Some(bridge),
            (None, None) => None,
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &S> {
        self.boundary.iter().chain(&self.bridge).map(Box::as_ref)
    }

    pub(super) fn drain_all(&mut self) -> impl Iterator<Item = S> {
        self.boundary.take().into_iter().chain(self.bridge.take()).map(|solution| *solution)
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
        if self.bridge.is_some()
            && range.contains(&rank)
            && let Some(solution) = self.bridge.take()
        {
            result.push(*solution);
        }

        result
    }

    pub(super) fn clear(&mut self) {
        self.boundary = None;
        self.bridge = None;
    }

    pub(super) fn len(&self) -> usize {
        usize::from(self.boundary.is_some()) + usize::from(self.bridge.is_some())
    }

    fn update<O: HeuristicObjective<Solution = S>>(objective: &O, slot: &mut Option<Box<S>>, input: Box<S>) {
        let is_better =
            slot.as_deref().is_none_or(|known| compare_relaxed(objective, input.as_ref(), known) == Ordering::Less);
        if is_better {
            *slot = Some(input);
        }
    }

    fn add_unclassified<O: HeuristicObjective<Solution = S>>(&mut self, objective: &O, input: S) {
        let mut candidates = [self.boundary.take(), self.bridge.take(), Some(Box::new(input))];
        let boundary_idx = (0..candidates.len())
            .filter(|&index| candidates[index].is_some())
            .min_by(|&left, &right| {
                compare_relaxed(
                    objective,
                    candidates[left].as_deref().expect("candidate is present"),
                    candidates[right].as_deref().expect("candidate is present"),
                )
            })
            .expect("new relaxed candidate is present");
        let objective_idx = (0..candidates.len())
            .filter(|&index| candidates[index].is_some())
            .min_by(|&left_idx, &right_idx| {
                let left = candidates[left_idx].as_deref().expect("candidate is present");
                let right = candidates[right_idx].as_deref().expect("candidate is present");
                objective
                    .total_order(left, right)
                    .then_with(|| compare_relaxed(objective, left, right))
                    .then_with(|| left_idx.cmp(&right_idx))
            })
            .expect("new relaxed candidate is present");

        self.boundary = candidates[boundary_idx].take();
        if objective_idx != boundary_idx {
            self.bridge = candidates[objective_idx].take();
        }
    }
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
            let feasible_reference = self.regular.best_with_objective(objective);
            self.relaxed.add(objective, feasible_reference, input);
        } else {
            self.regular.add(input);
            let objective = self.relaxed_objective.as_ref();
            let feasible_reference = self.regular.best_with_objective(objective);
            self.relaxed.on_regular_updated(objective, feasible_reference);
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
