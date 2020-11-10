use super::super::rand::prelude::SliceRandom;
use super::*;
use crate::algorithms::gsom::{Input, Network, Storage};
use crate::construction::heuristics::*;
use crate::models::Problem;
use crate::solver::SOLUTION_WEIGHTS_KEY;
use crate::utils::{as_mut, Random};
use std::cell::Ref;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

/// Implements custom algorithm, code name Routing Optimizations with Self Organizing
/// Maps And eXtrAs (pronounced as "rosomaha", from russian "росомаха" - "wolverine").
pub struct RosomaxaPopulation {
    problem: Arc<Problem>,
    random: Arc<dyn Random + Send + Sync>,
    elite: DominancePopulation,
    network: Network<IndividualInput, IndividualStorage>,
    populations: Vec<Rc<DominancePopulation>>,
    selection_size: usize,
}

impl Population for RosomaxaPopulation {
    fn add_all(&mut self, individuals: Vec<Individual>, statistics: &Statistics) -> bool {
        individuals.into_iter().fold(false, |acc, individual| acc || self.add(individual, statistics))
    }

    fn add(&mut self, individual: Individual, statistics: &Statistics) -> bool {
        let is_improvement =
            if self.is_improvement(&individual) { self.elite.add(individual.deep_copy(), statistics) } else { false };

        // TODO use statistics to control network parameters

        self.network.train(IndividualInput::new(individual));

        self.update();

        is_improvement
    }

    fn cmp(&self, a: &Individual, b: &Individual) -> Ordering {
        self.elite.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Individual> + 'a> {
        // TODO return individuals from elite (exploitation) and network (exploration)
        //      use statistics from add to control exploitation vs exploration balance
        //      use hits to select candidates for mating depending on evolution progress (statistics)

        // NOTE we always promote 2 elements from elite and 2 from each population in the network.
        //      2 is not a magic number: dominance population always promotes the best individual
        //      as first, all others are selected with equal probability then.
        //      If calling site selects always less than 3 elements, then the algorithm should not be used.

        Box::new(
            self.elite
                .select()
                .take(2)
                .chain(self.populations.iter().flat_map(|population| population.select().take(2)))
                .take(self.selection_size),
        )
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Individual, usize)> + 'a> {
        // NOTE return only elite
        self.elite.ranked()
    }

    fn size(&self) -> usize {
        self.elite.size()
    }
}

impl RosomaxaPopulation {
    /// Creates a new instance of `RosomaxaPopulation`.
    pub fn new(
        problem: Arc<Problem>,
        random: Arc<dyn Random + Send + Sync>,
        initials: [InsertionContext; 4],
        selection_size: usize,
        max_elite_size: usize,
        max_node_size: usize,
        spread_factor: f64,
        reduction_factor: f64,
        distribution_factor: f64,
        learning_rate: f64,
    ) -> Result<Self, ()> {
        // NOTE see note at selection method implementation
        if max_elite_size < 2 || max_node_size < 2 || selection_size < 4 {
            return Err(());
        }

        let [a, b, c, d] = initials;

        Ok(Self {
            problem: problem.clone(),
            random: random.clone(),
            elite: DominancePopulation::new(problem.clone(), random.clone(), max_elite_size, max_elite_size),
            network: Network::new(
                [IndividualInput::new(a), IndividualInput::new(b), IndividualInput::new(c), IndividualInput::new(d)],
                spread_factor,
                reduction_factor,
                distribution_factor,
                learning_rate,
                Box::new(move || IndividualStorage {
                    population: Rc::new(DominancePopulation::new(
                        problem.clone(),
                        random.clone(),
                        max_node_size,
                        max_node_size,
                    )),
                }),
            ),
            populations: vec![],
            selection_size,
        })
    }

    fn is_improvement(&self, individual: &Individual) -> bool {
        if let Some((best, _)) = self.elite.ranked().next() {
            if self.elite.cmp(individual, best) != Ordering::Greater {
                return !is_same_fitness(individual, best, self.problem.objective.as_ref());
            }
        } else {
            return true;
        }

        false
    }

    fn update(&mut self) {
        // NOTE we keep track of actual populations and randomized order to keep selection algorithm simple
        self.populations = self
            .network
            .get_storages(|_| true)
            .map(|storage| Ref::map(storage, |storage| &storage.population).clone())
            .collect();

        self.populations.shuffle(&mut self.random.get_rng());
    }
}

struct IndividualInput {
    individual: InsertionContext,
}

impl IndividualInput {
    pub fn new(individual: InsertionContext) -> Self {
        let weights = IndividualInput::get_weights(&individual);

        let mut individual = individual;
        individual.solution.state.insert(SOLUTION_WEIGHTS_KEY, Arc::new(weights));

        Self { individual }
    }

    fn get_weights(individual: &InsertionContext) -> Vec<f64> {
        vec![
            get_max_load_variance(individual),
            get_customers_deviation(individual),
            get_duration_mean(individual),
            get_distance_mean(individual),
            get_distance_gravity_mean(individual),
        ]
    }
}

impl Input for IndividualInput {
    fn weights(&self) -> &[f64] {
        self.individual
            .solution
            .state
            .get(&SOLUTION_WEIGHTS_KEY)
            .and_then(|s| s.downcast_ref::<Vec<f64>>())
            .unwrap()
            .as_slice()
    }
}

struct IndividualStorage {
    population: Rc<DominancePopulation>,
}

impl IndividualStorage {
    fn get_population_mut(&mut self) -> &mut DominancePopulation {
        // NOTE use black magic here to avoid RefCell, should not break memory safety guarantee
        unsafe { as_mut(self.population.deref()) }
    }
}

impl Storage for IndividualStorage {
    type Item = IndividualInput;

    fn add(&mut self, input: Self::Item) {
        self.get_population_mut().add(input.individual, &Statistics::default());
    }

    fn drain(&mut self) -> Vec<Self::Item> {
        self.get_population_mut().drain().into_iter().map(|individual| IndividualInput { individual }).collect()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        // NOTE as weights are not normalized, apply standardization using relative change: D = |x - y| / max(|x|, |y|)
        a.iter()
            .zip(b.iter())
            .fold(0., |acc, (a, b)| {
                let divider = a.abs().max(b.abs());
                let change = if compare_floats(divider, 0.) == Ordering::Equal { 0. } else { (a - b) / divider };

                acc + change * change
            })
            .sqrt()
    }
}
