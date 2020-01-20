//! Provides a way to represent VRP solution in adjacency matrix form (experimental).
//!

#[cfg(test)]
#[path = "../../../tests/unit/models/matrix/decipher_test.rs"]
mod decipher_test;

use super::AdjacencyMatrix;
use crate::construction::heuristics::{evaluate_job_insertion_in_route, InsertionPosition};
use crate::construction::states::{InsertionContext, InsertionResult, RouteContext, SolutionContext};
use crate::models::problem::{Actor, ActorDetail, Job, Place, Single};
use crate::models::solution::TourActivity;
use crate::models::Problem;
use crate::utils::DefaultRandom;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

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
    pub fn decode_feasible<T: AdjacencyMatrix>(&self, matrix: &T) -> SolutionContext {
        // NOTE A new context already contains routes with locked jobs which is important as
        // passed AM solution might ignore these rules.
        let mut ctx = InsertionContext::new(self.problem.clone(), Arc::new(DefaultRandom::default()));
        ctx.problem.constraint.accept_solution_state(&mut ctx.solution);

        let mut unprocessed = ctx.solution.required.iter().cloned().collect::<HashSet<_>>();
        let mut routes = self.get_routes(&mut ctx.solution, matrix);

        routes.iter_mut().for_each(|mut rc| {
            let actor = &rc.route.actor;
            let actor_idx = *self.actor_direct_index.get(actor).unwrap();

            let start_info = create_activity_info(actor, rc.route.tour.start().unwrap());
            let start_row_idx = *self.activity_direct_index.get(&start_info).unwrap();
            let activity_infos = self.get_activity_infos(matrix, actor_idx, start_row_idx);

            ActivityInfoInserter::new(&mut ctx, &mut rc, &mut unprocessed, activity_infos).insert();
        });

        // TODO propagate left required jobs to unassigned

        ctx.solution.routes = routes;
        ctx.solution
    }

    /// Decodes solution without checking feasibility.
    pub fn decode_vague<T: AdjacencyMatrix>(&self, _matrix: &T) -> SolutionContext {
        unimplemented!()
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
fn create_single_job(activity_info: &ActivityInfo) -> Option<(Job, Arc<Single>, usize)> {
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

            Some((job.clone(), Arc::new(Single { places: vec![place], dimens: single.dimens.clone() }), *single_index))
        }
        ActivityInfo::Terminal(_) => None,
    }
}

fn try_match_activity_place(activity: &TourActivity, places: &Vec<Place>) -> Option<(usize, usize)> {
    places.iter().enumerate().fold(None, |acc, (place_idx, place)| {
        if acc.is_none() {
            if let Some(location) = place.location {
                if location == activity.place.location {
                    if activity.place.duration == place.duration {
                        for (tw_idx, tw) in place.times.iter().enumerate() {
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

/// Inserts jobs into tour taking care constraints.
struct ActivityInfoInserter<'a> {
    insertion_ctx: &'a mut InsertionContext,
    route_ctx: &'a mut RouteContext,
    unprocessed: &'a mut HashSet<Job>,
    activity_infos: Vec<&'a ActivityInfo>,

    inserted_job_map: HashMap<Job, Vec<usize>>,
    multi_job_map: HashMap<Job, Vec<usize>>,
}

impl<'a> ActivityInfoInserter<'a> {
    pub fn new(
        insertion_ctx: &'a mut InsertionContext,
        route_ctx: &'a mut RouteContext,
        unprocessed: &'a mut HashSet<Job>,
        activity_infos: Vec<&'a ActivityInfo>,
    ) -> Self {
        let multi_job_map = Self::get_multi_job_map(&activity_infos);
        Self {
            insertion_ctx,
            route_ctx,
            unprocessed,
            activity_infos,
            inserted_job_map: Default::default(),
            multi_job_map,
        }
    }

    pub fn insert(&mut self) {
        let mut next_idx = 0_usize;
        loop {
            if let Some(activity_info) = self.activity_infos.get(next_idx) {
                if let Some((job, single, single_idx)) = create_single_job(activity_info) {
                    if self.unprocessed.contains(&job) {
                        if self.try_insert_single(&job, single, single_idx) {
                            self.accept_insertion(&job);
                        } else {
                            next_idx = self.discard_insertion(&job, next_idx);
                            continue;
                        }
                    }
                }
            } else {
                break;
            }

            next_idx = next_idx + 1;
        }
    }

    fn try_insert_single(&mut self, job: &Job, single: Arc<Single>, single_idx: usize) -> bool {
        let single = Job::Single(single);
        let result =
            evaluate_job_insertion_in_route(&single, self.insertion_ctx, self.route_ctx, InsertionPosition::Last, None);

        match result {
            InsertionResult::Success(success) => {
                assert_eq!(success.activities.len(), 1);
                let (mut activity, index) = success.activities.into_iter().next().unwrap();
                activity.job = job
                    .as_multi()
                    .and_then(|multi| multi.jobs.get(single_idx).cloned())
                    .or_else(|| activity.job.clone());

                self.route_ctx.route_mut().tour.insert_last(activity);
                self.inserted_job_map.entry(job.clone()).or_insert_with(|| vec![]).push(index);

                true
            }
            InsertionResult::Failure(_) => false,
        }
    }

    fn accept_insertion(&mut self, job: &Job) {
        // TODO we call accept insertion on job which might be to early for multi
        //      job case, so document this as it might be unexpected for callee
        self.insertion_ctx.problem.constraint.accept_insertion(&mut self.insertion_ctx.solution, self.route_ctx, &job);

        let should_remove =
            job.as_multi().map_or(true, |multi| multi.jobs.len() == self.inserted_job_map.get(job).unwrap().len());

        if should_remove {
            self.unprocessed.remove(job);
        }
    }

    /// Removes all job activities from the tour and all its successors. Returns an index of last kept job.
    fn discard_insertion(&mut self, job: &Job, current_idx: usize) -> usize {
        unimplemented!()
    }

    /// Get multi jobs within their sub job insertion order.
    fn get_multi_job_map(activity_infos: &Vec<&ActivityInfo>) -> HashMap<Job, Vec<usize>> {
        activity_infos.iter().enumerate().fold(HashMap::new(), |mut acc, (ai_idx, ai)| {
            match ai {
                ActivityInfo::Job((job, single_idx, _, _)) => {
                    if let Some(multi) = job.as_multi() {
                        acc.entry(job.clone()).or_insert_with(|| vec![]).push(*single_idx)
                    }
                }
                _ => {}
            }

            acc
        })
    }
}
