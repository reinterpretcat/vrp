#[cfg(test)]
#[path = "../../../tests/unit/json/problem/reader_test.rs"]
mod reader_test;

#[path = "./job_reader.rs"]
mod job_reader;

#[path = "./fleet_reader.rs"]
mod fleet_reader;

use super::StringReader;
use crate::constraints::*;
use crate::extensions::{MultiDimensionalCapacity, OnlyVehicleActivityCost};
use crate::json::coord_index::CoordIndex;
use crate::json::problem::reader::fleet_reader::{create_transport_costs, read_fleet, read_limits};
use crate::json::problem::reader::job_reader::{read_jobs_with_extra_locks, read_locks};
use crate::json::problem::{deserialize_matrix, deserialize_problem, JobVariant, Matrix};
use chrono::DateTime;
use core::construction::constraints::*;
use core::models::common::{Dimensions, TimeWindow, Timestamp};
use core::models::problem::{ActivityCost, Fleet, Job, TransportCost};
use core::models::{Extras, Lock, Problem};
use core::refinement::objectives::PenalizeUnassigned;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::iter::FromIterator;
use std::sync::Arc;

pub type ApiProblem = crate::json::problem::Problem;
pub type JobIndex = HashMap<String, Arc<Job>>;

/// Reads specific problem definition from various sources.
pub trait HereProblem {
    fn read_here(self) -> Result<Problem, String>;
}

impl HereProblem for (File, Vec<File>) {
    fn read_here(self) -> Result<Problem, String> {
        let problem = deserialize_problem(BufReader::new(&self.0)).map_err(|err| err.to_string())?;

        let matrices = self.1.iter().fold(vec![], |mut acc, matrix| {
            acc.push(deserialize_matrix(BufReader::new(matrix)).unwrap());
            acc
        });

        map_to_problem(problem, matrices)
    }
}

impl HereProblem for (String, Vec<String>) {
    fn read_here(self) -> Result<Problem, String> {
        let problem = deserialize_problem(BufReader::new(StringReader::new(&self.0))).map_err(|err| err.to_string())?;

        let matrices = self.1.iter().fold(vec![], |mut acc, matrix| {
            acc.push(deserialize_matrix(BufReader::new(StringReader::new(matrix))).unwrap());
            acc
        });

        map_to_problem(problem, matrices)
    }
}

impl HereProblem for (ApiProblem, Vec<Matrix>) {
    fn read_here(self) -> Result<Problem, String> {
        map_to_problem(self.0, self.1)
    }
}

pub struct ProblemProperties {
    has_multi_dimen_capacity: bool,
    has_breaks: bool,
    has_skills: bool,
    has_unreachable_locations: bool,
    has_fixed_cost: bool,
    has_multi_tour: bool,
}

fn map_to_problem(api_problem: ApiProblem, matrices: Vec<Matrix>) -> Result<Problem, String> {
    let problem_props = get_problem_properties(&api_problem, &matrices);

    let coord_index = create_coord_index(&api_problem);
    let transport = Arc::new(create_transport_costs(&matrices));
    let activity = Arc::new(OnlyVehicleActivityCost::default());
    let fleet = read_fleet(&api_problem, &problem_props, &coord_index);

    let mut job_index = Default::default();
    let (jobs, locks) = read_jobs_with_extra_locks(
        &api_problem,
        &problem_props,
        &coord_index,
        &fleet,
        transport.as_ref(),
        &mut job_index,
    );
    let locks = locks.into_iter().chain(read_locks(&api_problem, &job_index).into_iter()).collect();
    let limits = read_limits(&api_problem);
    let extras = create_extras(&api_problem.id, &problem_props, coord_index);
    let constraint =
        create_constraint_pipeline(&fleet, activity.clone(), transport.clone(), problem_props, &locks, limits);

    Ok(Problem {
        fleet: Arc::new(fleet),
        jobs: Arc::new(jobs),
        locks,
        constraint: Arc::new(constraint),
        activity,
        transport,
        objective: Arc::new(PenalizeUnassigned::default()),
        extras: Arc::new(extras),
    })
}

fn create_coord_index(api_problem: &ApiProblem) -> CoordIndex {
    let mut index = CoordIndex::default();

    // process plan
    api_problem.plan.jobs.iter().for_each(|job| match &job {
        JobVariant::Single(job) => {
            if let Some(pickup) = &job.places.pickup {
                index.add_from_vec(&pickup.location);
            }
            if let Some(delivery) = &job.places.delivery {
                index.add_from_vec(&delivery.location);
            }
        }
        JobVariant::Multi(job) => {
            job.places.pickups.iter().for_each(|pickup| {
                index.add_from_vec(&pickup.location);
            });
            job.places.deliveries.iter().for_each(|delivery| {
                index.add_from_vec(&delivery.location);
            });
        }
    });

    // process fleet
    api_problem.fleet.types.iter().for_each(|vehicle| {
        vehicle.shifts.iter().for_each(|shift| {
            index.add_from_vec(&shift.start.location);

            if let Some(end) = &shift.end {
                index.add_from_vec(&end.location);
            }

            if let Some(breaks) = &shift.breaks {
                breaks.iter().for_each(|vehicle_break| {
                    if let Some(location) = &vehicle_break.location {
                        index.add_from_vec(location);
                    }
                });
            }
        });
    });

    index
}

