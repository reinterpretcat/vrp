use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;
use std::{fs, process};
use vrp_pragmatic::json::problem::PragmaticProblem;
use vrp_pragmatic::json::solution::PragmaticSolution;
use vrp_solver::SolverBuilder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base_path = args.get(1).unwrap_or_else(|| panic!("Path proper path to examples!"));
    run_examples(base_path.as_str());
}

fn run_examples(base_path: &str) {
    let names = vec![
        "break.basic",
        "multi-day.basic",
        "multi-job.basic",
        "relation-strict.basic",
        "relation-tour.basic",
        "reload.basic",
        "reload.multi",
        "simple.basic",
        "skills.basic",
        "unassigned.unreachable",
    ];

    for name in names {
        let problem = open_file(format!["{}/{}.problem.json", base_path, name].as_str());
        let matrices = vec![open_file(format!["{}/{}.matrix.json", base_path, name].as_str())];

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
        run_examples("data");
    }
}
