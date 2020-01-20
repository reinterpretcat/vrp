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

#[cfg(test)]
#[path = "../../../tests/unit/construction/states/adjacency_matrix_test.rs"]
mod adjacency_matrix_test;

use crate::construction::states::{InsertionContext, InsertionResult, RouteContext, SolutionContext};
use crate::models::common::{Location, Schedule, TimeWindow};
use crate::models::problem::{Actor, ActorDetail, Job, Place, Single};
use crate::models::solution::{Activity, Registry, TourActivity};
use crate::models::Problem;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use crate::construction::heuristics::{evaluate_job_insertion_in_route, InsertionPosition};
use crate::models::problem::Place as JobPlace;
use crate::models::solution::Place as ActivityPlace;
use crate::utils::DefaultRandom;
use std::cmp::Ordering::Less;

/// An adjacency matrix specifies behaviour of a data structure which is used to store VRP solution.
pub trait AdjacencyMatrix {
    /// Creates a new AdjacencyMatrix with `size`*`size`
    fn new(size: usize) -> Self;

    /// Iterates over unique matrix values.
    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a>;

    /// Sets given value to cell.
    fn set_cell(&mut self, row: usize, col: usize, value: f64);

    /// Scans given row in order to find first occurrence of element for which predicate returns true.
    fn scan_row<F>(&self, row: usize, predicate: F) -> Option<usize>
    where
        F: Fn(f64) -> bool;
}

/// A simple `AdjacencyMatrix` implementation using naive sparse matrix implementation.
pub struct SparseMatrix {
    size: usize,
    data: HashMap<usize, Vec<(usize, f64)>>,
    values: HashSet<i64>,
}

impl AdjacencyMatrix for SparseMatrix {
    fn new(size: usize) -> Self {
        Self { size, data: Default::default(), values: Default::default() }
    }

    fn values<'a>(&'a self) -> Box<dyn Iterator<Item = f64> + 'a> {
        Box::new(self.values.iter().map(|&v| unsafe { std::mem::transmute(v) }))
    }

    fn set_cell(&mut self, row: usize, col: usize, value: f64) {
        let mut cells = self.data.entry(row).or_insert_with(|| vec![]);
        cells.push((col, value));
        cells.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(Less));
        self.values.insert(unsafe { std::mem::transmute(value) });
    }

    fn scan_row<F>(&self, row: usize, predicate: F) -> Option<usize>
    where
        F: Fn(f64) -> bool,
    {
        self.data.get(&row).and_then(|cells| cells.iter().find(|(col, v)| predicate(*v))).map(|(row, _)| *row)
    }
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
    Terminal(ActivityWithActor),
}

