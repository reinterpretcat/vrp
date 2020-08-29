#[cfg(test)]
#[path = "../../../tests/unit/construction/constraints/locking_test.rs"]
mod locking_test;

use crate::construction::constraints::*;
use crate::construction::heuristics::{ActivityContext, RouteContext, SolutionContext};
use crate::models::problem::{Actor, Fleet, Job};
use crate::models::{Lock, LockOrder, LockPosition};
use hashbrown::{HashMap, HashSet};
use std::slice::Iter;
use std::sync::Arc;

/// A module which allows to lock specific actors within specific jobs using different rules.
pub struct StrictLockingModule {
    state_keys: Vec<i32>,
    constraints: Vec<ConstraintVariant>,
}

impl ConstraintModule for StrictLockingModule {
    fn accept_insertion(&self, _solution_ctx: &mut SolutionContext, _route_index: usize, _job: &Job) {}

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
    /// Creates an instance of `StrictLockingModule`.
    pub fn new(fleet: &Fleet, locks: &[Arc<Lock>], code: i32) -> Self {
        let mut rules = vec![];
        let mut conditions = HashMap::new();
        locks.iter().for_each(|lock| {
            let condition = lock.condition.clone();
            lock.details.iter().for_each(|detail| {
                // NOTE create rule only for strict order
                if let LockOrder::Strict = detail.order {
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

                detail.jobs.iter().cloned().collect::<HashSet<Job>>().into_iter().for_each(|job| {
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
    conditions: HashMap<Job, Arc<dyn Fn(&Actor) -> bool + Sync + Send>>,
}

impl HardRouteConstraint for StrictLockingHardRouteConstraint {
    fn evaluate_job(&self, _: &SolutionContext, ctx: &RouteContext, job: &Job) -> Option<RouteConstraintViolation> {
        if let Some(condition) = self.conditions.get(job) {
            if !(condition)(&ctx.route.actor) {
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
        if let Some(rules) = self.rules.get(&route_ctx.route.actor) {
            let can_insert = rules.iter().all(|rule| {
                rule.can_insert(
                    &activity_ctx.target.retrieve_job(),
                    &activity_ctx.prev.retrieve_job(),
                    &activity_ctx.next.and_then(|n| n.retrieve_job()),
                )
            });

            if !can_insert {
                return Some(ActivityConstraintViolation { code: self.code, stopped: false });
            }
        }

        None
    }
}

struct JobIndex {
    first: Job,
    last: Job,
    jobs: HashSet<Job>,
}

/// Represents a rule created from lock model.
struct Rule {
    condition: Arc<dyn Fn(&Actor) -> bool + Sync + Send>,
    position: LockPosition,
    index: JobIndex,
}

impl Rule {
    /// Checks whether a new job can be inserted between given prev/next according to rules.
    pub fn can_insert(&self, job: &Option<Job>, prev: &Option<Job>, next: &Option<Job>) -> bool {
        self.is_in_rule(job)
            || match self.position {
                LockPosition::Any => self.can_insert_after(prev, next) || self.can_insert_before(prev, next),
                LockPosition::Departure => self.can_insert_after(prev, next),
                LockPosition::Arrival => self.can_insert_before(prev, &next),
                LockPosition::Fixed => false,
            }
    }

    fn contains(&self, job: &Job) -> bool {
        self.index.jobs.contains(job)
    }

    /// Checks whether given job is in rule. Such jobs are inserted manually and should not by
    /// prevented from insertion.
    fn is_in_rule(&self, job: &Option<Job>) -> bool {
        job.as_ref().map_or(false, |job| self.contains(job))
    }

    /// Checks whether a new job can be inserted between given prev/next according to after rule.
    fn can_insert_after(&self, prev: &Option<Job>, next: &Option<Job>) -> bool {
        prev.as_ref().map_or(false, |p| !self.contains(p) || *p == self.index.last)
            && next.as_ref().map_or(true, |n| !self.contains(n))
    }

    /// Checks whether a new job can be inserted between given prev/next according to before rule.
    fn can_insert_before(&self, prev: &Option<Job>, next: &Option<Job>) -> bool {
        next.as_ref().map_or(false, |n| !self.contains(n) || *n == self.index.first)
            && prev.as_ref().map_or(true, |p| !self.contains(p))
    }
}
