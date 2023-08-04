#[cfg(test)]
#[path = "../../tests/unit/commands/import_test.rs"]
mod import_test;

use super::*;
use std::io::BufReader;
use vrp_cli::extensions::import::import_problem;
use vrp_core::prelude::GenericError;
use vrp_pragmatic::format::problem::serialize_problem;

pub const FORMAT_ARG_NAME: &str = "FORMAT";
pub const INPUT_ARG_NAME: &str = "input-files";
pub const OUT_RESULT_ARG_NAME: &str = "out-result";

pub fn get_import_app() -> Command {
    Command::new("import")
        .about("Provides the way to import problem from various formats")
        .arg(Arg::new(FORMAT_ARG_NAME).help("Specifies input type").required(true).value_parser(["csv"]).index(1))
        .arg(
            Arg::new(INPUT_ARG_NAME)
                .help("Sets input files which contains a VRP definition")
                .short('i')
                .long(INPUT_ARG_NAME)
                .required(true)
                .num_args(1..),
        )
        .arg(
            Arg::new(OUT_RESULT_ARG_NAME)
                .help("Specifies path to file for result output")
                .short('o')
                .long(OUT_RESULT_ARG_NAME)
                .required(false),
        )
}

pub fn run_import(matches: &ArgMatches) -> Result<(), GenericError> {
    let input_format = matches.get_one::<String>(FORMAT_ARG_NAME).unwrap();
    let input_files = matches
        .get_many::<String>(INPUT_ARG_NAME)
        .map(|paths| paths.map(|path| BufReader::new(open_file(path, "input"))).collect::<Vec<_>>());

    match import_problem(input_format, input_files) {
        Ok(problem) => {
            let out_result = matches.get_one::<String>(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out result"));
            let mut out_buffer = create_write_buffer(out_result);
            serialize_problem(&problem, &mut out_buffer)
                .map_err(|err| format!("cannot serialize result problem: '{err}'").into())
        }
        Err(err) => Err(format!("cannot import problem: '{err}'").into()),
    }
}