fn create_constraint_pipeline(
    fleet: &Fleet,
    activity: Arc<dyn ActivityCost + Send + Sync>,
    transport: Arc<dyn TransportCost + Send + Sync>,
    props: ProblemProperties,
    locks: &Vec<Arc<Lock>>,
    limits: Option<TravelLimitFunc>,
) -> ConstraintPipeline {
    let mut constraint = ConstraintPipeline::default();
    constraint.add_module(Box::new(TimingConstraintModule::new(activity, transport.clone(), 1)));

    if props.has_multi_tour {
        let threshold = 0.9;
        if props.has_multi_dimen_capacity {
            // TODO
            constraint.add_module(Box::new(MultiTourCapacityConstraintModule::<MultiDimensionalCapacity>::new(
                2,
                Box::new(|_capacity| unimplemented!()),
            )));
        } else {
            constraint.add_module(Box::new(MultiTourCapacityConstraintModule::<i32>::new(
                2,
                Box::new(move |capacity| (*capacity as f64 * threshold).round() as i32),
            )));
        }
    } else {
        if props.has_multi_dimen_capacity {
            constraint.add_module(Box::new(CapacityConstraintModule::<MultiDimensionalCapacity>::new(2)));
        } else {
            constraint.add_module(Box::new(CapacityConstraintModule::<i32>::new(2)));
        }
    }

    if props.has_breaks {
        constraint.add_module(Box::new(BreakModule::new(4, Some(-100.), false)));
    }

    if props.has_skills {
        constraint.add_module(Box::new(SkillsModule::new(10)));
    }

    if !locks.is_empty() {
        constraint.add_module(Box::new(StrictLockingModule::new(fleet, locks.clone(), 3)));
    }

    if let Some(limits) = limits {
        constraint.add_module(Box::new(TravelModule::new(limits.clone(), transport.clone(), 5, 6)));
    }

    if props.has_unreachable_locations {
        constraint.add_module(Box::new(ReachableModule::new(transport.clone(), 11)));
    }

    if props.has_fixed_cost {
        constraint.add_module(Box::new(ExtraCostModule::default()));
    }

    constraint
}

fn create_extras(problem_id: &String, props: &ProblemProperties, coord_index: CoordIndex) -> Extras {
    let mut extras = Extras::default();
    extras.insert("problem_id".to_string(), Box::new(problem_id.clone()));
    extras.insert(
        "capacity_type".to_string(),
        Box::new((if props.has_multi_dimen_capacity { "multi" } else { "single" }).to_string()),
    );
    extras.insert("coord_index".to_owned(), Box::new(coord_index));

    extras
}

fn parse_time(time: &String) -> Timestamp {
    let time = DateTime::parse_from_rfc3339(time).unwrap();
    time.timestamp() as Timestamp
}

fn parse_time_window(tw: &Vec<String>) -> TimeWindow {
    assert_eq!(tw.len(), 2);
    TimeWindow::new(parse_time(tw.first().unwrap()), parse_time(tw.last().unwrap()))
}

fn get_problem_properties(api_problem: &ApiProblem, matrices: &Vec<Matrix>) -> ProblemProperties {
    let has_unreachable_locations = matrices.iter().any(|m| m.error_codes.is_some());
    let has_multi_dimen_capacity = api_problem.fleet.types.iter().any(|t| t.capacity.len() > 1)
        || api_problem.plan.jobs.iter().any(|j| match j {
            JobVariant::Single(job) => job.demand.len() > 1,
            JobVariant::Multi(job) => {
                job.places.pickups.iter().any(|p| p.demand.len() > 1)
                    || job.places.deliveries.iter().any(|p| p.demand.len() > 1)
            }
        });
    let has_breaks = api_problem
        .fleet
        .types
        .iter()
        .flat_map(|t| &t.shifts)
        .any(|shift| shift.breaks.as_ref().map_or(false, |b| b.len() > 0));
    let has_skills = api_problem.plan.jobs.iter().any(|j| match j {
        JobVariant::Single(job) => job.skills.is_some(),
        JobVariant::Multi(job) => job.skills.is_some(),
    });
    let has_fixed_cost = api_problem.fleet.types.iter().any(|t| t.costs.fixed.is_some());
    let has_multi_tour =
        api_problem.fleet.types.iter().any(|t| t.shifts.iter().any(|s| s.max_tours.map_or(false, |mt| mt > 1)));

    ProblemProperties {
        has_multi_dimen_capacity,
        has_breaks,
        has_skills,
        has_unreachable_locations,
        has_fixed_cost,
        has_multi_tour,
    }
}

fn add_skills(dimens: &mut Dimensions, skills: &Option<Vec<String>>) {
    if let Some(skills) = skills {
        dimens.insert("skills".to_owned(), Box::new(HashSet::<String>::from_iter(skills.iter().cloned())));
    }
}
