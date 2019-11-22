use crate::checker::index::create_solution_info;
use crate::checker::models::{SolutionInfo, TourInfo};
use crate::helpers::create_default_vehicle;
use crate::json::problem::*;
use crate::json::solution::{Activity, Extras, Solution, Tour};
use std::collections::{HashMap, HashSet};

struct Prototype {
    activity_type: String,
    job_type: String,
    place_index: usize,

    location: Vec<f64>,
    demand: Vec<i32>,
    times: Vec<Vec<i32>>,
    duration: f64,

    tag: String,
}

/// Creates tag which parameters for original job place
pub fn create_info_tag(
    job_type: &str,
    place_index: usize,
    location: Vec<f64>,
    demand: Vec<i32>,
    times: Vec<Vec<i32>>,
    duration: f64,
) -> String {
    assert!(job_type == "single" || job_type == "multi");
    format!(
        "{} {} {} {} {} {}",
        job_type,
        place_index,
        location.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(":"),
        demand.iter().map(|v| v.to_string()).collect::<Vec<String>>().join(":"),
        times
            .iter()
            .map(|tw| tw.iter().map(|t| t.to_string()).collect::<Vec<String>>().join(","))
            .collect::<Vec<String>>()
            .join(":"),
        duration
    )
}

pub fn create_test_solution_info(
    vehicle_types: Vec<VehicleType>,
    relations: Option<Vec<Relation>>,
    solution: Solution,
) -> SolutionInfo {
    assert!(solution.unassigned.is_empty());

    let job_prototypes = create_job_prototypes(&solution);
    let job_variants = create_job_variants(job_prototypes);

    let problem = Problem {
        id: solution.problem_id.clone(),
        plan: Plan { jobs: job_variants, relations },
        fleet: Fleet { types: vehicle_types },
    };

    create_solution_info(&problem, &solution).unwrap_or_else(|err| panic!("Cannot create solution: '{}'", err))
}

pub fn create_test_tour_info(tour: Tour) -> TourInfo {
    create_test_solution_info(
        vec![create_default_vehicle("my_vehicle")],
        None,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Default::default(),
            tours: vec![tour],
            unassigned: vec![],
            extras: Extras { performance: vec![] },
        },
    )
    .tours
    .first()
    .unwrap()
    .clone()
}

fn create_job_prototypes(solution: &Solution) -> HashMap<String, Vec<Prototype>> {
    solution.tours.iter().fold(HashMap::new(), |mut acc, tour| {
        tour.stops.iter().for_each(|stop| {
            assert_eq!(stop.activities.len(), 1);
            stop.activities.iter().for_each(|activity| match activity.activity_type.as_str() {
                "departure" | "arrival" | "break" => {}
                "pickup" | "delivery" => {
                    let tag = activity.job_tag.as_ref().unwrap_or_else(|| panic!("Activity must specify tag.")).clone();
                    let (job_type, place_index, location, demand, times, duration) = extract_params_from_tag(&tag);

                    acc.entry(activity.job_id.clone()).or_insert_with(|| vec![]).push(Prototype {
                        activity_type: activity.activity_type.clone(),
                        location,
                        demand,
                        duration,
                        job_type,
                        place_index,
                        tag,
                        times,
                    });
                }
                _ => panic!("Unknown activity type: '{}'", activity.activity_type),
            });
        });

        acc
    })
}

fn create_job_variants(prototypes: HashMap<String, Vec<Prototype>>) -> Vec<JobVariant> {
    prototypes.into_iter().fold(Vec::default(), |mut acc, (id, mut prototypes)| {
        assert_eq!(prototypes.iter().map(|p| &p.job_type).collect::<HashSet<_>>().len(), 1);
        assert_eq!(prototypes.iter().map(|p| &p.place_index).collect::<HashSet<_>>().len(), prototypes.len());
        prototypes.sort_by(|a, b| a.place_index.cmp(&b.place_index));

        let first = prototypes.first().unwrap();
        let variant = match first.job_type.as_ref() {
            "single" => {
                assert!(prototypes.iter().len() <= 2);
                let places = prototypes.iter().fold(HashMap::new(), |mut acc, p| {
                    acc.insert(
                        p.activity_type.clone(),
                        JobPlace {
                            times: Option::None,
                            location: p.location.clone(),
                            duration: p.duration,
                            tag: Some(p.tag.clone()),
                        },
                    );

                    acc
                });

                JobVariant::Single(Job {
                    id,
                    places: JobPlaces {
                        pickup: places.get("pickup").cloned(),
                        delivery: places.get("delivery").cloned(),
                    },
                    demand: first.demand.clone(),
                    skills: Option::None,
                })
            }
            "multi" => {
                let mut places = prototypes.iter().fold(HashMap::new(), |mut acc, p| {
                    acc.entry(p.activity_type.clone()).or_insert_with(|| vec![]).push(MultiJobPlace {
                        times: Option::None,
                        location: p.location.clone(),
                        duration: p.duration,
                        demand: p.demand.clone(),
                        tag: Some(p.tag.clone()),
                    });

                    acc
                });

                JobVariant::Multi(MultiJob {
                    id,
                    places: MultiJobPlaces {
                        pickups: places.get("pickup").unwrap().clone(),
                        deliveries: places.get("delivery").unwrap().clone(),
                    },
                    skills: Option::None,
                })
            }
            value @ _ => panic!("Unknown job type: '{}'", value),
        };
        acc.push(variant);

        acc
    })
}

fn extract_params_from_tag(tag: &String) -> (String, usize, Vec<f64>, Vec<i32>, Vec<Vec<i32>>, f64) {
    // single 1 53.1:13.1 1,2,3 10,20:30,40 180
    let parts: Vec<&str> = tag.split_whitespace().collect();
    assert_eq!(parts.len(), 6);

    let job_type = parts.get(0).unwrap().to_string();

    let place_index = parts
        .get(1)
        .unwrap()
        .parse::<usize>()
        .unwrap_or_else(|err| panic!("Cannot parse place index in tag '{}': '{}'", tag, err));

    let location = parts.get(2).unwrap().split(':').map(|v| v.parse::<f64>().unwrap()).collect();

    let demand = parts.get(3).unwrap().split(',').map(|v| v.parse::<i32>().unwrap()).collect();

    let times = parts
        .get(4)
        .unwrap()
        .split(':')
        .map(|v| v.split(','))
        .map(|tw| tw.map(|v| v.parse::<i32>().unwrap()).collect::<Vec<i32>>())
        .collect();

    let duration = parts.get(5).and_then(|v| v.parse::<f64>().ok()).unwrap();

    (job_type, place_index, location, demand, times, duration)
}
