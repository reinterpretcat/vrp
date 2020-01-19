//! Provides functionality to represent VRP solution in adjacency matrix form.
//!
//! Encoding schema:
//!
//! For each job in plan create a tuple:
//!  single -> places -> times : (job, 0, place_index, time_window_index)
//!  multi -> singles-> places-> times -> (job, single_index, place_index, time_window_index)
//!   => assign unique index
//!
//! For each actor in fleet create a tuple:
//!  actors -> (start, time), (end, time) -> unique
//!  => assign unique index (agreed indexing within jobs)
//!
//! Example:
//!
//! from problem:
//!   actors:       a b c
//!   activities:   (01) 02 03 04 05 06 07 08 09 (10)
//! where (01) and (10) - depots (start and end)
//!
//! routes with their activities in solution:
//!   a: 01 03 06 08 10
//!   b: 01 07 04 10
//!   c: 01 09 05 02 10
//!
//! adjacency matrix:
//!   01 02 03 04 05 06 07 08 09 10
//! 01       a           b     c
//! 02                            c
//! 03                a
//! 04                            b
//! 05    c
//! 06                      a
//! 07          b
//! 08                            a
//! 09             c
//! 10
//!

//#[cfg(test)]
//#[path = "../../tests/unit/ann/adjacency_matrix_test.rs"]
//mod adjacency_matrix_test;

use crate::construction::states::{RouteContext, SolutionContext};
use crate::models::common::{Location, Schedule, TimeWindow};
use crate::models::problem::{Actor, ActorDetail, Job, Single};
use crate::models::solution::{Activity, Registry, TourActivity};
use crate::models::Problem;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use crate::models::problem::Place as JobPlace;
use crate::models::solution::Place as ActivityPlace;
use std::slice::Iter;

pub trait AdjacencyMatrix {
    ///
    fn new(dimension: usize) -> Self;

    /// Iterates over matrix values.
    fn iter(&self) -> Iter<f64>;

    /// Sets given value to cell.
    fn set_cell(&mut self, row: usize, col: usize, value: f64);

    /// Scans given row in order to find first occurance of element for which predicate returns true.
    fn scan_row<F>(&self, row: usize, predicate: F) -> Option<usize>
    where
        F: Fn(f64) -> bool;
}

/// Provides way to encode/decode solution to adjacency matrix representation.
pub struct AdjacencyMatrixDecipher {
    problem: Arc<Problem>,
    activity_direct_index: HashMap<ActivityInfo, usize>,
    activity_reverse_index: HashMap<usize, ActivityInfo>,
    actor_direct_index: HashMap<Arc<Actor>, usize>,
    actor_reverse_index: HashMap<usize, Arc<Actor>>,
}

/// Provides way to store job or actor information to restore tour activity properly.
#[derive(Hash, Eq, PartialEq, Clone)]
enum ActivityInfo {
    Job(ActivityWithJob),
    Actor(ActivityWithActor),
}

// TODO remove this type and implement Hash/Eq on ActorDetail directly.
type ActorDetailKey = (Option<Location>, Option<Location>, i64, i64);

/// Represents specific job activity: (job, single index, place index, time window index) schema.
type ActivityWithJob = (Job, usize, usize, usize);
/// Represent specific terminal activity: (actor detail, location).
type ActivityWithActor = (ActorDetailKey, usize);

