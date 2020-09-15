///! Contains naive branching muration operator.
use super::*;
use crate::utils::parallel_into_collect;
use std::cmp::Ordering;
use std::ops::Range;

/// A mutation operator which uses naive branching strategy to avoid local optimum.
pub struct NaiveBranching {
    inner: Arc<dyn Mutation + Send + Sync>,
    normal_chance: f64,
    intensive_chance: f64,
    threshold: f64,
    steepness: f64,
    generations: Range<usize>,
}

impl NaiveBranching {
    /// Creates a new instance of `NaiveBranching`.
    pub fn new(
        inner: Arc<dyn Mutation + Send + Sync>,
        chance: (f64, f64, f64),
        steepness: f64,
        generations: Range<usize>,
    ) -> Self {
        Self { inner, normal_chance: chance.0, intensive_chance: chance.1, threshold: chance.2, steepness, generations }
    }
}

impl Mutation for NaiveBranching {
    fn mutate_one(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let branching_chance = self.get_branching_chance(refinement_ctx);

        let is_branch = insertion_ctx.random.uniform_real(0., 1.) < branching_chance;
        if is_branch {
            let random = insertion_ctx.random.clone();
            let (min, max) = (self.generations.start as i32, self.generations.end as i32);
            let gens = random.uniform_int(min, max) as usize;
            (1_usize..=gens).fold(insertion_ctx, |parent, idx| {
                let child = self.inner.mutate_one(&refinement_ctx, parent.deep_copy());

                let use_worse_chance = random.uniform_real(0., 1.);
                let use_worse_probability = get_use_worse_probability(idx, gens, self.steepness);
                let is_child_better = refinement_ctx.population.cmp(&child, &parent) == Ordering::Less;

                if use_worse_chance < use_worse_probability || is_child_better {
                    child
                } else {
                    parent
                }
            })
        } else {
            self.inner.mutate_one(&refinement_ctx, insertion_ctx)
        }
    }

    fn mutate_all(
        &self,
        refinement_ctx: &RefinementContext,
        individuals: Vec<InsertionContext>,
    ) -> Vec<InsertionContext> {
        parallel_into_collect(individuals, |insertion_ctx| self.mutate_one(refinement_ctx, insertion_ctx))
    }
}

impl NaiveBranching {
    fn get_branching_chance(&self, refinement_ctx: &RefinementContext) -> f64 {
        if refinement_ctx.statistics.improvement_1000_ratio < self.threshold {
            self.intensive_chance
        } else {
            self.normal_chance
        }
    }
}

fn get_use_worse_probability(current: usize, total: usize, steepness: f64) -> f64 {
    1. - (current as f64 / total as f64).powf(steepness)
}
