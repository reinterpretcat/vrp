use super::*;
use crate::helpers::SolomonBuilder;
use crate::solomon::{SolomonProblem, SolomonSolution};
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::rosomaxa::evolution::TelemetryMode;
use vrp_core::solver::search::{Recreate, RecreateWithCheapest};
use vrp_core::solver::{ElitismPopulation, RefinementContext};
use vrp_core::utils::Environment;

#[test]
fn can_write_solomon_solution() {
    let environment = Arc::new(Environment::default());
    let problem = Arc::new(
        SolomonBuilder::default()
            .set_title("Trivial problem")
            .set_vehicle((1, 10))
            .add_customer((0, 0, 0, 0, 0, 1000, 1))
            .add_customer((1, 1, 0, 1, 5, 1000, 5))
            .build()
            .read_solomon(false)
            .unwrap(),
    );

    let refinement_ctx = RefinementContext::new(
        problem.clone(),
        Box::new(ElitismPopulation::new(problem.goal.clone(), environment.random.clone(), 1, 1)),
        TelemetryMode::None,
        environment.clone(),
    );

    let mut writer = BufWriter::new(Vec::new());
    let solution = RecreateWithCheapest::new(environment.random.clone())
        .run(&refinement_ctx, InsertionContext::new(problem, environment))
        .solution
        .into();
    (&solution, 3.123456).write_solomon(&mut writer).unwrap();
    let result = String::from_utf8(writer.into_inner().unwrap()).unwrap();

    assert_eq!(result, "Route 1: 1\nCost 3.12");
}
