//! Provides a feature to minimize total overdue days for scheduled jobs.

use super::*;

/// Seconds per day constant for converting timestamp difference to days.
const SECONDS_PER_DAY: Float = 86400.0;

/// A function type to extract due date from a job.
pub type JobDueDateFn = Arc<dyn Fn(&Job) -> Option<Float> + Send + Sync>;

/// A function type to extract scheduled date from route context.
pub type ScheduledDateFn = Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>;

/// A function type to calculate penalty for unassigned overdue jobs.
/// Takes the job and returns the overdue penalty in days.
pub type UnassignedOverduePenaltyFn = Arc<dyn Fn(&Job) -> Float + Send + Sync>;

/// Provides a way to build a feature to minimize overdue.
pub struct MinimizeOverdueBuilder {
    name: String,
    job_due_date_fn: Option<JobDueDateFn>,
    scheduled_date_fn: Option<ScheduledDateFn>,
    unassigned_penalty_fn: Option<UnassignedOverduePenaltyFn>,
}

impl MinimizeOverdueBuilder {
    /// Creates a new instance of `MinimizeOverdueBuilder`.
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), job_due_date_fn: None, scheduled_date_fn: None, unassigned_penalty_fn: None }
    }

    /// Sets the function to extract due date from a job.
    pub fn set_job_due_date_fn<F>(mut self, func: F) -> Self
    where
        F: Fn(&Job) -> Option<Float> + Send + Sync + 'static,
    {
        self.job_due_date_fn = Some(Arc::new(func));
        self
    }

    /// Sets the function to extract scheduled date from route context.
    pub fn set_scheduled_date_fn<F>(mut self, func: F) -> Self
    where
        F: Fn(&RouteContext) -> Float + Send + Sync + 'static,
    {
        self.scheduled_date_fn = Some(Arc::new(func));
        self
    }

    /// Sets the function to calculate penalty for unassigned overdue jobs.
    /// This function should return the overdue penalty in days for jobs that are not scheduled.
    /// If not set, unassigned jobs will not contribute to the overdue penalty.
    pub fn set_unassigned_penalty_fn<F>(mut self, func: F) -> Self
    where
        F: Fn(&Job) -> Float + Send + Sync + 'static,
    {
        self.unassigned_penalty_fn = Some(Arc::new(func));
        self
    }

    /// Builds the feature.
    pub fn build(mut self) -> GenericResult<Feature> {
        let job_due_date_fn =
            self.job_due_date_fn.take().ok_or_else(|| GenericError::from("job_due_date_fn must be set"))?;

        let scheduled_date_fn =
            self.scheduled_date_fn.take().ok_or_else(|| GenericError::from("scheduled_date_fn must be set"))?;

        let unassigned_penalty_fn = self.unassigned_penalty_fn.take();

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_objective(MinimizeOverdueObjective { job_due_date_fn, scheduled_date_fn, unassigned_penalty_fn })
            .build()
    }
}

struct MinimizeOverdueObjective {
    job_due_date_fn: JobDueDateFn,
    scheduled_date_fn: ScheduledDateFn,
    unassigned_penalty_fn: Option<UnassignedOverduePenaltyFn>,
}

impl MinimizeOverdueObjective {
    /// Calculates overdue in days for a single job.
    fn calculate_overdue(&self, route_ctx: &RouteContext, job: &Job) -> Float {
        let due_date = match (self.job_due_date_fn)(job) {
            Some(date) => date,
            None => return 0.0, // No due date means no overdue
        };

        let scheduled_date = (self.scheduled_date_fn)(route_ctx);

        // Overdue = how many days past due date the job is scheduled
        // If scheduled before due date, overdue is 0
        let diff_seconds = scheduled_date - due_date;
        if diff_seconds <= 0.0 { 0.0 } else { diff_seconds / SECONDS_PER_DAY }
    }
}

impl FeatureObjective for MinimizeOverdueObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        // Calculate overdue for scheduled jobs
        let scheduled_overdue: Cost = solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| route_ctx.route().tour.jobs().map(|job| self.calculate_overdue(route_ctx, job)))
            .sum();

        // Calculate penalty for unassigned overdue jobs
        let unassigned_overdue: Cost = match &self.unassigned_penalty_fn {
            Some(penalty_fn) => solution.solution.unassigned.keys().map(|job| (penalty_fn)(job)).sum(),
            None => 0.0,
        };

        scheduled_overdue + unassigned_overdue
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => {
                let scheduled_overdue = self.calculate_overdue(route_ctx, job);

                // If we have an unassigned penalty function, assigning this job
                // removes its unassigned penalty (negative delta)
                let unassigned_penalty_delta = match &self.unassigned_penalty_fn {
                    Some(penalty_fn) => -(penalty_fn)(job),
                    None => 0.0,
                };

                scheduled_overdue + unassigned_penalty_delta
            }
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}
