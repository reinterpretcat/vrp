#[cfg(test)]
#[path = "../../tests/unit/population/elitism_test.rs"]
mod elitism_test;

use super::*;
use crate::utils::Random;
use crate::{HeuristicSpeed, HeuristicStatistics};
use std::cmp::Ordering;
use std::fmt::{Formatter, Write};
use std::iter::{empty, once};
use std::ops::RangeBounds;
use std::sync::Arc;

/// A function type to deduplicate individuals.
pub type DedupFn<O, S> = Box<dyn Fn(&O, &S, &S) -> bool + Send + Sync>;

/// A simple evolution aware implementation of `Population` trait which keeps predefined amount
/// of best known individuals.
pub struct Elitism<F, O, S>
where
    F: HeuristicFitness,
    O: HeuristicObjective<Solution = S, Fitness = F> + Shuffled,
    S: HeuristicSolution<Fitness = F>,
{
    objective: Arc<O>,
    random: Arc<dyn Random + Send + Sync>,
    selection_size: usize,
    max_population_size: usize,
    individuals: Vec<S>,
    speed: Option<HeuristicSpeed>,
    dedup_fn: DedupFn<O, S>,
}

/// Provides way to get a new objective by shuffling existing one.
pub trait Shuffled {
    /// Returns a new objective.
    fn get_shuffled(&self, random: &(dyn Random + Send + Sync)) -> Self;
}

impl<F, O, S> HeuristicPopulation for Elitism<F, O, S>
where
    F: HeuristicFitness,
    O: HeuristicObjective<Solution = S, Fitness = F> + Shuffled,
    S: HeuristicSolution<Fitness = F>,
{
    type Objective = O;
    type Individual = S;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        if individuals.is_empty() {
            return false;
        }

        self.add_with_iter(individuals.into_iter())
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        self.add_with_iter(once(individual))
    }

    fn on_generation(&mut self, statistics: &HeuristicStatistics) {
        self.speed = Some(statistics.speed.clone());
    }

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.objective.total_order(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        let selection_size = match &self.speed {
            Some(HeuristicSpeed::Slow { ratio, .. }) => (self.selection_size as f64 * ratio).max(1.).round() as usize,
            _ => self.selection_size,
        };

        if self.individuals.is_empty() {
            Box::new(empty())
        } else {
            Box::new(
                once(0_usize)
                    .chain(
                        (1..selection_size).map(move |_| self.random.uniform_int(0, self.size() as i32 - 1) as usize),
                    )
                    .take(selection_size)
                    .filter_map(move |idx| self.individuals.get(idx)),
            )
        }
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        Box::new(self.individuals.iter())
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        Box::new(self.individuals.iter())
    }

    fn size(&self) -> usize {
        self.individuals.len()
    }

    fn selection_phase(&self) -> SelectionPhase {
        SelectionPhase::Exploitation
    }
}

impl<F, O, S> Elitism<F, O, S>
where
    F: HeuristicFitness,
    O: HeuristicObjective<Solution = S, Fitness = F> + Shuffled,
    S: HeuristicSolution<Fitness = F>,
{
    /// Creates a new instance of `Elitism`.
    pub fn new(
        objective: Arc<O>,
        random: Arc<dyn Random + Send + Sync>,
        max_population_size: usize,
        selection_size: usize,
    ) -> Self {
        Self::new_with_dedup(
            objective,
            random,
            max_population_size,
            selection_size,
            // NOTE dedup solutions only if their objectives are the same
            Box::new(|objective, a, b| objective.total_order(a, b) == Ordering::Equal),
        )
    }

    fn add_with_iter<I>(&mut self, iter: I) -> bool
    where
        I: Iterator<Item = S>,
    {
        let best_known_fitness = self.individuals.first().map(|i| i.fitness());

        self.individuals.extend(iter);

        self.sort();
        self.ensure_max_population_size();
        self.is_improved(best_known_fitness)
    }

    /// Creates a new instance of `Elitism` with custom deduplication function.
    pub fn new_with_dedup(
        objective: Arc<O>,
        random: Arc<dyn Random + Send + Sync>,
        max_population_size: usize,
        selection_size: usize,
        dedup_fn: DedupFn<O, S>,
    ) -> Self {
        assert!(max_population_size > 0);
        Self { objective, random, selection_size, max_population_size, individuals: vec![], speed: None, dedup_fn }
    }

    /// Shuffles objective function.
    pub fn shuffle_objective(&mut self) {
        self.objective = Arc::new(self.objective.get_shuffled(self.random.as_ref()));
    }

    /// Extracts all individuals from population.
    pub fn drain<R>(&mut self, range: R) -> Vec<S>
    where
        R: RangeBounds<usize>,
    {
        self.individuals.drain(range).collect()
    }

    fn sort(&mut self) {
        self.individuals.sort_by(|a, b| self.objective.total_order(a, b));
        self.individuals.dedup_by(|a, b| (self.dedup_fn)(&self.objective, a, b));
    }

    fn ensure_max_population_size(&mut self) {
        if self.individuals.len() > self.max_population_size {
            self.individuals.truncate(self.max_population_size);
        }
    }

    fn is_improved(&self, best_known_fitness: Option<F>) -> bool {
        best_known_fitness.zip(self.individuals.first()).map_or(true, |(best_known_fitness, new_best_known)| {
            best_known_fitness
                .iter()
                .zip(new_best_known.fitness().iter())
                .any(|(a, b)| compare_floats(a, b) != Ordering::Equal)
        })
    }
}

impl<F, O, S> Display for Elitism<F, O, S>
where
    F: HeuristicFitness,
    O: HeuristicObjective<Solution = S, Fitness = F> + Shuffled,
    S: HeuristicSolution<Fitness = F>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fitness = self.individuals.iter().fold(String::new(), |mut res, individual| {
            let values = individual.fitness();
            write!(&mut res, "[{values}],").unwrap();

            res
        });

        write!(f, "[{fitness}]")
    }
}
