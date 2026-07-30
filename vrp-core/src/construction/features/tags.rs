//! A job-vehicle tags feature.

#[cfg(test)]
#[path = "../../../tests/unit/construction/features/tags_test.rs"]
mod tags_test;

use super::*;
use std::collections::HashSet;

custom_dimension!(pub JobTags typeof HashSet<String>);
custom_dimension!(pub VehicleTags typeof HashSet<String>);

/// Creates a tags feature as hard constraint.
/// 
/// A job with tags can only be assigned to a vehicle that has all those tags.
/// If a job has no tags, any vehicle can service it.
pub fn create_tags_feature(name: &str, code: ViolationCode) -> Result<Feature, GenericError> {
    FeatureBuilder::default().with_name(name).with_constraint(TagsConstraint { code }).build()
}

struct TagsConstraint {
    code: ViolationCode,
}

impl FeatureConstraint for TagsConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => {
                if let Some(job_tags) = job.dimens().get_job_tags() {
                    if !job_tags.is_empty() {
                        let vehicle_tags = route_ctx.route().actor.vehicle.dimens.get_vehicle_tags();
                        
                        // Vehicle must have all job tags
                        let has_required_tags = match vehicle_tags {
                            Some(v_tags) => job_tags.is_subset(v_tags),
                            None => false,
                        };
                        
                        if !has_required_tags {
                            return ConstraintViolation::fail(self.code);
                        }
                    }
                }

                None
            }
            MoveContext::Activity { .. } => None,
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        let source_tags = source.dimens().get_job_tags();
        let candidate_tags = candidate.dimens().get_job_tags();

        let has_compatible_tags = match (source_tags, candidate_tags) {
            (None, None) | (Some(_), None) | (None, Some(_)) => true,
            (Some(source_tags), Some(candidate_tags)) => {
                // Both jobs must have the same tags to be merged
                source_tags == candidate_tags
            }
        };

        if has_compatible_tags { Ok(source) } else { Err(self.code) }
    }
}