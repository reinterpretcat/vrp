#[cfg(test)]
#[path = "../../tests/unit/commands/check_test.rs"]
mod check_test;

use super::*;
use std::io::BufReader;
use std::process;
use vrp_cli::extensions::check::check_pragmatic_solution;

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "problem-file";
const SOLUTION_ARG_NAME: &str = "solution-file";
const MATRIX_ARG_NAME: &str = "matrix";

pub fn get_check_app<'a, 'b>() -> App<'a, 'b> {
    App::new("check")
        .about("Provides the way to check solution feasibility")
        .arg(
            Arg::with_name(FORMAT_ARG_NAME)
                .help("Specifies input type")
                .required(true)
                .possible_values(&["pragmatic"])
                .index(1),
        )
        .arg(
            Arg::with_name(PROBLEM_ARG_NAME)
                .help("Sets input files which contain a VRP definition")
                .short("p")
                .long(PROBLEM_ARG_NAME)
                .required(true)
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name(SOLUTION_ARG_NAME)
                .help("Sets solution file")
                .short("s")
                .long(SOLUTION_ARG_NAME)
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(MATRIX_ARG_NAME)
                .help("Specifies path to file with routing matrix")
                .short("m")
                .long(MATRIX_ARG_NAME)
                .multiple(true)
                .required(false)
                .takes_value(true),
        )
}

pub fn run_check(matches: &ArgMatches) {
    let input_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let problem_files = matches
        .values_of(PROBLEM_ARG_NAME)
        .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "problem"))).collect::<Vec<_>>());
    let solution_file = matches.value_of(SOLUTION_ARG_NAME).map(|path| BufReader::new(open_file(path, "solution")));
    let matrix_files = matches
        .values_of(MATRIX_ARG_NAME)
        .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "routing matrix"))).collect());

    let result = match (input_format, problem_files, solution_file) {
        ("pragmatic", Some(mut problem_files), Some(solution_file)) if problem_files.len() == 1 => {
            check_pragmatic_solution(problem_files.swap_remove(0), solution_file, matrix_files)
        }
        ("pragmatic", _, _) => {
            Err(vec!["pragmatic format expects one problem, one solution file, and optionally matrices".to_string()])
        }
        _ => Err(vec![format!("unknown format: '{}'", input_format)]),
    };

    if let Err(err) = result {
        eprintln!("checker found {} errors:\n{}", err.len(), err.join("\n"));
        process::exit(1);
    }
}
