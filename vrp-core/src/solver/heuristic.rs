use super::*;
use crate::construction::heuristics::InsertionContext;
use crate::models::problem::ObjectiveCost;
use rosomaxa::heuristics::population::*;
use rosomaxa::prelude::*;

// TODO add type aliases for greedy, elitism, rosomaxa populations?

pub type GreedyPopulation = Greedy<ObjectiveCost, InsertionContext>;
pub type ElitismPopulation = Elitism<ObjectiveCost, InsertionContext>;
pub type RosomaxaPopulation = Rosomaxa<ObjectiveCost, InsertionContext>;

/// Gets default population selection size.
pub fn get_default_selection_size(environment: &Environment) -> usize {
    environment.parallelism.available_cpus().min(8)
}

/// Gets default population algorithm.
pub fn get_default_population(
    objective: Arc<ObjectiveCost>,
    environment: Arc<Environment>,
) -> Box<dyn Population + Send + Sync> {
    let selection_size = get_default_selection_size(environment.as_ref());
    if selection_size == 1 {
        Box::new(Greedy::new(objective, 1, None))
    } else {
        let config = RosomaxaConfig::new_with_defaults(selection_size);
        let population =
            Rosomaxa::new(objective, environment, config).expect("cannot create rosomaxa with default configuration");

        Box::new(population)
    }
}

/// Creates elitism population algorithm.
pub fn create_elitism_population(
    objective: Arc<ObjectiveCost>,
    environment: Arc<Environment>,
) -> Box<dyn Population + Sync + Send> {
    let selection_size = get_default_selection_size(environment.as_ref());
    Box::new(Elitism::new(objective, environment.random.clone(), 4, selection_size))
}

impl RosomaxaWeighted for InsertionContext {
    fn weights(&self) -> Vec<f64> {
        todo!()
    }
}

impl DominanceOrdered for InsertionContext {
    fn get_order(&self) -> &DominanceOrder {
        todo!()
    }

    fn set_order(&mut self, order: DominanceOrder) {
        todo!()
    }
}
