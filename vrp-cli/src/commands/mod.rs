use clap::{Arg, ArgMatches, Command};

pub mod analyze;
pub mod check;
pub mod generate;
pub mod import;
pub mod solve;

use std::fs::File;
use std::io::{stdout, BufReader, BufWriter, Read, Write};
use std::process;
use std::str::FromStr;
use vrp_cli::extensions::check::check_pragmatic_solution;
use vrp_core::models::Problem;
use vrp_core::prelude::GenericError;
use vrp_pragmatic::format::problem::{deserialize_matrix, deserialize_problem, PragmaticProblem};
use vrp_pragmatic::format::MultiFormatError;

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
) -> Result<Option<T>, GenericError> {
    matches
        .get_one::<String>(arg_name)
        .map(|arg| {
            arg.parse::<T>()
                .map_err(|err| format!("cannot get float value, error: '{err}': '{arg_desc}'").into())
                .map(Some)
        })
        .unwrap_or(Ok(None))
}

fn parse_int_value<T: FromStr<Err = std::num::ParseIntError>>(
    matches: &ArgMatches,
    arg_name: &str,
    arg_desc: &str,
) -> Result<Option<T>, GenericError> {
    matches
        .get_one::<String>(arg_name)
        .map(|arg| {
            arg.parse::<T>()
                .map_err(|err| format!("cannot get integer value, error: '{err}': '{arg_desc}'").into())
                .map(Some)
        })
        .unwrap_or(Ok(None))
}

fn check_solution(
    matches: &ArgMatches,
    input_format: &str,
    problem_arg_name: &str,
    solution_arg_name: &str,
    matrix_arg_name: &str,
) -> Result<(), GenericError> {
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
            Err(vec!["pragmatic format expects one problem, one solution file, and optionally matrices".into()])
        }
        _ => Err(vec![format!("unknown format: '{input_format}'").into()]),
    }
    .map_err(|errs| format!("checker found {} errors:\n{}", errs.len(), GenericError::join_many(&errs, "\n")).into())
}

pub(crate) fn get_core_problem<F: Read>(
    problem_reader: BufReader<F>,
    matrices_readers: Option<Vec<BufReader<F>>>,
) -> Result<Problem, MultiFormatError> {
    let problem = deserialize_problem(problem_reader)?;

    let matrices = matrices_readers.map(|matrices| {
        matrices.into_iter().map(|file| deserialize_matrix(BufReader::new(file))).collect::<Result<Vec<_>, _>>()
    });

    let matrices = if let Some(matrices) = matrices { Some(matrices?) } else { None };

    (problem, matrices).read_pragmatic()
}
