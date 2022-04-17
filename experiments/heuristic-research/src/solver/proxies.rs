use crate::*;
use rosomaxa::example::*;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::MutexGuard;

/// A type alias for vector based population.
pub type VectorPopulation =
    Box<dyn HeuristicPopulation<Objective = VectorObjective, Individual = VectorSolution> + Send + Sync>;

/// Keeps track of all experiment data for visualization purposes.
#[derive(Default)]
pub struct ExperimentData {
    /// Current generation.
    pub generation: usize,
    /// Called on new individuals addition.
    pub on_add: HashMap<usize, Vec<DataPoint3D>>,
    /// Called on individual selection.
    pub on_select: HashMap<usize, Vec<DataPoint3D>>,
    /// Called on generation.
    pub on_generation: HashMap<usize, (HeuristicStatistics, Vec<DataPoint3D>)>,
    /// Keeps track population state at generation.
    pub population_state: HashMap<usize, PopulationState>,
}

impl ExperimentData {
    /// Clears all data strored.
    pub fn clear(&mut self) {
        self.generation = 0;
        self.on_add.clear();
        self.on_select.clear();
        self.on_generation.clear();
    }
}

impl From<&VectorSolution> for DataPoint3D {
    fn from(solution: &VectorSolution) -> Self {
        assert_eq!(solution.data.len(), 2);
        DataPoint3D(solution.data[0], solution.fitness(), solution.data[1])
    }
}

/// A population type which provides way to intercept some of population data.
pub struct ProxyPopulation {
    generation: usize,
    inner: VectorPopulation,
}

impl ProxyPopulation {
    /// Creates a new instance of `ProxyPopulation`.
    pub fn new(inner: VectorPopulation) -> Self {
        EXPERIMENT_DATA.lock().unwrap().clear();
        Self { generation: 0, inner }
    }

    fn acquire(&self) -> MutexGuard<ExperimentData> {
        EXPERIMENT_DATA.lock().unwrap()
    }
}

impl HeuristicPopulation for ProxyPopulation {
    type Objective = VectorObjective;
    type Individual = VectorSolution;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        self.acquire()
            .on_add
            .entry(self.generation)
            .or_insert_with(Vec::new)
            .extend(individuals.iter().map(|i| i.into()));

        self.inner.add_all(individuals)
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        self.acquire().on_add.entry(self.generation).or_insert_with(Vec::new).push((&individual).into());

        self.inner.add(individual)
    }

    fn on_generation(&mut self, statistics: &HeuristicStatistics) {
        self.generation = statistics.generation;
        self.acquire().generation = statistics.generation;

        let individuals = self.inner.all().map(|individual| individual.into()).collect();
        self.acquire().on_generation.insert(self.generation, (statistics.clone(), individuals));

        self.acquire().population_state.insert(self.generation, parse_population_state(self.inner.to_string()));

        self.inner.on_generation(statistics)
    }

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.inner.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        Box::new(self.inner.select().map(|individual| {
            self.acquire().on_select.entry(self.generation).or_insert_with(Vec::new).push(individual.into());

            individual
        }))
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Self::Individual, usize)> + 'a> {
        self.inner.ranked()
    }

    fn all<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        self.inner.all()
    }

    fn size(&self) -> usize {
        self.inner.size()
    }

    fn selection_phase(&self) -> SelectionPhase {
        self.inner.selection_phase()
    }
}

impl Display for ProxyPopulation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
