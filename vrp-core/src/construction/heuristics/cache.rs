#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/cache_test.rs"]
mod cache_test;

use crate::construction::heuristics::*;
use crate::models::problem::{Actor, Job};
use hashbrown::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct CacheContext {
    lookup: Option<LookupTable>,
}

impl CacheContext {
    /// Injects a new empty cache into insertion context.
    pub fn inject(insertion_ctx: &mut InsertionContext) {
        insertion_ctx.solution.state.insert(INSERTION_CACHE, Arc::new(CacheContext::new()));
    }

    /// Removes cache from insertion context.
    pub fn remove(insertion_ctx: &mut InsertionContext) {
        insertion_ctx.solution.state.remove(&INSERTION_CACHE);
    }
}

impl CacheContext {
    /// Creates a new `CacheContext` with initialized lookup table.
    pub fn new() -> Self {
        Self { lookup: Some(LookupTable::default()) }
    }

    /// Evaluates insertion of the given route/job in a given position within cache.
    /// Automatically fallbacks and caches a new value.
    pub fn evaluate_insertion<F>(
        &self,
        insertion_ctx: &InsertionContext,
        eval_ctx: &EvaluationContext,
        route_ctx: &RouteContext,
        position: InsertionPosition,
        alternative: InsertionResult,
        fallback: F,
    ) -> InsertionResult
    where
        F: FnOnce(InsertionResult) -> InsertionResult,
    {
        if let Some(lookup) = self.lookup.as_ref() {
            let result = if let Some(result) = lookup.get(route_ctx, eval_ctx.job, &position) {
                result
            } else {
                // NOTE shouldn't use alternative here as we need to cache result for this job/route.
                let result = (fallback)(InsertionResult::make_failure());
                lookup.insert(route_ctx, eval_ctx.job.clone(), position, result.clone());
                result
            };

            eval_ctx.result_selector.select_insertion(insertion_ctx, result, alternative)
        } else {
            (fallback)(alternative)
        }
    }

    /// Cleans cache from evaluations for non-used routes.
    pub fn clean_routes(&self, insertion_ctx: &InsertionContext, routes: &[&RouteContext]) {
        if let Some(lookup) = self.lookup.as_ref() {
            routes
                .iter()
                .filter(|route_ctx| !insertion_ctx.solution.registry.is_used(route_ctx.route().actor.as_ref()))
                .for_each(|route_ctx| lookup.remove_route(route_ctx));
        }
    }

    /// Cleans cache for given route and job as result of insertion.
    pub fn accept_insertion(&self, route_ctx: &RouteContext, job: &Job) {
        if let Some(lookup) = self.lookup.as_ref() {
            lookup.remove_route(route_ctx);
            lookup.remove_job(job);
        }
    }

    /// Cleans cache for given job.
    pub fn accept_failure(&self, job: &Job) {
        if let Some(lookup) = self.lookup.as_ref() {
            lookup.remove_job(job);
        }
    }
}

impl From<&InsertionContext> for CacheContext {
    fn from(insertion_ctx: &InsertionContext) -> Self {
        insertion_ctx
            .solution
            .state
            .get(&INSERTION_CACHE)
            .and_then(|s| s.downcast_ref::<CacheContext>().cloned())
            .unwrap_or_default()
    }
}

#[derive(Clone, Default)]
struct LookupTable {
    table: Arc<RwLock<HashMap<Arc<Actor>, HashMap<Job, HashMap<InsertionPosition, InsertionResult>>>>>,
}

impl LookupTable {
    /// Gets entry if exists.
    pub fn get(&self, route_ctx: &RouteContext, job: &Job, position: &InsertionPosition) -> Option<InsertionResult> {
        self.table
            .read()
            .unwrap()
            .get(route_ctx.route().actor.as_ref())
            .and_then(|jobs| jobs.get(job))
            .and_then(|positions| positions.get(position))
            .cloned()
    }

    /// Inserts entry in lookup table. Replaces existing one.
    pub fn insert(&self, route_ctx: &RouteContext, job: Job, position: InsertionPosition, result: InsertionResult) {
        self.table
            .write()
            .unwrap()
            .entry(route_ctx.route().actor.clone())
            .or_insert_with(HashMap::default)
            .entry(job)
            .or_insert_with(HashMap::default)
            .insert(position, result);
    }

    /// Removes all data associated with given route from the table.
    pub fn remove_route(&self, route_ctx: &RouteContext) {
        self.table.write().unwrap().remove(&route_ctx.route().actor);
    }

    /// Removes all data associated with given job from the table.
    pub fn remove_job(&self, job: &Job) {
        self.table.write().unwrap().iter_mut().for_each(|(_, jobs)| {
            jobs.remove(job);
        });
    }
}
