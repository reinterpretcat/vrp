#[cfg(test)]
#[path = "../../tests/unit/commands/analyze_test.rs"]
mod analyze_test;

use super::*;
use std::sync::Arc;
use vrp_cli::extensions::analyze::{
    TerritorySettings, find_territory_settings, get_dbscan_clusters, get_k_medoids_clusters, get_territory_derivation,
};
use vrp_core::prelude::*;
use vrp_pragmatic::format::Location as ApiLocation;
use vrp_pragmatic::format::problem::{
    BalancePeriodMetric, PragmaticProblem, Problem as ApiProblem, TerritoryProximity,
};
use vrp_pragmatic::format::solution::serialize_named_locations_as_geojson;

const FORMAT_ARG_NAME: &str = "FORMAT";
const PROBLEM_ARG_NAME: &str = "PROBLEM";
const MATRIX_ARG_NAME: &str = "matrix";
const MIN_POINTS_ARG_NAME: &str = "min-points";
const EPSILON_ARG_NAME: &str = "epsilon";
const K_ARG_NAME: &str = "k";
const PROXIMITY_ARG_NAME: &str = "proximity";
const BALANCE_ARG_NAME: &str = "balance";
const OUT_RESULT_ARG_NAME: &str = "out-result";

pub fn get_analyze_app() -> Command {
    Command::new("analyze")
        .about("Provides helper functionality to analyze problem or solution")
        .subcommand(
            Command::new("dbscan")
                .about("Analyzes job clusters using dbscan algorithm")
                .arg(
                    Arg::new(FORMAT_ARG_NAME)
                        .help("Specifies input type")
                        .required(true)
                        .value_parser(["pragmatic"])
                        .index(1),
                )
                .arg(Arg::new(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
                .arg(
                    Arg::new(MIN_POINTS_ARG_NAME)
                        .help("Minimum cluster size")
                        .short('c')
                        .default_value("3")
                        .long(MIN_POINTS_ARG_NAME)
                        .required(false),
                )
                .arg(
                    Arg::new(EPSILON_ARG_NAME)
                        .help("Epsilon parameter in DBSCAN")
                        .short('e')
                        .long(EPSILON_ARG_NAME)
                        .required(false),
                )
                .arg(
                    Arg::new(MATRIX_ARG_NAME)
                        .help("Specifies path to file with routing matrix")
                        .short('m')
                        .long(MATRIX_ARG_NAME)
                        .num_args(1..)
                        .required(false),
                )
                .arg(
                    Arg::new(OUT_RESULT_ARG_NAME)
                        .help("Specifies path to the file for result output")
                        .short('o')
                        .long(OUT_RESULT_ARG_NAME)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("kmedoids")
                .about("Analyzes job clusters using kmedoids algorithm")
                .arg(
                    Arg::new(FORMAT_ARG_NAME)
                        .help("Specifies input type")
                        .required(true)
                        .value_parser(["pragmatic"])
                        .index(1),
                )
                .arg(Arg::new(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
                .arg(
                    Arg::new(K_ARG_NAME)
                        .help("Number of clusters (k) to create")
                        .short('k')
                        .default_value("2")
                        .required(false),
                )
                .arg(
                    Arg::new(MATRIX_ARG_NAME)
                        .help("Specifies path to file with routing matrix")
                        .short('m')
                        .long(MATRIX_ARG_NAME)
                        .num_args(1..)
                        .required(false),
                )
                .arg(
                    Arg::new(OUT_RESULT_ARG_NAME)
                        .help("Specifies path to the file for result output")
                        .short('o')
                        .long(OUT_RESULT_ARG_NAME)
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("territory")
                .about("Derives territory anchors and weights from the problem without solving it")
                .arg(
                    Arg::new(FORMAT_ARG_NAME)
                        .help("Specifies input type")
                        .required(true)
                        .value_parser(["pragmatic"])
                        .index(1),
                )
                .arg(Arg::new(PROBLEM_ARG_NAME).help("Sets the problem file to use").required(true).index(2))
                .arg(
                    Arg::new(MATRIX_ARG_NAME)
                        .help("Specifies path to file with routing matrix")
                        .short('m')
                        .long(MATRIX_ARG_NAME)
                        .num_args(1..)
                        .required(false),
                )
                .arg(
                    Arg::new(PROXIMITY_ARG_NAME)
                        .help("Overrides the proximity metric taken from the problem's territory objective")
                        .short('p')
                        .long(PROXIMITY_ARG_NAME)
                        .value_parser(["distance", "time"])
                        .required(false),
                )
                .arg(
                    Arg::new(BALANCE_ARG_NAME)
                        .help("Overrides the balance metric taken from the problem's territory objective")
                        .short('b')
                        .long(BALANCE_ARG_NAME)
                        .value_parser(["none", "distance", "duration", "activities", "production-value"])
                        .required(false),
                )
                .arg(
                    // Required, like the sibling analyze subcommands: reading the problem logs
                    // progress lines to stdout, so a dump taken from a pipe would not be valid JSON.
                    Arg::new(OUT_RESULT_ARG_NAME)
                        .help("Specifies path to the file for result output")
                        .short('o')
                        .long(OUT_RESULT_ARG_NAME)
                        .required(true),
                ),
        )
}

pub fn run_analyze(
    matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
) -> GenericResult<()> {
    match matches.subcommand() {
        Some(("dbscan", clusters_matches)) => {
            let min_points = parse_int_value::<usize>(clusters_matches, MIN_POINTS_ARG_NAME, "min points")?;
            let epsilon = parse_float_value::<Float>(clusters_matches, EPSILON_ARG_NAME, "epsilon")?;

            read_and_execute_clusters_command(clusters_matches, out_writer_func, |problem| {
                get_dbscan_clusters(problem, min_points, epsilon)
            })
        }
        Some(("kmedoids", clusters_matches)) => {
            let k = parse_int_value::<usize>(clusters_matches, K_ARG_NAME, "k")?;

            read_and_execute_clusters_command(clusters_matches, out_writer_func, |problem| {
                get_k_medoids_clusters(problem, k.unwrap_or(2))
            })
        }
        Some(("territory", territory_matches)) => run_territory(territory_matches, out_writer_func),
        _ => Err("no argument with analyze subcommand was used. Use -h to print help information".into()),
    }
}

/// Dumps the territory the solver would derive for the problem — the anchors and weights the
/// `territory` objective builds when none are supplied — without running the solver. The reference
/// answer a reimplementation of the derivation is checked against, so the output is sorted and
/// byte-stable.
fn run_territory(
    territory_matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
) -> GenericResult<()> {
    let problem_format = territory_matches.get_one::<String>(FORMAT_ARG_NAME).unwrap();
    if problem_format != "pragmatic" {
        return Err(format!("unknown problem format: '{problem_format}'").into());
    }

    let problem_path = territory_matches.get_one::<String>(PROBLEM_ARG_NAME).unwrap();
    let api_problem = deserialize_problem(BufReader::new(open_file(problem_path, "problem")))
        .map_err(|errs| GenericError::from(errs.to_string()))?;

    let matrices = territory_matches
        .get_many::<String>(MATRIX_ARG_NAME)
        .map(|paths| {
            paths
                .map(|path| deserialize_matrix(BufReader::new(open_file(path, "routing matrix"))))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
        .map_err(|errs| GenericError::from(errs.to_string()))?;

    let settings = resolve_territory_settings(&api_problem, territory_matches)?;

    let core_problem = (api_problem, matrices).read_pragmatic().map_err(|errs| GenericError::from(errs.to_string()))?;
    let result = get_territory_derivation(&core_problem, &settings)?;

    let out_file = territory_matches.get_one::<String>(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out result"));
    let mut writer = out_writer_func(out_file);

    writer.write_all(result.as_bytes()).map_err(|err| format!("cannot write result: '{err}'").into())
}

/// The settings the dump runs under: the problem's own `territory` objective, with either component
/// overridable by a flag. Errors rather than guessing when the problem has no territory objective
/// and no `--proximity` was given — a silently defaulted metric would produce a dump that looks
/// authoritative but describes a territory the real solve never builds.
fn resolve_territory_settings(
    api_problem: &ApiProblem,
    territory_matches: &ArgMatches,
) -> GenericResult<TerritorySettings> {
    let configured = find_territory_settings(api_problem);

    let proximity = match territory_matches.get_one::<String>(PROXIMITY_ARG_NAME).map(String::as_str) {
        Some("distance") => Some(TerritoryProximity::Distance),
        Some("time") => Some(TerritoryProximity::Time),
        Some(other) => return Err(format!("unknown proximity metric: '{other}'").into()),
        None => None,
    };
    let proximity = proximity.or(configured.as_ref().map(|settings| settings.proximity)).ok_or_else(|| {
        GenericError::from("problem defines no territory objective: pass --proximity to choose the metric")
    })?;

    let balance = match territory_matches.get_one::<String>(BALANCE_ARG_NAME).map(String::as_str) {
        Some("none") => None,
        Some("distance") => Some(BalancePeriodMetric::Distance),
        Some("duration") => Some(BalancePeriodMetric::Duration),
        Some("activities") => Some(BalancePeriodMetric::Activities),
        Some("production-value") => Some(BalancePeriodMetric::ProductionValue),
        Some(other) => return Err(format!("unknown balance metric: '{other}'").into()),
        // No override: the objective's own balance, and no balance at all when there is no objective.
        None => configured.and_then(|settings| settings.balance),
    };

    Ok(TerritorySettings { proximity, balance })
}

fn read_and_execute_clusters_command<F>(
    clusters_matches: &ArgMatches,
    out_writer_func: fn(Option<File>) -> BufWriter<Box<dyn Write>>,
    command_fn: F,
) -> GenericResult<()>
where
    F: Fn(&Problem) -> GenericResult<Vec<(String, ApiLocation, usize)>>,
{
    let problem_path = clusters_matches.get_one::<String>(PROBLEM_ARG_NAME).unwrap();
    let problem_format = clusters_matches.get_one::<String>(FORMAT_ARG_NAME).unwrap();
    if problem_format != "pragmatic" {
        return Err(format!("unknown problem format: '{problem_format}'").into());
    }

    let problem_reader = BufReader::new(open_file(problem_path, "problem"));
    let matrices_readers = clusters_matches
        .get_many::<String>(MATRIX_ARG_NAME)
        .map(|paths| paths.map(|path| BufReader::new(open_file(path, "routing matrix"))).collect());

    let problem = Arc::new(get_core_problem(problem_reader, matrices_readers).map_err(|errs| errs.to_string())?);
    let locations = command_fn(&problem)?;
    let result = serialize_named_locations_as_geojson(locations.as_slice())?;

    let out_geojson =
        clusters_matches.get_one::<String>(OUT_RESULT_ARG_NAME).map(|path| create_file(path, "out geojson"));
    let mut geo_writer = out_writer_func(out_geojson);

    geo_writer.write_all(result.as_bytes()).map_err(|err| format!("cannot write result: '{err}'").into())
}
