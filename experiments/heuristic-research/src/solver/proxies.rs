use crate::*;
use rosomaxa::example::VectorSolution;
use rosomaxa::population::{Alternative, RosomaxaContext, RosomaxaSolution};
use rosomaxa::prelude::*;
use std::any::TypeId;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::MutexGuard;
use vrp_scientific::core::models::common::Shadow;
use vrp_scientific::core::prelude::InsertionContext;

/// Keeps track of all experiment data for visualization purposes.
#[derive(Default, Serialize, Deserialize)]
pub struct ExperimentData {
    /// Current generation.
    pub generation: usize,
    /// Called on new individuals addition.
    pub on_add: BTreeMap<usize, Vec<ObservationData>>,
    /// Called on individual selection.
    pub on_select: BTreeMap<usize, Vec<ObservationData>>,
    /// Population observations and occasional aggregate VRP footprints captured on generation.
    pub on_generation: BTreeMap<usize, (FootprintState, Vec<ObservationData>)>,
    /// Population size at a recorded generation.
    #[serde(default)]
    pub population_sizes: BTreeMap<usize, usize>,
    /// Population phase at a recorded generation.
    #[serde(default)]
    pub population_phases: BTreeMap<usize, String>,
    /// Keeps track of population state at specific generation.
    pub population_state: BTreeMap<usize, PopulationState>,
    /// Keeps track of heuristic state at specific generation.
    pub heuristic_state: HyperHeuristicState,
    /// Requested generation limit.
    #[serde(default)]
    pub max_generations: usize,
    /// Distance between retained visualization snapshots.
    #[serde(default = "default_recording_interval")]
    pub recording_interval: usize,
}

impl ExperimentData {
    /// Clears all stored data.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Prepares storage for a new run while limiting the number of expensive visualization snapshots.
    pub fn configure(&mut self, max_generations: usize) {
        const MAX_SNAPSHOTS: usize = 250;

        self.clear();
        self.max_generations = max_generations;
        self.recording_interval = max_generations.div_ceil(MAX_SNAPSHOTS).max(1);
    }
}

fn default_recording_interval() -> usize {
    1
}

impl<'a> TryFrom<&'a str> for ExperimentData {
    type Error = String;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        // Check if this is telemetry CSV format (contains "TELEMETRY" somewhere in the content)
        // Extract telemetry section if present, otherwise try JSON
        if let Some(telemetry_start) = value.find("TELEMETRY") {
            // Extract everything from TELEMETRY onwards
            let telemetry_content = &value[telemetry_start..];

            // Parse telemetry CSV using existing parser
            let heuristic_state = HyperHeuristicState::try_parse_all(telemetry_content)
                .ok_or_else(|| "Failed to parse telemetry data".to_string())?;

            // Find max generation from telemetry data
            let generation = heuristic_state.heuristic_states.keys().copied().max().unwrap_or(0);

            return Ok(ExperimentData { heuristic_state, generation, ..Default::default() });
        }

        // Try parsing as JSON
        serde_json::from_str(value).map_err(|err| format!("cannot deserialize experiment data: {err}"))
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
    max_generations: usize,
    recording_interval: usize,
    footprint_interval: usize,
    progress_interval: usize,
    logger: InfoLogger,
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
    pub fn new(inner: P, max_generations: usize, logger: InfoLogger) -> Self {
        const MAX_VRP_FOOTPRINTS: usize = 24;

        let recording_interval = {
            let mut data = EXPERIMENT_DATA.lock().unwrap();
            data.configure(max_generations);
            data.recording_interval
        };

        Self {
            generation: 0,
            max_generations,
            recording_interval,
            footprint_interval: max_generations.div_ceil(MAX_VRP_FOOTPRINTS).max(1),
            progress_interval: max_generations.div_ceil(100).max(1),
            logger,
            inner,
            _phantom: Default::default(),
        }
    }

