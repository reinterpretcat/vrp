use crate::EXPERIMENT_DATA;
use rosomaxa::example::FitnessFn;
use rosomaxa::prelude::Float;
use std::sync::Arc;

/// Returns a fitness function which can be used to evaluate population state.
/// It is not actual fitness function, but a wrapper around [crate::FootprintState].
pub fn get_population_fitness_fn(generation: usize) -> FitnessFn {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .and_then(|data| data.on_generation.get(&generation).map(|(footprint, _)| footprint.clone()))
        .map(|footprint| {
            Arc::new(move |input: &[Float]| {
                if let &[from, to] = input {
                    footprint.get(from as usize, to as usize) as Float
                } else {
                    panic!("expected 2 input values which encode from/to edge weights");
                }
            })
        })
        .expect("cannot get data from EXPERIMENT_DATA")
}

/// Returns a description of population state.
pub fn get_population_desc(generation: usize) -> String {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .and_then(|data| {
            data.on_generation.get(&generation).map(|(footprint, individuals)| {
                format!("total [{}], known edges [{}]", individuals.len(), footprint.desc())
            })
        })
        .expect("cannot get data from EXPERIMENT_DATA")
}
