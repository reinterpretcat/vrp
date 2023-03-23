#[cfg(test)]
#[path = "../../tests/unit/commands/generate_test.rs"]
mod generate_test;

use super::*;
use std::io::BufReader;
use vrp_cli::extensions::generate::generate_problem;
use vrp_pragmatic::format::problem::{serialize_problem, Problem};
use vrp_pragmatic::format::{CoordIndex, FormatError};
use vrp_pragmatic::validation::ValidationContext;

pub const FORMAT_ARG_NAME: &str = "FORMAT";
pub const PROTOTYPES_ARG_NAME: &str = "prototypes";
pub const OUT_RESULT_ARG_NAME: &str = "out-result";
pub const JOBS_SIZE_ARG_NAME: &str = "jobs-size";
pub const VEHICLES_SIZE_ARG_NAME: &str = "vehicles-size";
pub const LOCATIONS_ARG_NAME: &str = "locations";
pub const AREA_SIZE_ARG_NAME: &str = "area-size";

pub fn get_generate_app() -> Command {
    Command::new("generate")
        .about("Provides the way to generate meaningful problems for testing")
        .arg(Arg::new(FORMAT_ARG_NAME).help("Specifies input type").required(true).value_parser(["pragmatic"]).index(1))
        .arg(
            Arg::new(PROTOTYPES_ARG_NAME)
                .help("Sets input files which contains a VRP definition prototype")
                .short('p')
                .long(PROTOTYPES_ARG_NAME)
                .required(true)
                .num_args(1..),
        )
        .arg(
            Arg::new(OUT_RESULT_ARG_NAME)
                .help("Specifies path to the file for result output")
                .short('o')
                .long(OUT_RESULT_ARG_NAME)
                .required(false),
        )
        .arg(
            Arg::new(LOCATIONS_ARG_NAME)
                .help("Specifies path to the file with a list of job locations")
                .short('l')
                .long(LOCATIONS_ARG_NAME)
                .required(false),
        )
        .arg(
            Arg::new(JOBS_SIZE_ARG_NAME)
                .help("Amount of jobs in the plan of generated problem")
                .short('j')
                .long(JOBS_SIZE_ARG_NAME)
                .required(true),
        )
        .arg(
            Arg::new(VEHICLES_SIZE_ARG_NAME)
                .help("Amount of vehicle types in the fleet of generated problem")
                .short('v')
                .long(VEHICLES_SIZE_ARG_NAME)
                .required(true),
        )
        .arg(
            Arg::new(AREA_SIZE_ARG_NAME)
                .help("Half side size of job distribution bounding box. Center is calculated using prototype locations")
                .short('a')
                .long(AREA_SIZE_ARG_NAME)
                .required(false),
        )
}

pub fn run_generate(matches: &ArgMatches) -> Result<(), String> {
    match generate_problem_from_args(matches) {
        Ok((problem, input_format)) => {
            let out_result = matches.get_one::<String>(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out result"));
            let out_buffer = create_write_buffer(out_result);

            match input_format.as_str() {
                "pragmatic" => serialize_problem(out_buffer, &problem)
                    .map_err(|err| format!("cannot serialize as pragmatic problem: '{err}'")),
                _ => Err(format!("unknown output format: '{input_format}'")),
            }
        }
        Err(err) => Err(format!("cannot generate problem: '{err}'")),
    }
}

fn generate_problem_from_args(matches: &ArgMatches) -> Result<(Problem, String), String> {
    let input_format = matches.get_one::<String>(FORMAT_ARG_NAME).unwrap();

    let input_files = matches
        .get_many::<String>(PROTOTYPES_ARG_NAME)
        .map(|paths| paths.map(|path| BufReader::new(open_file(path, "input"))).collect::<Vec<_>>());

    let locations_file =
        matches.get_one::<String>(LOCATIONS_ARG_NAME).map(|path| BufReader::new(open_file(path, "locations")));

    let jobs_size = parse_int_value::<usize>(matches, JOBS_SIZE_ARG_NAME, "jobs size")?.unwrap();
    let vehicles_size = parse_int_value::<usize>(matches, VEHICLES_SIZE_ARG_NAME, "vehicles size")?.unwrap();
    let area_size = parse_float_value::<f64>(matches, AREA_SIZE_ARG_NAME, "area size")?;

    generate_problem(input_format, input_files, locations_file, jobs_size, vehicles_size, area_size).and_then(
        |problem| {
            let coord_index = CoordIndex::new(&problem);
            ValidationContext::new(&problem, None, &coord_index)
                .validate()
                .map_err(|errors| {
                    format!(
                        "generated problem has some validation errors:\n{}",
                        FormatError::format_many(errors.as_slice(), "\n")
                    )
                })
                .map(|_| (problem, input_format.to_owned()))
        },
    )
}
