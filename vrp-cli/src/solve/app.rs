use super::*;

pub const FORMAT_ARG_NAME: &str = "FORMAT";
pub const PROBLEM_ARG_NAME: &str = "PROBLEM";
pub const MATRIX_ARG_NAME: &str = "matrix";
pub const GENERATIONS_ARG_NAME: &str = "max-generations";
pub const TIME_ARG_NAME: &str = "max-time";
pub const GEO_JSON_ARG_NAME: &str = "geo-json";

pub const INIT_SOLUTION_ARG_NAME: &str = "init-solution";
pub const OUT_RESULT_ARG_NAME: &str = "out-result";
pub const GET_LOCATIONS_ARG_NAME: &str = "get-locations";

pub fn get_solve_app<'a,'b>() -> App<'a, 'b> {
    App::new("solve")
        .about("Solves variations of Vehicle Routing Problem")
        .arg(
            Arg::with_name(FORMAT_ARG_NAME)
                .help("Specifies the problem type")
                .required(true)
                .possible_values(&["solomon", "lilim", "pragmatic"])
                .index(1),
        )
        .arg(Arg::with_name(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
        .arg(
            Arg::with_name(GENERATIONS_ARG_NAME)
                .help("Specifies maximum number of generations")
                .short("n")
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
                .short("i")
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
                .help("Returns list of unique locations")
                .short("l")
                .long(GET_LOCATIONS_ARG_NAME)
                .required(false),
        )
        .arg(
            Arg::with_name(GEO_JSON_ARG_NAME)
                .help("Specifies path to solution output in geo json format")
                .short("g")
                .long(GEO_JSON_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
}
