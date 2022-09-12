//! A feature to lock specific jobs to specific vehicles.

use super::*;
use crate::models::problem::{Actor, Fleet};
use crate::models::{Lock, LockOrder, LockPosition};
use hashbrown::HashMap;

type ConditionMap = HashMap<Job, Arc<dyn Fn(&Actor) -> bool + Sync + Send>>;

/// Creates a feature which allows to lock specific actors within specific jobs using different rules.
/// It is a hard constraint, so locking cannot be violated.
pub fn create_locked_jobs(fleet: &Fleet, locks: &[Arc<Lock>], code: ViolationCode) -> Result<Feature, String> {
    let (rules, conditions) = locks.iter().fold((Vec::new(), HashMap::new()), |(mut rules, mut conditions), lock| {
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

        (rules, conditions)
    });

    let rules = fleet.actors.iter().fold(HashMap::new(), |mut acc, actor| {
        acc.insert(actor.clone(), rules.iter().filter(|rule| (rule.condition)(actor)).cloned().collect());
        acc
    });

    FeatureBuilder::default().with_constraint(Arc::new(LockingConstraint { code, conditions, rules })).build()
}

struct LockingConstraint {
    code: ViolationCode,
    conditions: ConditionMap,
    rules: HashMap<Arc<Actor>, Vec<Arc<Rule>>>,
}

impl LockingConstraint {
    fn evaluate_route(&self, route_ctx: &RouteContext, job: &Job) -> Option<ConstraintViolation> {
        if let Some(condition) = self.conditions.get(job) {
            if !(condition)(&route_ctx.route.actor) {
                return ConstraintViolation::fail(self.code);
            }
        }

        None
    }

    fn evaluate_activity(
        &self,
        route_ctx: &RouteContext,
        activity_ctx: &ActivityContext,
    ) -> Option<ConstraintViolation> {
        if let Some(rules) = self.rules.get(&route_ctx.route.actor) {
            let can_insert = rules.iter().all(|rule| {
                rule.can_insert(
                    &activity_ctx.target.retrieve_job(),
                    &activity_ctx.prev.retrieve_job(),
                    &activity_ctx.next.and_then(|n| n.retrieve_job()),
                )
            });

            if !can_insert {
                return ConstraintViolation::skip(self.code);
            }
        }

        None
    }
}

impl FeatureConstraint for LockingConstraint {
    fn evaluate(&self, move_ctx: &MoveContext<'_>) -> Option<ConstraintViolation> {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.evaluate_route(route_ctx, job),
            MoveContext::Activity { route_ctx, activity_ctx } => self.evaluate_activity(route_ctx, activity_ctx),
        }
    }

    fn merge(&self, source: Job, candidate: Job) -> Result<Job, ViolationCode> {
        if self.conditions.contains_key(&candidate) {
            Err(self.code)
        } else {
            Ok(source)
        }
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
                LockPosition::Arrival => self.can_insert_before(prev, next),
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
