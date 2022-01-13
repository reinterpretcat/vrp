//! An examples of **Vehicle Routing Problem** solver usage.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use vrp_pragmatic::checker::CheckerContext;
use vrp_pragmatic::core::models::{Problem as CoreProblem, Solution as CoreSolution};
use vrp_pragmatic::core::prelude::*;
use vrp_pragmatic::core::solver::SolverBuilder;
use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem, Matrix, PragmaticProblem, Problem};
use vrp_pragmatic::format::solution::{deserialize_solution, PragmaticSolution, Solution};
use vrp_pragmatic::format::FormatError;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base_path = args.get(1).expect("please set a proper path to example data");
    run_examples(base_path.as_str());
}

fn run_examples(base_path: &str) {
    let names: Vec<(_, Option<Vec<&str>>)> = vec![
        ("basics/break.basic", None),
        ("basics/dispatch.basic", None),
        ("basics/multi-day.basic", None),
        ("basics/multi-job.basic", None),
        ("basics/multi-job.mixed", None),
        ("basics/multi-objective.balance-load", None),
        ("basics/multi-objective.default", None),
        ("basics/multi-objective.maximize-value", None),
        ("basics/priorities.value", None),
        ("basics/profiles.basic", Some(vec!["basics/profiles.basic.matrix.car", "basics/profiles.basic.matrix.truck"])),
        ("basics/relation-strict.basic", None),
        ("basics/relation-any.basic", None),
        ("basics/reload.basic", None),
        ("basics/reload.multi", None),
        ("simple.basic", None),
        ("simple.index", Some(vec!["simple.basic.matrix"])),
        ("basics/skills.basic", None),
        ("basics/unassigned.unreachable", None),
    ];

    for (name, matrices) in names {
        let environment = Arc::new(Environment::default());
        let problem = get_pragmatic_problem(base_path, name);

        let (core_problem, problem, matrices) = if let Some(matrices) = matrices {
            let matrices = matrices
                .iter()
                .map(|path| deserialize_matrix(open_file(format!["{}/{}.json", base_path, path].as_str())))
                .collect::<Result<Vec<Matrix>, _>>()
                .unwrap_or_else(|errors| {
                    panic!("cannot read matrix: {}", FormatError::format_many(errors.as_slice(), "\t\n"))
                });
            ((problem.clone(), matrices.clone()).read_pragmatic(), problem, Some(matrices))
        } else {
            (problem.clone().read_pragmatic(), problem, None)
        };

        let core_problem = Arc::new(core_problem.unwrap_or_else(|errors| {
            panic!("cannot read pragmatic problem: {}", FormatError::format_many(errors.as_slice(), "\t\n"))
        }));

        let (solution, cost, _) = SolverBuilder::new(core_problem.clone(), environment)
            .with_max_generations(Some(100))
            .build()
            .unwrap_or_else(|err| panic!("cannot build solver: {}", err))
            .solve()
            .unwrap_or_else(|err| panic!("cannot solver problem: {}", err));

        let solution = get_pragmatic_solution(&core_problem, &solution, cost);

        if let Err(err) = CheckerContext::new(core_problem, problem, matrices, solution).and_then(|ctx| ctx.check()) {
            panic!("unfeasible solution in '{}':\n'{}'", name, err.join("\n"));
        }
    }
}

fn open_file(path: &str) -> BufReader<File> {
    println!("Reading '{}'", path);
    BufReader::new(File::open(path).unwrap_or_else(|err| panic!("cannot open {} file: '{}'", path, err.to_string())))
}

fn get_pragmatic_problem(base_path: &str, name: &str) -> Problem {
    deserialize_problem(open_file(format!["{}/{}.problem.json", base_path, name].as_str())).unwrap()
}

fn get_pragmatic_solution(problem: &CoreProblem, solution: &CoreSolution, cost: f64) -> Solution {
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };

    (solution, cost).write_pragmatic_json(problem, writer).expect("cannot write pragmatic solution");

    deserialize_solution(BufReader::new(buffer.as_bytes())).expect("cannot deserialize solution")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_run_examples() {
        run_examples("../data/pragmatic");
    }
}
