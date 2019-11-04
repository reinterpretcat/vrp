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
pub struct StrictLockingModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConstraintModule for StrictLockingModule {
    fn accept_route_state(&self, _ctx: &mut RouteContext) {}

    fn accept_solution_state(&self, _ctx: &mut SolutionContext) {}

    fn state_keys(&self) -> Iter<i32> {
        self.state_keys.iter()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        self.constraints.iter()
    }
}

impl StrictLockingModule {
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
                ConstraintVariant::HardRoute(Arc::new(StrictLockingHardRouteConstraint { code, conditions })),
                ConstraintVariant::HardActivity(Arc::new(StrictLockingHardActivityConstraint {
                    code,
                    rules: actor_rules,
                })),
            ],
        }
    }
}

struct StrictLockingHardRouteConstraint {
    code: i32,
    conditions: HashMap<Arc<Job>, Arc<dyn Fn(&Actor) -> bool + Sync + Send>>,
}

impl HardRouteConstraint for StrictLockingHardRouteConstraint {
    fn evaluate_job(&self, ctx: &RouteContext, job: &Arc<Job>) -> Option<RouteConstraintViolation> {
        if let Some(condition) = self.conditions.get(job) {
            if !(condition)(&ctx.route.read().unwrap().actor) {
                return Some(RouteConstraintViolation { code: self.code });
            }
        }

        None
    }
}

struct StrictLockingHardActivityConstraint {
    code: i32,
    rules: HashMap<Arc<Actor>, Vec<Arc<Rule>>>,
}

impl HardActivityConstraint for StrictLockingHardActivityConstraint {
    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ActivityConstraintViolation> {
        let actor = &route_ctx.route.read().unwrap().actor;
        if let Some(rules) = self.rules.get(actor) {
            if !rules.iter().all(|rule| {
                rule.can_insert(&activity_ctx.prev.retrieve_job(), &activity_ctx.next.and_then(|n| n.retrieve_job()))
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
    condition: Arc<dyn Fn(&Actor) -> bool + Sync + Send>,
    /// Specifies lock position.
    position: LockPosition,
    /// Stores jobs.
    index: JobIndex,
}

impl Rule {
    fn contains(&self, job: &Arc<Job>) -> bool {
        self.index.jobs.contains(job)
    }

    /// Checks whether new job can be inserted between given according to rule's jobs.
    pub fn can_insert(&self, prev: &Option<Arc<Job>>, next: &Option<Arc<Job>>) -> bool {
        match self.position {
            LockPosition::Any => self.can_insert_after(&prev, &next) || self.can_insert_before(&prev, &next),
            LockPosition::Departure => self.can_insert_after(&prev, &next),
            LockPosition::Arrival => self.can_insert_before(&prev, &next),
            LockPosition::Fixed => false,
        }
    }

    /// Checks whether new job can be inserted between given after rule's jobs.
    fn can_insert_after(&self, prev: &Option<Arc<Job>>, next: &Option<Arc<Job>>) -> bool {
        prev.as_ref().map_or(false, |p| !self.contains(p) || p.clone() == self.index.last)
            && next.as_ref().map_or(true, |n| !self.contains(n))
    }

    /// Checks whether new job can be inserted between given before rule's jobs.
    fn can_insert_before(&self, prev: &Option<Arc<Job>>, next: &Option<Arc<Job>>) -> bool {
        next.as_ref().map_or(false, |n| !self.contains(n) || n.clone() == self.index.first)
            && prev.as_ref().map_or(true, |p| !self.contains(p))
    }
}
