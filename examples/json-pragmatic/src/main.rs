use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;
use std::{fs, process};
use vrp_core::construction::states::InsertionContext;
use vrp_core::models::matrix::{AdjacencyMatrixDecipher, SparseMatrix};
use vrp_core::models::{Problem, Solution};
use vrp_core::utils::DefaultRandom;
use vrp_pragmatic::json::problem::PragmaticProblem;
use vrp_pragmatic::json::solution::PragmaticSolution;
use vrp_solver::SolverBuilder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base_path = args.get(1).unwrap_or_else(|| panic!("Please set a proper path to example data"));
    run_examples(base_path.as_str());
}

fn run_examples(base_path: &str) {
    let names = vec![
        ("break.basic", None, true),
        ("multi-day.basic", None, true),
        ("multi-job.basic", None, true),
        ("multi-job.mixed", None, true),
        //("multi-objective.balance-activities", Some("simple.basic"), false),
        ("multi-objective.balance-load", Some("simple.basic"), false),
        ("multi-objective.default", Some("simple.basic"), false),
        ("multi-objective.goal", Some("simple.basic"), false),
        ("relation-strict.basic", None, true),
        ("relation-any.basic", None, true),
        ("reload.basic", None, true),
        ("reload.multi", None, true),
        ("simple.basic", None, true),
        ("skills.basic", None, true),
        ("unassigned.unreachable", None, true),
    ];

    for (name, matrix, has_existing_solution) in names {
        let problem = open_file(format!["{}/{}.problem.json", base_path, name].as_str());
        let matrices = vec![open_file(format!["{}/{}.matrix.json", base_path, matrix.unwrap_or(name)].as_str())];

        let problem = Arc::new((problem, matrices).read_pragmatic().unwrap_or_else(|err| {
            eprintln!("Cannot read pragmatic problem: '{}'", err);
            process::exit(1);
        }));

        let (solution, _, _) =
            SolverBuilder::default().with_max_generations(Some(100)).build().solve(problem.clone()).unwrap_or_else(
                || {
                    eprintln!("Cannot solve pragmatic problem");
                    process::exit(1);
                },
            );

        let solution = Arc::new(solution);

        if has_existing_solution {
            validate_with_existing(&problem, &solution, base_path, name);
        }
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

fn validate_with_existing(problem: &Arc<Problem>, solution: &Arc<Solution>, base_path: &str, name: &str) {
    let mut solution_serialized = String::new();
    let writer = unsafe { BufWriter::new(solution_serialized.as_mut_vec()) };
    solution.write_pragmatic(&problem, writer).ok().unwrap();

    let solution_expected = fs::read_to_string(format!["{}/{}.solution.json", base_path, name].as_str())
        .unwrap_or_else(|err| {
            eprintln!("Cannot read solution: '{}'", err);
            process::exit(1);
        });

    // TODO improve check and make it assertion
    if solution_serialized != solution_expected {
        println!("Solutions are different for {}", name);
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