/// Represents specific job activity: (job, single index, place index, time window index) schema.
type ActivityWithJob = (Job, usize, usize, usize);
/// Represent specific terminal activity: (actor detail, location).
type ActivityWithActor = (ActorDetail, usize);

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

        get_unique_actor_details(&problem.fleet.actors).into_iter().for_each(|adk| match (adk.start, adk.end) {
            (Some(start), Some(end)) if start == end => decipher.add(ActivityInfo::Terminal((adk.clone(), start))),
            (Some(start), Some(end)) => {
                decipher.add(ActivityInfo::Terminal((adk.clone(), start)));
                decipher.add(ActivityInfo::Terminal((adk.clone(), end)));
            }
            (None, Some(end)) => decipher.add(ActivityInfo::Terminal((adk.clone(), end))),
            (Some(start), None) => decipher.add(ActivityInfo::Terminal((adk.clone(), start))),
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

    /// Encodes solution to adjacency matrix.
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

    /// Decodes a feasible solution from adjacency matrix specified by `matrix` which, potentially
    /// might define an unfeasible solution.
    pub fn decode<T: AdjacencyMatrix>(&self, matrix: &T) -> SolutionContext {
        let mut ctx = InsertionContext::new(self.problem.clone(), Arc::new(DefaultRandom::default()));
        ctx.problem.constraint.accept_solution_state(&mut ctx.solution);

        let mut unprocessed = ctx.solution.required.iter().cloned().collect::<HashSet<_>>();
        let mut routes = self.get_routes(&mut ctx.solution, matrix);

        routes.iter_mut().for_each(|rc| {
            let actor = &rc.route.actor;
            let actor_idx = *self.actor_direct_index.get(actor).unwrap();

            let start_info = create_activity_info(actor, rc.route.tour.start().unwrap());
            let start_row_idx = *self.activity_direct_index.get(&start_info).unwrap();
            let activity_infos = self.get_activity_infos(matrix, actor_idx, start_row_idx);

            //let multi_job_index: HashMap<Job, usize> = Default::default();

            activity_infos.into_iter().filter_map(|activity_info| create_single_job(activity_info)).for_each(
                |(job, single)| {
                    let is_unprocessed = unprocessed.contains(&job);

                    if is_unprocessed {
                        let single = Job::Single(single);
                        let mut result =
                            evaluate_job_insertion_in_route(&single, &ctx, &rc, InsertionPosition::Last, None);

                        match result {
                            InsertionResult::Success(success) => {}
                            InsertionResult::Failure(_) => {}
                        }

                        // TODO evaluate insertion based on job type from activity info

                        // TODO delete from required
                    }
                },
            );
        });

        // TODO propagate left required jobs to unassigned

        ctx.solution.routes = routes;
        ctx.solution
    }

    fn add(&mut self, activity_info: ActivityInfo) {
        assert_eq!(self.activity_direct_index.len(), self.activity_reverse_index.len());

        self.activity_direct_index.insert(activity_info.clone(), self.activity_direct_index.len());
        self.activity_reverse_index.insert(self.activity_reverse_index.len(), activity_info);
    }

    fn dimensions(&self) -> usize {
        self.activity_direct_index.len()
    }

    fn get_routes<T: AdjacencyMatrix>(&self, solution: &mut SolutionContext, matrix: &T) -> Vec<RouteContext> {
        let used_actors = solution.routes.iter().map(|r| r.route.actor.clone()).collect::<HashSet<_>>();
        let mut routes = solution.routes.clone();

        routes.extend(
            matrix
                .values()
                .map(|i| self.actor_reverse_index.get(&(i as usize)).cloned().unwrap())
                .filter(|a| used_actors.get(a).is_none())
                .map(|a| {
                    solution.registry.use_actor(&a);
                    RouteContext::new(a)
                }),
        );

        routes
    }

    fn get_activity_infos<T: AdjacencyMatrix>(
        &self,
        matrix: &T,
        actor_idx: usize,
        start_row_idx: usize,
    ) -> Vec<&ActivityInfo> {
        let mut next_row_idx = start_row_idx;
        let mut activity_infos = vec![];

        loop {
            if let Some(activity_info_idx) = matrix.scan_row(next_row_idx, |v| v == actor_idx as f64) {
                activity_infos.push(self.activity_reverse_index.get(&activity_info_idx).unwrap());
                next_row_idx = activity_info_idx;

                continue;
            }
            break;
        }

        // TODO scan activity infos to check that multi jobs are in allowed order.

        activity_infos
    }
}

fn get_unique_actor_details(actors: &Vec<Arc<Actor>>) -> Vec<ActorDetail> {
    let mut unique: HashSet<ActorDetail> = Default::default();
    let mut details = actors.iter().map(|a| a.detail.clone()).collect::<Vec<_>>();

    details.retain(|d| unique.insert(d.clone()));

    details
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
        None => ActivityInfo::Terminal((actor.detail.clone(), a.place.location)),
    }
}

/// Creates a fake single job with single place and single time window to avoid uncertainty
/// during insertion evaluation process.
fn create_single_job(activity_info: &ActivityInfo) -> Option<(Job, Arc<Single>)> {
    match activity_info {
        ActivityInfo::Job(activity_info) => {
            let (job, single_index, place_index, tw_index) = activity_info;
            let single = match job {
                Job::Single(single) => single.clone(),
                Job::Multi(multi) => multi.jobs.get(*single_index).cloned().unwrap(),
            };

            let place = single.places.get(*place_index).unwrap();
            let place = Place {
                location: place.location,
                duration: place.duration,
                times: vec![place.times.get(*tw_index).unwrap().clone()],
            };

            Some((job.clone(), Arc::new(Single { places: vec![place], dimens: single.dimens.clone() })))
        }
        ActivityInfo::Terminal(activity_info) => None,
    }
}

fn try_match_activity_place(activity: &TourActivity, places: &Vec<JobPlace>) -> Option<(usize, usize)> {
    (0_usize..).zip(places.iter()).fold(None, |acc, (place_idx, place)| {
        if acc.is_none() {
            if let Some(location) = place.location {
                if location == activity.place.location {
                    if activity.place.duration == place.duration {
                        for (tw_idx, tw) in (0_usize..).zip(place.times.iter()) {
                            if &activity.place.time == tw {
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