impl AdjacencyMatrixDecipher {
    /// Creates `AdjacencyMatrixDecipher` for the given problem.
    pub fn new(problem: Arc<Problem>) -> Self {
        let mut decipher = Self {
            problem: problem.clone(),
            activity_direct_index: Default::default(),
            activity_reverse_index: Default::default(),
            actor_direct_index: problem.fleet.actors.iter().cloned().zip(1..).collect(),
            actor_reverse_index: (1..).zip(problem.fleet.actors.iter().cloned()).collect(),
        };

        get_unique_actor_details(&problem.fleet.actors).into_iter().for_each(|adk| match (adk.0, adk.1) {
            (Some(start), Some(end)) if start == end => decipher.add(ActivityInfo::Actor((adk.clone(), start))),
            (Some(start), Some(end)) => {
                decipher.add(ActivityInfo::Actor((adk.clone(), start)));
                decipher.add(ActivityInfo::Actor((adk.clone(), end)));
            }
            (None, Some(end)) => decipher.add(ActivityInfo::Actor((adk.clone(), end))),
            (Some(start), None) => decipher.add(ActivityInfo::Actor((adk.clone(), start))),
            _ => {}
        });

        problem.jobs.all().for_each(|job| {
            match &job {
                Job::Single(single) => vec![(0, single.places.iter().collect::<Vec<_>>())],
                Job::Multi(multi) => (0..)
                    .zip(multi.jobs.iter())
                    .map(|(idx, j)| (idx, j.places.iter().collect::<Vec<_>>()))
                    .collect::<Vec<_>>(),
            }
            .iter()
            .for_each(|(single_idx, places)| {
                (0..).zip(places.iter()).for_each(|(place_idx, place)| {
                    (0..).zip(place.times.iter()).for_each(|(tw_idx, _)| {
                        decipher.add(ActivityInfo::Job((job.clone(), *single_idx, place_idx, tw_idx)));
                    })
                })
            });
        });

        decipher
    }

    pub fn encode<T: AdjacencyMatrix>(&self, solution_ctx: &SolutionContext) -> T {
        let mut matrix = T::new(self.dimensions());

        solution_ctx.routes.iter().for_each(|rc| {
            let actor = &rc.route.actor;
            let actor_idx = *self.actor_direct_index.get(actor).unwrap() as f64;

            rc.route.tour.legs().for_each(|(items, _)| {
                match items {
                    [prev, next] => {
                        let from = *self.activity_direct_index.get(&create_activity_info(actor, prev)).unwrap();
                        let to = *self.activity_direct_index.get(&create_activity_info(actor, next)).unwrap();

                        matrix.set_cell(from, to, actor_idx);
                    }
                    [_] => {}
                    _ => panic!("Unexpected route leg configuration."),
                };
            });
        });

        matrix
    }

    pub fn decode<T: AdjacencyMatrix>(&self, matrix: &T) -> SolutionContext {
        let registry = Registry::new(&self.problem.fleet);
        let actor_indices = matrix.iter().map(|&i| i as usize).filter(|&i| i != 0).collect::<HashSet<_>>();

        // TODO consider locked and ignored!

        // TODO make it in parallel
        let routes = actor_indices.iter().fold(vec![], |mut acc, actor_idx| {
            let actor = self.actor_reverse_index.get(actor_idx).unwrap().clone();
            let rc = RouteContext::new(actor.clone());

            // TODO
            //            let mut row_idx = *self.activity_direct_index.get(&create_activity_info(&actor, rc.route.tour.start().unwrap())).unwrap();
            //
            //            let next_activity = actor.detail.end.and_then(|_| rc.route.tour.end());
            //            let mut prev_tour_activity_idx = 0;
            //            loop {
            //                if let Some(target_activity_info_idx) = matrix.scan_row(row_idx, |v| v == *actor_idx as f64) {
            //                    let prev_activity = rc.route.tour.get(prev_tour_activity_idx);
            //                    let prev_location = prev_activity.map(|a| a.place.location);
            //
            //                    let target_activity_info = self.activity_reverse_index.get(&target_activity_info_idx).unwrap();
            //                    let target_activity = create_tour_activity(target_activity_info, prev_location);
            //                } else {
            //                    break;
            //                }
            //                unimplemented!()
            //            }

            acc.push(rc);
            acc
        });

        SolutionContext {
            required: vec![],
            ignored: vec![],
            unassigned: Default::default(),
            locked: Default::default(),
            routes,
            registry,
        }
    }

