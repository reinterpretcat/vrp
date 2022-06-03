use crate::*;
use rosomaxa::example::VectorSolution;
use rosomaxa::population::{DominanceOrdered, RosomaxaWeighted, Shuffled};
use rosomaxa::prelude::*;
use std::any::TypeId;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::MutexGuard;

/// Keeps track of all experiment data for visualization purposes.
#[derive(Default)]
pub struct ExperimentData {
    /// Current generation.
    pub generation: usize,
    /// Called on new individuals addition.
    pub on_add: HashMap<usize, Vec<ObservationData>>,
    /// Called on individual selection.
    pub on_select: HashMap<usize, Vec<ObservationData>>,
    /// Called on generation.
    pub on_generation: HashMap<usize, (HeuristicStatistics, Vec<ObservationData>)>,
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

impl<S> From<&S> for ObservationData
where
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered + 'static,
{
    fn from(solution: &S) -> Self {
        if TypeId::of::<S>() == TypeId::of::<VectorSolution>() {
            let solution = unsafe { std::mem::transmute::<&S, &VectorSolution>(solution) };
            assert_eq!(solution.data.len(), 2);
            return ObservationData::Function(DataPoint3D(solution.data[0], solution.fitness(), solution.data[1]));
        }

        unimplemented!()
    }
}

/// A population type which provides way to intercept some of population data.
pub struct ProxyPopulation<P, O, S>
where
    P: HeuristicPopulation<Objective = O, Individual = S> + 'static,
    O: HeuristicObjective<Solution = S> + Shuffled + 'static,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered + 'static,
{
    generation: usize,
    inner: P,
}

impl<P, O, S> ProxyPopulation<P, O, S>
where
    P: HeuristicPopulation<Objective = O, Individual = S> + 'static,
    O: HeuristicObjective<Solution = S> + Shuffled + 'static,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered + 'static,
{
    /// Creates a new instance of `ProxyPopulation`.
    pub fn new(inner: P) -> Self {
        EXPERIMENT_DATA.lock().unwrap().clear();
        Self { generation: 0, inner }
    }

    fn acquire(&self) -> MutexGuard<ExperimentData> {
        EXPERIMENT_DATA.lock().unwrap()
    }
}

impl<P, O, S> HeuristicPopulation for ProxyPopulation<P, O, S>
where
    P: HeuristicPopulation<Objective = O, Individual = S> + 'static,
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    type Objective = O;
    type Individual = S;

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

        self.acquire().population_state.insert(self.generation, get_population_state(&self.inner));

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

impl<P, O, S> Display for ProxyPopulation<P, O, S>
where
    P: HeuristicPopulation<Objective = O, Individual = S> + 'static,
    O: HeuristicObjective<Solution = S> + Shuffled + 'static,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
