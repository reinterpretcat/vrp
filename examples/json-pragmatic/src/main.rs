//! An examples of **Vehicle Routing Problem** solver usage.

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use vrp_pragmatic::json::problem::PragmaticProblem;
use vrp_solver::SolverBuilder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base_path = args.get(1).expect("Please set a proper path to example data");
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
                    "Cannot read pragmatic problem:\n{}",
                    errors.iter().map(|err| err.to_string()).collect::<Vec<_>>().join("\t\n")
                )
            }),
        );

        let _ = SolverBuilder::default()
            .with_max_generations(Some(100))
            .build()
            .solve(problem.clone())
            .expect("Cannot solve pragmatic problem");

        // TODO use solution checker
    }
}

fn open_file(path: &str) -> File {
    println!("Reading '{}'", path);
    File::open(path).unwrap_or_else(|err| panic!(format!("Cannot open {} file: '{}'", path, err.to_string())))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_run_examples() {
        run_examples("../json-pragmatic/data");
    }
}
