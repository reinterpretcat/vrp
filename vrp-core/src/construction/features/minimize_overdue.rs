//! Provides a feature to minimize total overdue days for scheduled jobs.

use super::*;

/// Seconds per day constant for converting timestamp difference to days.
const SECONDS_PER_DAY: Float = 86400.0;

/// A function type to extract due date from a job.
pub type JobDueDateFn = Arc<dyn Fn(&Job) -> Option<Float> + Send + Sync>;

/// A function type to extract scheduled date from route context.
pub type ScheduledDateFn = Arc<dyn Fn(&RouteContext) -> Float + Send + Sync>;

/// Provides a way to build a feature to minimize overdue.
pub struct MinimizeOverdueBuilder {
    name: String,
    job_due_date_fn: Option<JobDueDateFn>,
    scheduled_date_fn: Option<ScheduledDateFn>,
}

impl MinimizeOverdueBuilder {
    /// Creates a new instance of `MinimizeOverdueBuilder`.
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), job_due_date_fn: None, scheduled_date_fn: None }
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

    /// Builds the feature.
    pub fn build(mut self) -> GenericResult<Feature> {
        let job_due_date_fn =
            self.job_due_date_fn.take().ok_or_else(|| GenericError::from("job_due_date_fn must be set"))?;

        let scheduled_date_fn =
            self.scheduled_date_fn.take().ok_or_else(|| GenericError::from("scheduled_date_fn must be set"))?;

        FeatureBuilder::default()
            .with_name(self.name.as_str())
            .with_objective(MinimizeOverdueObjective { job_due_date_fn, scheduled_date_fn })
            .build()
    }
}

struct MinimizeOverdueObjective {
    job_due_date_fn: JobDueDateFn,
    scheduled_date_fn: ScheduledDateFn,
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
        if diff_seconds <= 0.0 {
            0.0
        } else {
            diff_seconds / SECONDS_PER_DAY
        }
    }
}

impl FeatureObjective for MinimizeOverdueObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        solution
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| route_ctx.route().tour.jobs().map(|job| self.calculate_overdue(route_ctx, job)))
            .sum::<Cost>()
    }

    fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
        match move_ctx {
            MoveContext::Route { route_ctx, job, .. } => self.calculate_overdue(route_ctx, job),
            MoveContext::Activity { .. } => Cost::default(),
        }
    }
}
