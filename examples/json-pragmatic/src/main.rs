//! An examples of **Vehicle Routing Problem** solver usage.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::Arc;
use vrp_core::models::{Problem as CoreProblem, Solution as CoreSolution};
use vrp_core::solver::Builder;
use vrp_pragmatic::checker::CheckerContext;
use vrp_pragmatic::format::problem::{deserialize_problem, PragmaticProblem, Problem};
use vrp_pragmatic::format::solution::{deserialize_solution, PragmaticSolution, Solution};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base_path = args.get(1).expect("please set a proper path to example data");
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
        let problem = BufReader::new(open_file(format!["{}/{}.problem.json", base_path, name].as_str()));

        let problem = Arc::new(
            if let Some(matrices) = matrices {
                let matrices = matrices
                    .iter()
                    .map(|path| BufReader::new(open_file(format!["{}/{}.json", base_path, path].as_str())))
                    .collect();
                (problem, matrices).read_pragmatic()
            } else {
                problem.read_pragmatic()
            }
            .unwrap_or_else(|errors| {
                panic!(
                    "cannot read pragmatic problem:\n{}",
                    errors.iter().map(|err| err.to_string()).collect::<Vec<_>>().join("\t\n")
                )
            }),
        );

        let (solution, _) = Builder::default()
            .with_max_generations(Some(100))
            .with_problem(problem.clone())
            .build()
            .unwrap_or_else(|err| panic!("cannot build solver: {}", err))
            .solve()
            .unwrap_or_else(|err| panic!("cannot solver problem: {}", err));

        let solution = get_pragmatic_solution(&Arc::try_unwrap(problem).ok().unwrap(), &solution);
        let problem = get_pragmatic_problem(base_path, name);

        // TODO use matrices
        if let Err(err) = CheckerContext::new(problem, None, solution).check() {
            panic!("unfeasible solution in '{}': '{}'", name, err);
        }
    }
}

fn open_file(path: &str) -> File {
    println!("Reading '{}'", path);
    File::open(path).unwrap_or_else(|err| panic!(format!("cannot open {} file: '{}'", path, err.to_string())))
}

fn get_pragmatic_problem(base_path: &str, name: &str) -> Problem {
    deserialize_problem(BufReader::new(open_file(format!["{}/{}.problem.json", base_path, name].as_str()))).unwrap()
}

fn get_pragmatic_solution(problem: &CoreProblem, solution: &CoreSolution) -> Solution {
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };

    solution.write_pragmatic_json(&problem, writer).expect("cannot write pragmatic solution");

    deserialize_solution(BufReader::new(buffer.as_bytes())).expect("cannot deserialize solution")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_run_examples() {
        run_examples("../json-pragmatic/data");
    }
}
