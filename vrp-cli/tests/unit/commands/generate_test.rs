use super::*;

const PRAGMATIC_PROBLEM_PATH: &str = "../examples/data/pragmatic/simple.basic.problem.json";

#[test]
fn can_generate_problem_from_args() {
    let args = vec![
        "generate",
        "pragmatic",
        "--prototypes",
        PRAGMATIC_PROBLEM_PATH,
        "--jobs-size",
        "100",
        "--vehicles-size",
        "10",
    ];
    let matches = get_generate_app().get_matches_from_safe(args).unwrap();

    let (problem, format) = generate_problem_from_args(&matches).unwrap();

    assert_eq!(format, "pragmatic");
    assert_eq!(problem.plan.jobs.len(), 100);
    assert_eq!(problem.fleet.vehicles.len(), 10);
}
