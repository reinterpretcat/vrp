extern crate clap;
use clap::{App, Arg, ArgMatches};

pub const FORMAT_ARG_NAME: &str = "FORMAT";
pub const PROBLEM_ARG_NAME: &str = "PROBLEM";
pub const MATRIX_ARG_NAME: &str = "routing-matrix";
pub const GENERATIONS_ARG_NAME: &str = "max-generations";
pub const VARIATION_COEFFICIENT_ARG_NAME: &str = "variation-coefficient";
pub const MINIMIZE_ROUTES_ARG_NAME: &str = "minimize-routes";
pub const INIT_SOLUTION_ARG_NAME: &str = "init-solution";

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
                .default_value("2000")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(VARIATION_COEFFICIENT_ARG_NAME)
                .help("Specifies variation coefficient termination criteria in form \"sample_size,threshold\"")
                .short("v")
                .long(VARIATION_COEFFICIENT_ARG_NAME)
                .required(false)
                .default_value("200,0.001")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(MINIMIZE_ROUTES_ARG_NAME)
                .help("Prefer less routes over total cost")
                .short("r")
                .long(MINIMIZE_ROUTES_ARG_NAME)
                .required(false)
                .default_value("false")
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
        .get_matches()
}
