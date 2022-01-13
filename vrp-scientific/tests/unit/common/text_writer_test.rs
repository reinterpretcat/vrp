use super::*;
use crate::helpers::SolomonBuilder;
use crate::solomon::{SolomonProblem, SolomonSolution};
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::solver::search::{Recreate, RecreateWithCheapest};
use vrp_core::solver::{ElitismPopulation, RefinementContext};
use vrp_core::utils::Environment;

#[test]
fn can_write_solomon_solution() {
    let environment = Arc::new(Environment::default());
    let problem = Arc::new(
        SolomonBuilder::new()
            .set_title("Trivial problem")
            .set_vehicle((1, 10))
            .add_customer((0, 0, 0, 0, 0, 1000, 1))
            .add_customer((1, 1, 0, 1, 5, 1000, 5))
            .build()
            .read_solomon(false)
            .unwrap(),
    );

    let mut refinement_ctx = RefinementContext::new(
        problem.clone(),
        Box::new(ElitismPopulation::new(problem.objective.clone(), environment.random.clone(), 1, 1)),
        environment.clone(),
    );

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    let solution = RecreateWithCheapest::new(environment.random.clone())
        .run(&mut refinement_ctx, InsertionContext::new(problem.clone(), environment))
        .solution
        .to_solution(problem.extras.clone());
    (&solution, 3.123456).write_solomon(writer).unwrap();

    assert_eq!(buffer, "Route 1: 1\nCost 3.12");
}