    fn add(&mut self, activity_info: ActivityInfo) {
        assert_eq!(self.activity_direct_index.len(), self.activity_reverse_index.len());

        self.activity_direct_index.insert(activity_info.clone(), self.activity_direct_index.len());
        self.activity_reverse_index.insert(self.activity_reverse_index.len(), activity_info);
    }

    fn dimensions(&self) -> usize {
        self.activity_direct_index.len()
    }
}

fn get_unique_actor_details(actors: &Vec<Arc<Actor>>) -> Vec<ActorDetailKey> {
    let mut unique: HashSet<ActorDetailKey> = Default::default();
    let mut details = actors.iter().map(|a| create_actor_detail_key(&a.detail)).collect::<Vec<_>>();

    details.retain(|&d| unique.insert(d));

    details
}

fn create_actor_detail_key(detail: &ActorDetail) -> ActorDetailKey {
    (detail.start.clone(), detail.end.clone(), detail.time.start.round() as i64, detail.time.end.round() as i64)
}

fn create_activity_info(actor: &Arc<Actor>, a: &TourActivity) -> ActivityInfo {
    match a.retrieve_job() {
        Some(job) => {
            let (single_idx, single) = match &job {
                Job::Multi(multi) => {
                    let job = a.job.as_ref().unwrap();
                    let position = multi
                        .jobs
                        .iter()
                        .position(|j| &*j.as_ref() as *const Single == &*job.as_ref() as *const Single)
                        .unwrap();

                    (position, multi.jobs.get(position).unwrap().clone())
                }
                Job::Single(single) => (0, single.clone()),
            };

            let (place_idx, tw_idx) = try_match_activity_place(a, &single.places).unwrap();

            ActivityInfo::Job((job, single_idx, place_idx, tw_idx))
        }
        None => ActivityInfo::Actor((create_actor_detail_key(&actor.detail), a.place.location)),
    }
}

/// Creates tour activity from the corresponding activity info.
fn create_tour_activity(activity_info: &ActivityInfo, prev: Option<Location>) -> TourActivity {
    match activity_info {
        ActivityInfo::Job(activity_info) => {
            let (job, single_index, place_index, tw_index) = activity_info;
            let single = match job {
                Job::Single(single) => single.clone(),
                Job::Multi(multi) => multi.jobs.get(*single_index).cloned().unwrap(),
            };

            let place = single.places.get(*place_index).unwrap();

            Box::new(Activity {
                place: ActivityPlace {
                    location: place.location.or(prev).unwrap(),
                    duration: place.duration,
                    time: place.times.get(*tw_index).unwrap().clone(),
                },
                schedule: Schedule::new(0., 0.),
                job: Some(single),
            })
        }
        ActivityInfo::Actor(activity_info) => {
            let ((_, _, start, end), location) = activity_info;

            // TODO Remove this with proper hash/eq implementation for ActorDetail
            let start = *start as f64;
            let end = *end as f64;

            Box::new(Activity {
                place: ActivityPlace { location: *location, duration: 0., time: TimeWindow::new(start, end) },
                schedule: Schedule::new(0., 0.),
                job: None,
            })
        }
    }
}

fn try_match_activity_place(activity: &TourActivity, places: &Vec<JobPlace>) -> Option<(usize, usize)> {
    (0_usize..).zip(places.iter()).fold(None, |acc, (place_idx, place)| {
        if acc.is_none() {
            if let Some(location) = place.location {
                if location == activity.place.location {
                    if activity.place.duration == place.duration {
                        for (tw_idx, tw) in (0_usize..).zip(place.times.iter()) {
                            // TODO rely on Eq once implemented
                            if activity.place.time.start == tw.start && activity.place.time.end == tw.end {
                                return Some((place_idx, tw_idx));
                            }
                        }
                    }
                }
            }
        }

        acc
    })
}
