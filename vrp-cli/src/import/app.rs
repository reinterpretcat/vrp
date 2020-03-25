use super::*;

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
