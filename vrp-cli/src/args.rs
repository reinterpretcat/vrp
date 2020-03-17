extern crate clap;
use clap::{App, Arg, ArgMatches};

pub const FORMAT_ARG_NAME: &str = "FORMAT";
pub const PROBLEM_ARG_NAME: &str = "PROBLEM";
pub const MATRIX_ARG_NAME: &str = "routing-matrix";
pub const GENERATIONS_ARG_NAME: &str = "max-generations";
pub const TIME_ARG_NAME: &str = "max-time";

pub const INIT_SOLUTION_ARG_NAME: &str = "init-solution";
pub const OUT_RESULT_ARG_NAME: &str = "out-result";
pub const GET_LOCATIONS_ARG_NAME: &str = "get-locations";

pub fn get_arg_matches(formats: Vec<&str>) -> ArgMatches {
    App::new("Vehicle Routing Problem Solver")
        .version("0.1")
        .author("Ilya Builuk <ilya.builuk@gmail.com>")
        .about("Solves variations of Vehicle Routing Problem")
        .arg(
            Arg::with_name(FORMAT_ARG_NAME)
                .help("Specifies the problem type")
                .required(true)
                .possible_values(formats.as_slice())
                .index(1),
        )
        .arg(Arg::with_name(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
        .arg(
            Arg::with_name(GENERATIONS_ARG_NAME)
                .help("Specifies maximum amount of generations")
                .short("g")
                .long(GENERATIONS_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(TIME_ARG_NAME)
                .help("Specifies max time algorithm run in seconds")
                .short("t")
                .long(TIME_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(INIT_SOLUTION_ARG_NAME)
                .help("Specifies path to file with initial solution")
                .short("s")
                .long(INIT_SOLUTION_ARG_NAME)
                .required(false)
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
        .arg(
            Arg::with_name(OUT_RESULT_ARG_NAME)
                .help("Specifies path to file for output result")
                .short("o")
                .long(OUT_RESULT_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(GET_LOCATIONS_ARG_NAME)
                .help("Specifies whether list of unique locations should be returned")
                .short("l")
                .long(GET_LOCATIONS_ARG_NAME)
                .required(false),
        )
        .get_matches()
}
