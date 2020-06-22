#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/area_test.rs"]
mod area_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::{ActivityContext, RouteContext, SolutionContext};
use crate::models::common::Location;
use crate::models::problem::{Actor, Job, Single};
use std::ops::Deref;
use std::slice::Iter;
use std::sync::Arc;

/// A function which returns operating area for given actor.
pub type AreaResolver = Arc<dyn Fn(&Actor) -> Option<&Vec<Vec<(f64, f64)>>> + Sync + Send>;
/// A function which returns actual coordinate for given location.
pub type LocationResolver = Arc<dyn Fn(Location) -> (f64, f64) + Sync + Send>;

/// An area module provides way to restrict given actor to work in specific areas only.
pub struct AreaModule {
    constraints: Vec<ConstraintVariant>,
    keys: Vec<i32>,
}

impl AreaModule {
    /// Creates a new instance of `AreaModule`.
    pub fn new(area_resolver: AreaResolver, location_resolver: LocationResolver, code: i32) -> Self {
        Self {
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(AreaHardRouteConstraint {
                    area_resolver: area_resolver.clone(),
                    location_resolver: location_resolver.clone(),
                    code,
                })),
                ConstraintVariant::HardActivity(Arc::new(AreaHardActivityConstraint {
                    area_resolver,
                    location_resolver,
                    code,
                })),
            ],
            keys: vec![],
        }
    }
}

impl ConstraintModule for AreaModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_ctx: &mut RouteContext, _job: &Job) {}

    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

struct AreaHardRouteConstraint {
    area_resolver: AreaResolver,
    location_resolver: LocationResolver,
    code: i32,
}

impl HardRouteConstraint for AreaHardRouteConstraint {
    fn evaluate_job(&self, _: &SolutionContext, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if let Some(areas) = self.area_resolver.deref()(&ctx.route.actor) {
            let can_serve = match job {
                Job::Single(job) => can_serve_job_in_area(job, areas, &self.location_resolver),
                Job::Multi(job) => {
                    job.jobs.iter().all(|single| can_serve_job_in_area(single, areas, &self.location_resolver))
                }
            };

            if !can_serve {
                return Some(RouteConstraintViolation { code: self.code });
            }
        }

        None
    }
}

struct AreaHardActivityConstraint {
    area_resolver: AreaResolver,
    location_resolver: LocationResolver,
    code: i32,
}

impl HardActivityConstraint for AreaHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        if let Some(areas) = self.area_resolver.deref()(&route_ctx.route.actor) {
            let location = self.location_resolver.deref()(activity_ctx.target.place.location);
            let can_serve = areas.iter().any(|area| is_location_in_area(&location, area));

            if !can_serve {
                // NOTE do not stop job insertion evaluation if it has multiple locations
                let stopped = activity_ctx
                    .target
                    .job
                    .as_ref()
                    .map_or(false, |job| job.places.iter().filter_map(|place| place.location).count() == 1);

                return Some(ActivityConstraintViolation { code: self.code, stopped });
            }
        }

        None
    }
}

/// Checks whether job can be served in given areas.
fn can_serve_job_in_area(job: &Single, areas: &[Vec<(f64, f64)>], location_resolver: &LocationResolver) -> bool {
    job.places
        .iter()
        .filter_map(|place| place.location)
        .map(|location| location_resolver.deref()(location))
        .any(|location| areas.iter().any(|area| is_location_in_area(&location, area)))
}

/// Checks whether given location is inside area using ray casting algorithm.
/// Location is interpreted as 2D point, area - as 2D polygon.
fn is_location_in_area(location: &(f64, f64), area: &[(f64, f64)]) -> bool {
    let &(x, y) = location;

    let mut is_inside = false;
    let mut i = 0;
    let mut j = area.len() - 1;

    while i < area.len() {
        let &(ix, iy) = area.get(i).unwrap();
        let &(jx, jy) = area.get(j).unwrap();

        if ((ix > x) != (jx > x)) && (y < (jy - iy) * (x - ix) / (jx - ix) + iy) {
            is_inside = !is_inside;
        }

        j = i;
        i += 1;
    }

    is_inside
}
