use super::*;
use std::io::BufReader;
use std::process;
use vrp_pragmatic::checker::CheckerContext;
use vrp_pragmatic::format::problem::deserialize_problem;
use vrp_pragmatic::format::solution::deserialize_solution;

pub const FORMAT_ARG_NAME: &str = "FORMAT";
pub const PROBLEM_ARG_NAME: &str = "problem-files";
pub const SOLUTION_ARG_NAME: &str = "solution-file";

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
}

pub fn run_check(matches: &ArgMatches) {
    let input_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let problem_files = matches
        .values_of(PROBLEM_ARG_NAME)
        .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "problem"))).collect::<Vec<_>>());
    let solution_file = matches.value_of(SOLUTION_ARG_NAME).map(|path| BufReader::new(open_file(path, "solution")));

    let result = match (input_format, problem_files, solution_file) {
        ("pragmatic", Some(mut problem_files), Some(solution_file)) if problem_files.len() == 1 => {
            // TODO support matrix
            let problem_file = problem_files.swap_remove(0);

            deserialize_problem(problem_file)
                .into_iter()
                .zip(deserialize_solution(solution_file).into_iter())
                .map(|(problem, solution)| CheckerContext::new(problem, None, solution).check())
                .next()
                .expect("Cannot deserialize problem or solution")
        }
        ("pragmatic", _, _) => Err("pragmatic format expects one problem and one solution file".to_string()),
        _ => Err(format!("unknown format: '{}'", input_format)),
    };

    if let Err(err) = result {
        eprintln!("{}", err);
        process::exit(1);
    }
}
