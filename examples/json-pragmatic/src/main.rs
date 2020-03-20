use std::fs::File;
use std::process;
use std::sync::Arc;
use vrp_core::construction::states::InsertionContext;
use vrp_core::models::matrix::{AdjacencyMatrixDecipher, SparseMatrix};
use vrp_core::models::{Problem, Solution};
use vrp_core::utils::DefaultRandom;
use vrp_pragmatic::json::problem::PragmaticProblem;
use vrp_solver::SolverBuilder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base_path = args.get(1).unwrap_or_else(|| panic!("Please set a proper path to example data"));
    run_examples(base_path.as_str());
}

fn run_examples(base_path: &str) {
    let names: Vec<(_, Option<Vec<&str>>)> = vec![
        ("basics/break.basic", None),
        ("basics/multi-day.basic", None),
        ("basics/multi-job.basic", None),
        ("basics/multi-job.mixed", None),
        ("basics/multi-objective.balance-load", None),
        ("basics/multi-objective.default", None),
        ("basics/multi-objective.goal", None),
        ("basics/profiles.basic", Some(vec!["basics/profiles.basic.matrix.car", "basics/profiles.basic.matrix.truck"])),
        ("basics/relation-strict.basic", None),
        ("basics/relation-any.basic", None),
        ("basics/reload.basic", None),
        ("basics/reload.multi", None),
        ("simple.basic", None),
        ("basics/skills.basic", None),
        ("basics/unassigned.unreachable", None),
    ];

    for (name, matrices) in names {
        let problem = open_file(format!["{}/{}.problem.json", base_path, name].as_str());

        let problem = Arc::new(
            if let Some(matrices) = matrices {
                let matrices =
                    matrices.iter().map(|path| open_file(format!["{}/{}.json", base_path, path].as_str())).collect();
                (problem, matrices).read_pragmatic()
            } else {
                problem.read_pragmatic()
            }
            .unwrap_or_else(|err| {
                eprintln!("Cannot read pragmatic problem: '{}'", err);
                process::exit(1);
            }),
        );

        let (solution, _, _) =
            SolverBuilder::default().with_max_generations(Some(100)).build().solve(problem.clone()).unwrap_or_else(
                || {
                    eprintln!("Cannot solve pragmatic problem");
                    process::exit(1);
                },
            );

        let solution = Arc::new(solution);

        // TODO use solution checker

        validate_with_matrix(&problem, &solution);
    }
}

fn open_file(path: &str) -> File {
    File::open(path).unwrap_or_else(|err| {
        eprintln!("Cannot open {} file: '{}'", path, err.to_string());
        process::exit(1);
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_run_examples() {
        run_examples("../json-pragmatic/data");
    }
}

fn validate_with_matrix(problem: &Arc<Problem>, solution: &Arc<Solution>) {
    let insertion_ctx = InsertionContext::new_from_solution(
        problem.clone(),
        (solution.clone(), None),
        Arc::new(DefaultRandom::default()),
    );

    let decipher = AdjacencyMatrixDecipher::new(problem.clone());
    let adjacency_matrix_orig = decipher.encode::<SparseMatrix>(&insertion_ctx.solution);
    let restored_solution = decipher.decode(&adjacency_matrix_orig);
    let adjacency_matrix_rst = decipher.encode::<SparseMatrix>(&restored_solution);

    assert_eq!(adjacency_matrix_rst.to_vvec(), adjacency_matrix_orig.to_vvec());
}
