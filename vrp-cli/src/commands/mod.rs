use clap::{Arg, ArgMatches, Command};

pub mod analyze;
pub mod check;
pub mod generate;
pub mod import;
pub mod solve;

use std::fs::File;
use std::io::{stdout, BufReader, BufWriter, Write};
use std::process;
use std::str::FromStr;
use vrp_cli::extensions::check::check_pragmatic_solution;

pub(crate) fn create_write_buffer(out_file: Option<File>) -> BufWriter<Box<dyn Write>> {
    if let Some(out_file) = out_file {
        BufWriter::new(Box::new(out_file))
    } else {
        BufWriter::new(Box::new(stdout()))
    }
}

fn open_file(path: &str, description: &str) -> File {
    File::open(path).unwrap_or_else(|err| {
        eprintln!("cannot open {description} file '{path}': '{err}'");
        process::exit(1);
    })
}

fn create_file(path: &str, description: &str) -> File {
    File::create(path).unwrap_or_else(|err| {
        eprintln!("cannot create {description} file '{path}': '{err}'");
        process::exit(1);
    })
}

fn parse_float_value<T: FromStr<Err = std::num::ParseFloatError>>(
    matches: &ArgMatches,
    arg_name: &str,
    arg_desc: &str,
) -> Result<Option<T>, String> {
    matches
        .get_one::<String>(arg_name)
        .map(|arg| {
            arg.parse::<T>().map_err(|err| format!("cannot get float value, error: '{err}': '{arg_desc}'")).map(Some)
        })
        .unwrap_or(Ok(None))
}

fn parse_int_value<T: FromStr<Err = std::num::ParseIntError>>(
    matches: &ArgMatches,
    arg_name: &str,
    arg_desc: &str,
) -> Result<Option<T>, String> {
    matches
        .get_one::<String>(arg_name)
        .map(|arg| {
            arg.parse::<T>().map_err(|err| format!("cannot get integer value, error: '{err}': '{arg_desc}'")).map(Some)
        })
        .unwrap_or(Ok(None))
}

fn check_solution(
    matches: &ArgMatches,
    input_format: &str,
    problem_arg_name: &str,
    solution_arg_name: &str,
    matrix_arg_name: &str,
) -> Result<(), String> {
    let problem_files = matches
        .get_many::<String>(problem_arg_name)
        .map(|paths| paths.map(|path| BufReader::new(open_file(path, "problem"))).collect::<Vec<_>>());
    let solution_file =
        matches.get_one::<String>(solution_arg_name).map(|path| BufReader::new(open_file(path, "solution")));
    let matrix_files = matches
        .get_many::<String>(matrix_arg_name)
        .map(|paths| paths.map(|path| BufReader::new(open_file(path, "routing matrix"))).collect());

    match (input_format, problem_files, solution_file) {
        ("pragmatic", Some(mut problem_files), Some(solution_file)) if problem_files.len() == 1 => {
            check_pragmatic_solution(problem_files.swap_remove(0), solution_file, matrix_files)
        }
        ("pragmatic", _, _) => {
            Err(vec!["pragmatic format expects one problem, one solution file, and optionally matrices".to_string()])
        }
        _ => Err(vec![format!("unknown format: '{input_format}'")]),
    }
    .map_err(|err| format!("checker found {} errors:\n{}", err.len(), err.join("\n")))
}
