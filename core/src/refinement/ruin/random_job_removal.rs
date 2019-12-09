use crate::construction::states::InsertionContext;
use crate::refinement::ruin::Ruin;
use crate::refinement::RefinementContext;

/// Removes random jobs from solution.
pub struct RandomJobRemoval {
    /// Specifies minimum amount of removed jobs.
    min: f64,
    /// Specifies maximum amount of removed jobs.
    max: f64,
    /// Specifies threshold ratio of maximum removed jobs.
    threshold: f64,
}

impl RandomJobRemoval {
    pub fn new(min: usize, max: usize, threshold: f64) -> Self {
        Self { min: min as f64, max: max as f64, threshold }
    }
}

impl Default for RandomJobRemoval {
    fn default() -> Self {
        Self::new(1, 10, 0.2)
    }
}

impl Ruin for RandomJobRemoval {
    fn run(&self, _refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        let mut insertion_ctx = insertion_ctx;

        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let assigned = insertion_ctx.problem.jobs.size()
            - insertion_ctx.solution.unassigned.len()
            - insertion_ctx.solution.ignored.len();
        let max = (assigned as f64 * self.threshold).max(self.min).round() as usize;
        let affected =
            insertion_ctx.random.uniform_int(self.min as i32, self.max as i32).min(assigned.min(max) as i32) as usize;

        (0..affected).for_each(|_| {
            let solution = &mut insertion_ctx.solution;
            let route_index = insertion_ctx.random.uniform_int(0, solution.routes.len() as i32 - 1) as usize;

            let route = solution.routes.get_mut(route_index).unwrap().route_mut();

            if route.tour.job_count() > 0 {
                let job_index = insertion_ctx.random.uniform_int(0, route.tour.job_count() as i32 - 1) as usize;
                let job = route.tour.jobs().skip(job_index).next().unwrap();

                if !solution.locked.contains(&job) {
                    route.tour.remove(&job);
                    solution.required.push(job);
                }
            }
        });

        insertion_ctx
    }
}
