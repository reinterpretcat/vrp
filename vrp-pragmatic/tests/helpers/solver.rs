use crate::checker::CheckerContext;
use crate::format::problem::{Matrix, PragmaticProblem, Problem};
use crate::format::solution::{create_solution, Solution};
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::models::Problem as CoreProblem;
use vrp_core::models::Solution as CoreSolution;
use vrp_core::rosomaxa::evolution::TelemetryMode;
use vrp_core::solver::search::{Recreate, RecreateWithCheapest};
use vrp_core::solver::RefinementContext;
use vrp_core::solver::{create_elitism_population, Solver, VrpConfigBuilder};
use vrp_core::utils::{Environment, GenericError, Parallelism};

/// Runs solver with cheapest insertion heuristic.
pub fn solve_with_cheapest_insertion(problem: Problem, matrices: Option<Vec<Matrix>>) -> Solution {
    let environment = Arc::new(Environment::default());
    get_core_solution(problem, matrices, true, |problem: Arc<CoreProblem>| {
        let population = create_elitism_population(problem.goal.clone(), environment.clone());
        let refinement_ctx =
            RefinementContext::new(problem.clone(), Box::new(population), TelemetryMode::None, environment.clone());

        RecreateWithCheapest::new(environment.random.clone())
            .run(&refinement_ctx, InsertionContext::new(problem, environment))
            .into()
    })
}

/// Runs solver with default metaheuristic and default amount of generations.
pub fn solve_with_metaheuristic(problem: Problem, matrices: Option<Vec<Matrix>>) -> Solution {
    solve_with_metaheuristic_and_iterations(problem, matrices, 200)
}

/// Runs solver with default metaheuristic and specified amount of generations.
pub fn solve_with_metaheuristic_and_iterations(
    problem: Problem,
    matrices: Option<Vec<Matrix>>,
    generations: usize,
) -> Solution {
    solve(problem, matrices, generations, true)
}

/// Runs solver with default metaheuristic and specified amount of generations without feasibility check.
pub fn solve_with_metaheuristic_and_iterations_without_check(
    problem: Problem,
    matrices: Option<Vec<Matrix>>,
    generations: usize,
) -> Solution {
    solve(problem, matrices, generations, false)
}

pub fn solve(problem: Problem, matrices: Option<Vec<Matrix>>, generations: usize, perform_check: bool) -> Solution {
    // NOTE: hardcode cpus to guarantee rosomaxa population algorithm is used
    const AVAILABLE_CPUS: usize = 4;

    get_core_solution(problem, matrices, perform_check, |problem: Arc<CoreProblem>| {
        let environment =
            Arc::new(Environment { parallelism: Parallelism::new_with_cpus(AVAILABLE_CPUS), ..Environment::default() });

        VrpConfigBuilder::new(problem.clone())
            .set_environment(environment)
            .prebuild()
            .expect("cannot prebuild vrp configuration")
            .with_max_generations(Some(generations))
            .build()
            .map(|config| Solver::new(problem, config))
            .expect("cannot build solver")
            .solve()
            .expect("cannot solve the problem")
    })
}

fn get_core_problem(problem: Problem, matrices: Option<Vec<Matrix>>) -> Arc<CoreProblem> {
    Arc::new(
        if let Some(matrices) = matrices { (problem, matrices).read_pragmatic() } else { problem.read_pragmatic() }
            .unwrap(),
    )
}

fn get_core_solution<F: FnOnce(Arc<CoreProblem>) -> CoreSolution>(
    problem: Problem,
    matrices: Option<Vec<Matrix>>,
    perform_check: bool,
    solve_func: F,
) -> Solution {
    let format_problem = problem.clone();
    let format_matrices = matrices.clone();

    let core_problem = get_core_problem(problem, matrices);

    let core_solution = solve_func(core_problem.clone());

    let format_solution = sort_all_data(create_solution(&core_problem, &core_solution, &Default::default()));

    if perform_check {
        if let Some(errs) =
            CheckerContext::new(core_problem, format_problem.clone(), format_matrices, format_solution.clone())
                .and_then(|ctx| ctx.check())
                .err()
        {
            panic!(
                "check failed: '{}', problem: {:?}, solution: {:?}",
                GenericError::join_many(&errs, "\n"),
                format_problem,
                format_solution
            );
        }
    }

    sort_all_data(format_solution)
}

/// Sorts some solution properties in lexicographical order to simplify test assertions.
fn sort_all_data(solution: Solution) -> Solution {
    let mut solution = solution;

    solution.tours.sort_by(|a, b| a.vehicle_id.cmp(&b.vehicle_id));

    if let Some(ref mut unassigned) = solution.unassigned {
        unassigned.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    }

    solution
}
