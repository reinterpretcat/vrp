#[path = "./csv.rs"]
mod csv_import;
use self::csv_import::read_csv_problem;

use super::app::*;
use super::*;
use std::io::BufReader;
use vrp_pragmatic::json::problem::serialize_problem;

pub fn run_import(matches: &ArgMatches) {
    let input_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let input_files = matches
        .values_of(INPUT_ARG_NAME)
        .map(|paths: Values| paths.map(|path| open_file(path, "input")).collect::<Vec<_>>());
    let out_result = matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out result"));

    match (input_format, input_files) {
        ("csv", Some(ifs)) if ifs.len() == 2 => {
            let out_buffer = create_write_buffer(out_result);
            let problem = read_csv_problem(BufReader::new(ifs.first().unwrap()), BufReader::new(ifs.last().unwrap()))
                .unwrap_or_else(|err| {
                    eprintln!("Cannot read problem from csv: '{}'", err);
                    process::exit(1);
                });

            serialize_problem(out_buffer, &problem).unwrap()
        }
        ("csv", _) => {
            eprintln!("Expecting two files with jobs and vehicles as an input");
            process::exit(1);
        }
        _ => {
            eprintln!("Unknown format: '{}'", input_format);
            process::exit(1);
        }
    }
}
