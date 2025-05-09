use crate::*;
use rosomaxa::example::VectorSolution;
use rosomaxa::population::{Alternative, RosomaxaContext, RosomaxaSolution};
use rosomaxa::prelude::*;
use std::any::TypeId;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::MutexGuard;
use vrp_scientific::core::models::common::Shadow;
use vrp_scientific::core::prelude::*;

/// Keeps track of all experiment data for visualization purposes.
#[derive(Default, Serialize, Deserialize)]
pub struct ExperimentData {
    /// Current generation.
    pub generation: usize,
    /// Called on new individuals addition.
    pub on_add: HashMap<usize, Vec<ObservationData>>,
    /// Called on individual selection.
    pub on_select: HashMap<usize, Vec<ObservationData>>,
    /// Called on generation.
    pub on_generation: HashMap<usize, (FootprintState, Vec<ObservationData>)>,
    /// Keeps track of population state at specific generation.
    pub population_state: HashMap<usize, PopulationState>,
    /// Keeps track of heuristic state at specific generation.
    pub heuristic_state: HyperHeuristicState,
}

impl ExperimentData {
    /// Clears all stored data.
    pub fn clear(&mut self) {
        self.generation = 0;
        self.on_add.clear();
        self.on_select.clear();
        self.on_generation.clear();
    }
}

impl<'a> TryFrom<&'a str> for ExperimentData {
    type Error = String;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        serde_json::from_str(value).map_err(|err| format!("cannot deserialize experiment data: {err}"))
    }
}

impl<C, S> From<&S> for ObservationData
where
    C: RosomaxaContext<Solution = S> + 'static,
    S: RosomaxaSolution<Context = C> + 'static,
{
    fn from(solution: &S) -> Self {
        if TypeId::of::<S>() == TypeId::of::<VectorSolution>() {
            // SAFETY: type id check above ensures that S-type is the right one
            let solution = unsafe { std::mem::transmute::<&S, &VectorSolution>(solution) };

            let fitness = solution.fitness().next().expect("should have fitness");
            assert_eq!(solution.data.len(), 2);
            return ObservationData::Function(DataPoint3D(solution.data[0], fitness, solution.data[1]));
        }

        if TypeId::of::<S>() == TypeId::of::<InsertionContext>() {
            // SAFETY: type id check above ensures that S-type is the right one
            let insertion_ctx = unsafe { std::mem::transmute::<&S, &InsertionContext>(solution) };

            let shadow = Shadow::from(insertion_ctx);
            return ObservationData::Vrp((&shadow).into());
        }

        unreachable!("type is not supported by observation data");
    }
}

/// A population type which provides a way to intercept some of the population data.
pub struct ProxyPopulation<P, C, O, S>
where
    P: HeuristicPopulation<Objective = O, Individual = S> + 'static,
    C: RosomaxaContext<Solution = S> + 'static,
    O: HeuristicObjective<Solution = S> + Alternative + 'static,
    S: RosomaxaSolution<Context = C> + 'static,
{
    generation: usize,
    inner: P,
    _phantom: PhantomData<C>,
}

impl<P, C, O, S> ProxyPopulation<P, C, O, S>
where
    P: HeuristicPopulation<Objective = O, Individual = S> + 'static,
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative + 'static,
    S: RosomaxaSolution<Context = C>,
{
    /// Creates a new instance of `ProxyPopulation`.
    pub fn new(inner: P) -> Self {
        EXPERIMENT_DATA.lock().unwrap().clear();
        Self { generation: 0, inner, _phantom: Default::default() }
    }

    fn acquire(&self) -> MutexGuard<ExperimentData> {
        EXPERIMENT_DATA.lock().unwrap()
    }
}

impl<P, C, O, S> HeuristicPopulation for ProxyPopulation<P, C, O, S>
where
    P: HeuristicPopulation<Objective = O, Individual = S>,
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    type Objective = O;
    type Individual = S;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        self.acquire().on_add.entry(self.generation).or_default().extend(individuals.iter().map(|i| i.into()));

        self.inner.add_all(individuals)
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        self.acquire().on_add.entry(self.generation).or_default().push((&individual).into());

        self.inner.add(individual)
    }

    fn on_generation(&mut self, statistics: &HeuristicStatistics) {
        self.generation = statistics.generation;
        self.acquire().generation = statistics.generation;

        let individuals_data = self.inner.all().map(|individual| individual.into()).collect::<Vec<_>>();
        // NOTE this is not exactly how footprint is calculated in production code.
        // In the production version, approximation of the footprint is used to avoid iterating over
        // all individuals in the population on generation update.
        let individuals_footprint = individuals_data
            .iter()
            .filter_map(|data| match data {
                ObservationData::Vrp(shadow) => Some(shadow),
                _ => None,
            })
            .fold(FootprintState::default(), |mut footprint, shadow| {
                footprint.apply(shadow);
                footprint
            });
        self.acquire().on_generation.insert(self.generation, (individuals_footprint, individuals_data));

        self.acquire().population_state.insert(self.generation, get_population_state(&self.inner));

        self.inner.on_generation(statistics)
    }

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.inner.cmp(a, b)
    }

    fn select(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        Box::new(self.inner.select().inspect(|&individual| {
            self.acquire().on_select.entry(self.generation).or_default().push(individual.into());
        }))
    }

    fn ranked(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        self.inner.ranked()
    }

    fn all(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        self.inner.all()
    }

    fn size(&self) -> usize {
        self.inner.size()
    }

    fn selection_phase(&self) -> SelectionPhase {
        self.inner.selection_phase()
    }
}

/// Creates info logger proxy to catch dynamic heuristic state.
pub fn create_info_logger_proxy(inner: InfoLogger) -> InfoLogger {
    Arc::new(move |msg| {
        if let Some(state) = HyperHeuristicState::try_parse_all(msg) {
            EXPERIMENT_DATA.lock().unwrap().heuristic_state = state;
        } else {
            (inner)(msg)
        }
    })
}
