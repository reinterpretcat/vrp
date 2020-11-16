use super::*;
use crate::helpers::SolomonBuilder;
use crate::solomon::{SolomonProblem, SolomonSolution};
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::solver::mutation::{Recreate, RecreateWithCheapest};
use vrp_core::solver::population::Elitism;
use vrp_core::solver::RefinementContext;
use vrp_core::utils::DefaultRandom;

#[test]
fn can_write_solomon_solution() {
    let random = Arc::new(DefaultRandom::default());
    let problem = Arc::new(
        SolomonBuilder::new()
            .set_title("Trivial problem")
            .set_vehicle((1, 10))
            .add_customer((0, 0, 0, 0, 0, 1000, 1))
            .add_customer((1, 1, 0, 1, 5, 1000, 5))
            .build()
            .read_solomon()
            .unwrap(),
    );

    let mut refinement_ctx =
        RefinementContext::new(problem.clone(), Box::new(Elitism::new(problem.clone(), random.clone(), 1, 1)), None);

    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };
    RecreateWithCheapest::default()
        .run(&mut refinement_ctx, InsertionContext::new(problem.clone(), random))
        .solution
        .to_solution(problem.extras.clone())
        .write_solomon(writer)
        .unwrap();

    assert_eq!(buffer, "Solution\nRoute 1: 1\n");
}
