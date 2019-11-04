use crate::helpers::get_test_resource;
use crate::json::HereProblem;

#[test]
fn can_read_problem() {
    let problem = get_test_resource("../data/small/minimal.problem.json").unwrap();
    let matrix = get_test_resource("../data/small/minimal.matrix.json").unwrap();

    let problem = (problem, vec![matrix]).read_here().unwrap();

    assert_eq!(problem.fleet.vehicles.len(), 1);
    assert_eq!(problem.jobs.all().collect::<Vec<_>>().len(), 2);
    assert!(problem.locks.is_empty());

    let tw = problem
        .fleet
        .vehicles
        .first()
        .as_ref()
        .unwrap()
        .details
        .first()
        .as_ref()
        .unwrap()
        .time
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(tw.start, 1562230800.);
    assert_eq!(tw.end, 1562263200.);
}
