//! This module provides way to insert activity infos into existing route keeping solution feasible.
//!

use super::decipher::ActivityInfo;
use crate::construction::heuristics::{evaluate_job_insertion_in_route, InsertionPosition};
use crate::construction::states::{InsertionContext, InsertionResult, RouteContext};
use crate::models::problem::{Job, Place, Single};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Inserts jobs into tour taking care constraints.
pub struct ActivityInfoInserter<'a> {
    insertion_ctx: &'a mut InsertionContext,
    route_ctx: &'a mut RouteContext,
    unprocessed: &'a mut HashSet<Job>,
    unassigned: &'a mut HashSet<Job>,
    activity_infos: Vec<&'a ActivityInfo>,

    inserted_job_map: HashMap<Job, Vec<usize>>,
    planned_job_map: HashMap<Job, Vec<usize>>,
}

impl<'a> ActivityInfoInserter<'a> {
    pub fn new(
        insertion_ctx: &'a mut InsertionContext,
        route_ctx: &'a mut RouteContext,
        unprocessed: &'a mut HashSet<Job>,
        unassigned: &'a mut HashSet<Job>,
        activity_infos: Vec<&'a ActivityInfo>,
    ) -> Self {
        // TODO scan activity infos to check that multi jobs are in allowed order.
        //      if not, exclude these entries from collection.

        let planned_job_map = Self::get_multi_job_map(&activity_infos);
        Self {
            insertion_ctx,
            route_ctx,
            unprocessed,
            unassigned,
            activity_infos,
            inserted_job_map: Default::default(),
            planned_job_map,
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
        // TODO we call accept insertion on any job which might be to early for multi
        //      job case, so document this as it might be a bit unexpected for callee
        //      (it seems to be not a case at the moment).
        self.insertion_ctx.problem.constraint.accept_insertion(&mut self.insertion_ctx.solution, self.route_ctx, &job);

        let should_remove =
            job.as_multi().map_or(true, |multi| multi.jobs.len() == self.inserted_job_map.get(job).unwrap().len());

        if should_remove {
            self.unprocessed.remove(job);
        }
    }

    /// Removes all job activities from the tour and all its successors. Returns an index of last kept job.
    fn discard_insertion(&mut self, job: &Job, current_idx: usize) -> usize {
        let mut next_idx = current_idx + 1;
        match job {
            // NOTE keep activity info as it might be inserted if some multi job is deleted
            Job::Single(_) => next_idx,
            // NOTE remove everything after first sub job and remove multi job from the list
            Job::Multi(multi) => {
                if let Some(inserted) = self.inserted_job_map.get(job) {
                    let start = inserted.first().cloned().unwrap();
                    let end = self.route_ctx.route.tour.total() - 1;
                    let jobs = self.route_ctx.route_mut().tour.remove_activities_at(start, end);

                    self.unprocessed.extend(jobs.into_iter());
                }

                self.unprocessed.remove(job);
                self.unassigned.insert(job.clone());

                next_idx
            }
        }
    }

    /// Get multi jobs within their sub job insertion order.
    fn get_multi_job_map(activity_infos: &Vec<&ActivityInfo>) -> HashMap<Job, Vec<usize>> {
        activity_infos.iter().enumerate().fold(HashMap::new(), |mut acc, (ai_idx, ai)| {
            match ai {
                ActivityInfo::Job((job, single_idx, _, _)) => {
                    if let Some(_) = job.as_multi() {
                        acc.entry(job.clone()).or_insert_with(|| vec![]).push(*single_idx)
                    }
                }
                _ => {}
            }

            acc
        })
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
