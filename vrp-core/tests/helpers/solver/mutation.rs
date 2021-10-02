use super::*;
use crate::models::common::IdDimension;

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
