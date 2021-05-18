use super::*;
use std::io::BufReader;
use vrp_cli::extensions::import::import_problem;
use vrp_pragmatic::format::problem::serialize_problem;

pub const FORMAT_ARG_NAME: &str = "FORMAT";
pub const INPUT_ARG_NAME: &str = "input-files";
pub const OUT_RESULT_ARG_NAME: &str = "out-result";

pub fn get_import_app<'a, 'b>() -> App<'a, 'b> {
    App::new("import")
        .about("Provides the way to import problem from various formats")
        .arg(
            Arg::with_name(FORMAT_ARG_NAME)
                .help("Specifies input type")
                .required(true)
                .possible_values(&["csv"])
                .index(1),
        )
        .arg(
            Arg::with_name(INPUT_ARG_NAME)
                .help("Sets input files which contains a VRP definition")
                .short("i")
                .long(INPUT_ARG_NAME)
                .required(true)
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name(OUT_RESULT_ARG_NAME)
                .help("Specifies path to file for result output")
                .short("o")
                .long(OUT_RESULT_ARG_NAME)
                .required(false)
                .takes_value(true),
        )
}

pub fn run_import(matches: &ArgMatches) -> Result<(), String> {
    let input_format = matches.value_of(FORMAT_ARG_NAME).unwrap();
    let input_files = matches
        .values_of(INPUT_ARG_NAME)
        .map(|paths: Values| paths.map(|path| BufReader::new(open_file(path, "input"))).collect::<Vec<_>>());

    match import_problem(input_format, input_files) {
        Ok(problem) => {
            let out_result = matches.value_of(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out result"));
            let out_buffer = create_write_buffer(out_result);
            serialize_problem(out_buffer, &problem).map_err(|err| format!("cannot serialize result problem: '{}'", err))
        }
        Err(err) => Err(format!("cannot import problem: '{}'", err)),
    }
}
