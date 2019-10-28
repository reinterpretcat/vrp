#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/locking_test.rs"]
mod locking_test;

use crate::construction::constraints::*;
use crate::construction::states::{ActivityContext, RouteContext, SolutionContext};
use crate::models::problem::{Actor, Fleet, Job};
use crate::models::{Lock, LockOrder, LockPosition};
use std::collections::{HashMap, HashSet};
use std::slice::Iter;
use std::sync::Arc;

/// Allows to lock specific actors within specific jobs using different rules.
pub struct LockingModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConstraintModule for LockingModule {
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        unimplemented!()
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        unimplemented!()
    }

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl LockingModule {
    pub fn new(fleet: &Fleet, locks: Vec<Arc<Lock>>, code: i32) -> Self {
        let mut rules = vec![];
        let mut conditions = HashMap::new();
        locks.iter().for_each(|lock| {
            let condition = lock.condition.clone();
            lock.details.iter().for_each(|detail| {
                // NOTE create rule only for strict order
                match detail.order {
                    LockOrder::Strict => {
                        assert!(!detail.jobs.is_empty());
                        rules.push(Arc::new(Rule {
                            condition: condition.clone(),
                            position: detail.position.clone(),
                            index: JobIndex {
                                first: detail.jobs.first().unwrap().clone(),
                                last: detail.jobs.last().unwrap().clone(),
                                jobs: detail.jobs.iter().cloned().collect(),
                            },
                        }));
                    }
                    _ => {}
                }

                detail.jobs.iter().cloned().collect::<HashSet<Arc<Job>>>().into_iter().for_each(|job| {
                    conditions.insert(job, condition.clone());
                });
            });
        });

        let mut actor_rules = HashMap::new();
        fleet.actors.iter().for_each(|actor| {
            actor_rules.insert(actor.clone(), rules.iter().filter(|rule| (rule.condition)(actor)).cloned().collect());
        });

        Self {
            state_keys: vec![],
            constraints: vec![
                ConstraintVariant::HardRoute(Arc::new(LockingHardRouteConstraint { code, conditions })),
                ConstraintVariant::HardActivity(Arc::new(LockingHardActivityConstraint { code, rules: actor_rules })),
            ],
        }
    }
}

struct LockingHardRouteConstraint {
    code: i32,
    conditions: HashMap<Arc<Job>, Arc<dyn Fn(&Arc<Actor>) -> bool + Sync + Send>>,
}

impl HardRouteConstraint for LockingHardRouteConstraint {
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation> {
        if let Some(condition) = self.conditions.get(job) {
            if !(condition)(&ctx.route.read().unwrap().actor) {
                return Some(RouteConstraintViolation { code: self.code });
            }
        }

        None
    }
}

struct LockingHardActivityConstraint {
    code: i32,
    rules: HashMap<Arc<Actor>, Vec<Arc<Rule>>>,
}

impl HardActivityConstraint for LockingHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let actor = &route_ctx.route.read().unwrap().actor;
        if let Some(rules) = self.rules.get(actor) {
            if !rules.iter().all(|rule| match rule.position {
                LockPosition::Any => {
                    let has_prev = activity_ctx.prev.retrieve_job().map_or(false, |job| rule.index.first == job);
                    let has_next =
                        activity_ctx.next.and_then(|n| n.retrieve_job()).map_or(false, |job| rule.index.last == job);

                    has_prev && has_next
                }
                LockPosition::Departure => activity_ctx.prev.retrieve_job().is_none(),
                LockPosition::Arrival => activity_ctx.next.map_or(false, |n| n.retrieve_job().is_none()),
                LockPosition::Fixed => {
                    activity_ctx.prev.retrieve_job().is_none()
                        && activity_ctx.next.map_or(false, |n| n.retrieve_job().is_none())
                }
            }) {
                return Some(ActivityConstraintViolation { code: self.code, stopped: false });
            }
        }

        None
    }
}

struct JobIndex {
    first: Arc<Job>,
    last: Arc<Job>,
    jobs: HashSet<Arc<Job>>,
}

/// Represents a rule created from lock model.
struct Rule {
    /// Specifies condition when locked jobs can be assigned to specific actor.
    condition: Arc<dyn Fn(&Arc<Actor>) -> bool + Sync + Send>,
    /// Specifies lock position.
    position: LockPosition,
    /// Stores jobs.
    index: JobIndex,
}
