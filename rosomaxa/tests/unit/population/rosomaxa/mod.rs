use super::maintenance::*;
use super::phase::*;
use super::selection::*;
use super::storage::*;
use super::*;
use crate::algorithms::gsom::{Coordinate, Storage, StorageFactory};
use crate::example::*;
use crate::helpers::example::create_example_objective;
use crate::{HeuristicSpeed, Random};

type RosomaxaType = Rosomaxa<VectorRosomaxaContext, VectorObjective, VectorSolution>;

#[derive(Clone)]
struct RelaxedSolution {
    weights: Vec<Float>,
    fitness: Float,
    infeasibility: Option<Float>,
}

impl HeuristicSolution for RelaxedSolution {
    fn fitness(&self) -> impl Iterator<Item = Float> {
        std::iter::once(self.fitness)
    }

    fn deep_copy(&self) -> Self {
        self.clone()
    }
}

impl Input for RelaxedSolution {
    fn weights(&self) -> &[Float] {
        self.weights.as_slice()
    }

    fn is_same(&self, other: &Self) -> bool {
        self.infeasibility.is_some() == other.infeasibility.is_some()
            && self.weights == other.weights
            && (self.infeasibility.is_none()
                || (self.infeasibility == other.infeasibility && self.fitness == other.fitness))
    }
}

struct RelaxedContext;

impl RosomaxaContext for RelaxedContext {
    type Solution = RelaxedSolution;

    fn on_change(&mut self, _: &[Self::Solution]) {}
}

impl RosomaxaSolution for RelaxedSolution {
    type Context = RelaxedContext;

    fn on_init(&mut self, _: &Self::Context) {}

    fn on_update(&mut self, _: &Self::Context) {}

    fn relaxed_violation(&self) -> Option<Float> {
        self.infeasibility
    }
}

#[derive(Clone)]
struct RelaxedTestObjective;

impl HeuristicObjective for RelaxedTestObjective {
    type Solution = RelaxedSolution;

    fn total_order(&self, left: &Self::Solution, right: &Self::Solution) -> Ordering {
        left.fitness.total_cmp(&right.fitness)
    }
}

impl Alternative for RelaxedTestObjective {
    fn maybe_new(&self, _: &dyn Random) -> Self {
        self.clone()
    }
}

#[derive(Clone)]
struct AlternativeRelaxedTestObjective {
    reverse: bool,
}

impl HeuristicObjective for AlternativeRelaxedTestObjective {
    type Solution = RelaxedSolution;

    fn total_order(&self, left: &Self::Solution, right: &Self::Solution) -> Ordering {
        if self.reverse { right.fitness.total_cmp(&left.fitness) } else { left.fitness.total_cmp(&right.fitness) }
    }
}

impl Alternative for AlternativeRelaxedTestObjective {
    fn maybe_new(&self, _: &dyn Random) -> Self {
        Self { reverse: true }
    }
}

#[test]
fn can_reject_invalid_config() {
    let invalid_configs = [
        RosomaxaConfig { initial_size: 0, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { max_network_size: 0, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { spread_factor: 0., ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { spread_factor: 1., ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { spread_factor: Float::NAN, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { distribution_factor: 0., ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { distribution_factor: 1., ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { distribution_factor: Float::NAN, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { exploration_ratio: -0.1, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { exploration_ratio: 1.1, ..RosomaxaConfig::new_with_defaults(4) },
        RosomaxaConfig { exploration_ratio: Float::NAN, ..RosomaxaConfig::new_with_defaults(4) },
    ];

    for config in invalid_configs {
        let result =
            Rosomaxa::new(VectorRosomaxaContext, create_example_objective(), Arc::new(Environment::default()), config);

        assert!(result.is_err());
    }
}

#[test]
fn can_accept_exploration_ratio_boundaries() {
    for exploration_ratio in [0., 1.] {
        let config = RosomaxaConfig { exploration_ratio, ..RosomaxaConfig::new_with_defaults(4) };
        let result =
            Rosomaxa::new(VectorRosomaxaContext, create_example_objective(), Arc::new(Environment::default()), config);

        assert!(result.is_ok());
    }
}

fn create_rosomaxa(initial_size: usize) -> RosomaxaType {
    create_rosomaxa_with_config(RosomaxaConfig { initial_size, ..RosomaxaConfig::new_with_defaults(4) })
}

fn create_rosomaxa_with_config(config: RosomaxaConfig) -> RosomaxaType {
    let env = Arc::new(Environment::default());
    let objective = create_example_objective();

    Rosomaxa::new(VectorRosomaxaContext, objective, env, config).unwrap()
}

mod maintenance_test;
mod population_test;
mod relaxed_test;
mod selection_test;
mod storage_test;