    fn acquire(&self) -> MutexGuard<'_, ExperimentData> {
        EXPERIMENT_DATA.lock().unwrap()
    }

    fn should_record(&self, generation: usize) -> bool {
        generation == 0 || generation == self.max_generations || generation.is_multiple_of(self.recording_interval)
    }

    fn should_record_footprint(&self, generation: usize) -> bool {
        generation == 0 || generation == self.max_generations || generation.is_multiple_of(self.footprint_interval)
    }

    fn get_function_observation(solution: &S) -> Option<ObservationData> {
        if TypeId::of::<S>() != TypeId::of::<VectorSolution>() {
            return None;
        }

        // SAFETY: type id check above ensures that S-type is the right one.
        let solution = unsafe { std::mem::transmute::<&S, &VectorSolution>(solution) };
        let fitness = solution.fitness().next().unwrap_or_default();

        Some(ObservationData::Function(DataPoint3D(solution.data[0], fitness, solution.data[1])))
    }

    fn apply_vrp_footprint(footprint: &mut FootprintState, solution: &S) {
        if TypeId::of::<S>() == TypeId::of::<InsertionContext>() {
            // SAFETY: type id check above ensures that S-type is the right one.
            let insertion_ctx = unsafe { std::mem::transmute::<&S, &InsertionContext>(solution) };
            footprint.apply_shadow(&Shadow::from(insertion_ctx));
        }
    }

    fn report_progress(&self, statistics: &HeuristicStatistics) {
        if statistics.generation != 1
            && statistics.generation != self.max_generations
            && !statistics.generation.is_multiple_of(self.progress_interval)
        {
            return;
        }

        let phase = format!("{:?}", self.inner.selection_phase()).to_lowercase();
        let fitness = self
            .inner
            .ranked()
            .next()
            .map(|solution| solution.fitness().map(|value| format!("{value:.3}")).collect::<Vec<_>>().join(","))
            .unwrap_or_default();

        (self.logger)(&format!(
            "EXPERIMENT_PROGRESS|{}|{}|{}|{}",
            statistics.generation, self.max_generations, phase, fitness
        ));
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
        if self.should_record(self.generation) {
            let observations = individuals.iter().filter_map(Self::get_function_observation).collect::<Vec<_>>();
            if !observations.is_empty() {
                self.acquire().on_add.entry(self.generation).or_default().extend(observations);
            }
        }

        self.inner.add_all(individuals)
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        if self.should_record(self.generation)
            && let Some(observation) = Self::get_function_observation(&individual)
        {
            self.acquire().on_add.entry(self.generation).or_default().push(observation);
        }

        self.inner.add(individual)
    }

    fn on_generation(&mut self, statistics: &HeuristicStatistics) {
        self.generation = statistics.generation;
        self.acquire().generation = statistics.generation;

        let should_record = self.should_record(self.generation);
        let should_record_footprint =
            TypeId::of::<S>() == TypeId::of::<InsertionContext>() && self.should_record_footprint(self.generation);

        if should_record || should_record_footprint {
            let mut footprint = FootprintState::default();
            let observations = if should_record {
                self.inner.iter().filter_map(Self::get_function_observation).collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if should_record_footprint {
                // This exact aggregate is intentionally sampled sparsely because it scans every stored solution.
                self.inner.iter().for_each(|individual| Self::apply_vrp_footprint(&mut footprint, individual));
            }

            let mut data = self.acquire();
            if !observations.is_empty() || should_record_footprint {
                data.on_generation.insert(self.generation, (footprint, observations));
            }
            if should_record {
                let phase = format!("{:?}", self.inner.selection_phase()).to_lowercase();
                data.population_sizes.insert(self.generation, self.inner.size());
                data.population_phases.insert(self.generation, phase);
                data.population_state.insert(self.generation, get_population_state(&self.inner));
            }
        }

        self.report_progress(statistics);

        self.inner.on_generation(statistics)
    }

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.inner.cmp(a, b)
    }

    fn select(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        Box::new(self.inner.select().inspect(|&individual| {
            if self.should_record(self.generation)
                && let Some(observation) = Self::get_function_observation(individual)
            {
                self.acquire().on_select.entry(self.generation).or_default().push(observation);
            }
        }))
    }

    fn ranked(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        self.inner.ranked()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &'_ Self::Individual> + '_> {
        self.inner.iter()
    }

    fn into_iter(self: Box<Self>) -> Box<dyn Iterator<Item = Self::Individual>> {
        Box::new(self.inner).into_iter()
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
