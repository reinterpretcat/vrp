#[path = "./csv.rs"]
mod csv_import;
use self::csv_import::read_csv_problem;

#[path = "./hre.rs"]
mod hre_import;
use self::hre_import::read_hre_problem;

use super::app::*;
use super::*;

use std::io::{BufReader, Read};
use vrp_pragmatic::format::problem::{serialize_problem, Problem};

pub fn run_import(matches: &ArgMatches) {
    let input_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let input_files = matches
        .values_of(INPUT_ARG_NAME)
        .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "input"))).collect::<Vec<_>>());

    match import_problem(input_format, input_files) {
        Ok(problem) => {
            let out_result = matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out result"));
            let out_buffer = create_write_buffer(out_result);
            if let Err(err) = serialize_problem(out_buffer, &problem) {
                eprintln!("Cannot serialize result problem: '{}'", err);
                process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("Cannot import problem: '{}'", err);
            process::exit(1);
        }
    }
}

pub fn import_problem<R: Read>(input_format: &str, readers: Option<Vec<BufReader<R>>>) -> Result<Problem, String> {
    match (input_format, readers) {
        ("csv", Some(mut readers)) if readers.len() == 2 => {
            let jobs = readers.swap_remove(0);
            let vehicles = readers.swap_remove(0);
            read_csv_problem(jobs, vehicles).map_err(|err| format!("cannot read csv: {}", err))
        }
        ("csv", _) => Err("csv format expects two files with jobs and vehicles as an input".to_string()),
        ("hre", Some(mut readers)) if readers.len() == 1 => {
            let problem = readers.swap_remove(0);
            read_hre_problem(problem).map_err(|err| format!("cannot read problem from hre json: '{}'", err))
        }
        ("hre", _) => Err("hre format expects one input file".to_string()),
        _ => Err(format!("unknown format: '{}'", input_format)),
    }
}
