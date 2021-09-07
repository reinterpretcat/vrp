use crate::extensions::generate::generate_problem;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use vrp_pragmatic::format::problem::deserialize_locations;
use vrp_pragmatic::format::{CoordIndex, FormatError};
use vrp_pragmatic::validation::ValidationContext;

#[test]
fn can_generate_problem_from_simple_prototype() {
    let reader = BufReader::new(File::open("../examples/data/pragmatic/simple.basic.problem.json").unwrap());
    let problem =
        generate_problem("pragmatic", Some(vec![reader]), None, 50, 4, None).map_err(|err| panic!("{}", err)).unwrap();
    let coord_index = CoordIndex::new(&problem);

    ValidationContext::new(&problem, None, &coord_index)
        .validate()
        .map_err(|err| panic!("{}", FormatError::format_many(&err, "\t\n")))
        .unwrap();

    // TODO add more checks
    assert_eq!(problem.plan.jobs.len(), 50);
    assert_eq!(problem.fleet.vehicles.len(), 4);
}

#[test]
fn can_generate_problem_with_locations_file() {
    let get_location_reader =
        || BufReader::new(File::open("../examples/data/pragmatic/simple.basic.locations.json").unwrap());
    let problem_reader = BufReader::new(File::open("../examples/data/pragmatic/simple.basic.problem.json").unwrap());
    let locations =
        deserialize_locations(get_location_reader()).expect("cannot get locations").into_iter().collect::<HashSet<_>>();

    let problem = generate_problem("pragmatic", Some(vec![problem_reader]), Some(get_location_reader()), 50, 4, None)
        .expect("cannot generate problem");

    assert!(problem.plan.jobs.iter().all(|job| {
        job.pickups
            .iter()
            .chain(job.deliveries.iter())
            .chain(job.replacements.iter())
            .chain(job.services.iter())
            .flat_map(|tasks| tasks.iter().flat_map(|task| task.places.iter()))
            .map(|place| &place.location)
            .all(|location| locations.contains(location))
    }));
}
