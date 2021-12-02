use super::*;
use crate::construction::constraints::*;
use crate::construction::heuristics::*;
use crate::models::common::IdDimension;
use crate::models::solution::Activity;
use crate::utils::as_mut;
use hashbrown::HashMap;

/// Promotes given job ids to locked in given context.
pub fn promote_to_locked(mut insertion_ctx: InsertionContext, job_ids: &[&str]) -> InsertionContext {
    let ids = insertion_ctx.problem.jobs.all().filter(|job| job_ids.contains(&job.dimens().get_id().unwrap().as_str()));
    insertion_ctx.solution.locked.extend(ids);

    insertion_ctx
}

/// Compares given strings as ids using ignore id.
pub fn compare_with_ignore(left: &[Vec<String>], right: &[Vec<&str>], ignore: &str) {
    if left.len() != right.len() {
        assert_eq!(left, right);
    }

    left.iter().zip(right.iter()).for_each(|(a_vec, b_vec)| {
        if a_vec.len() != b_vec.len() {
            assert_eq!(left, right);
        }

        a_vec.iter().zip(b_vec.iter()).for_each(|(a_value, b_value)| {
            if a_value != ignore && *b_value != ignore && a_value != b_value {
                assert_eq!(left, right);
            }
        });
    })
}

/// Get jobs from insertion context with given ids in the given order.
pub fn get_jobs_by_ids(insertion_ctx: &InsertionContext, job_ids: &[&str]) -> Vec<Job> {
    let mut ids = insertion_ctx
        .problem
        .jobs
        .all()
        .filter_map(|job| {
            let job_id = job.dimens().get_id().unwrap().clone();
            if job_ids.contains(&job_id.as_str()) {
                Some((job_id, job))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let map = job_ids.iter().enumerate().map(|(idx, job)| (*job, idx)).collect::<HashMap<_, _>>();
    ids.sort_by(|(a, _), (b, _)| map.get(a.as_str()).unwrap().cmp(map.get(b.as_str()).unwrap()));

    let (_, jobs): (Vec<_>, Vec<_>) = ids.into_iter().unzip();

    jobs
}

/// Adds hard activity constraint on specific route legs.
pub fn add_leg_constraint(problem: &mut Problem, disallowed_pairs: Vec<(&str, &str)>) {
    let disallowed_pairs =
        disallowed_pairs.into_iter().map(|(prev, next)| (prev.to_string(), next.to_string())).collect();
    unsafe { as_mut(problem.constraint.as_ref()) }.add_constraint(&ConstraintVariant::HardActivity(Arc::new(
        LegConstraint::new(disallowed_pairs, "cX".to_string()),
    )));
}

struct LegConstraint {
    ignore: String,
    disallowed_pairs: Vec<(String, String)>,
}

impl HardActivityConstraint for LegConstraint {
    fn evaluate_activity(
        &self,
        _: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let retrieve_job_id = |activity: Option<&Activity>| {
            activity.as_ref().and_then(|next| {
                next.retrieve_job().and_then(|job| job.dimens().get_id().cloned()).or_else(|| Some(self.ignore.clone()))
            })
        };

        retrieve_job_id(Some(activity_ctx.prev)).zip(retrieve_job_id(activity_ctx.next)).and_then(|(prev, next)| {
            let is_disallowed = self.disallowed_pairs.iter().any(|(p_prev, p_next)| {
                let is_left_match = p_prev == &prev || p_prev == &self.ignore;
                let is_right_match = p_next == &next || p_next == &self.ignore;

                is_left_match && is_right_match
            });

            if is_disallowed {
                Some(ActivityConstraintViolation { code: 7, stopped: false })
            } else {
                None
            }
        })
    }
}

impl LegConstraint {
    fn new(disallowed_pairs: Vec<(String, String)>, ignore: String) -> Self {
        Self { disallowed_pairs, ignore }
    }
}
