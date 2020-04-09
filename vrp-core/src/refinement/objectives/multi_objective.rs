use super::*;
use std::ops::Deref;

/// A multi objective cost.
pub struct MultiObjectiveCost {
    primary_costs: ObjectiveCosts,
    secondary_costs: ObjectiveCosts,
    value_func: ObjectiveCostValueFn,
}

/// Encapsulates objective which has multiple objectives.
pub struct MultiObjective {
    /// List of primary objectives. Solution can be considered as improvement
    /// only if none of costs, returned by these objectives, is worse.
    primary_objectives: Vec<Box<dyn Objective + Send + Sync>>,
    /// List of secondary objectives. This list is evaluated only if primary objectives
    /// costs are considered as equal.
    secondary_objectives: Vec<Box<dyn Objective + Send + Sync>>,
    /// A function which extract actual cost from multiple objective costs.
    value_func: ObjectiveCostValueFn,
}

impl ObjectiveCost for MultiObjectiveCost {
    fn value(&self) -> f64 {
        self.value_func.deref()(&self.primary_costs, &self.secondary_costs)
    }

    fn cmp_relaxed(&self, other: &ObjectiveCostType) -> (Ordering, Ordering) {
        let (primary_costs, secondary_costs) = self.get_costs(other);

        let (result, _) = match Self::analyze(&self.primary_costs, primary_costs, 0) {
            (Equal, relaxed_count) => Self::analyze(&self.secondary_costs, secondary_costs, relaxed_count),
            result_pair => result_pair,
        };

        (result, result)
    }

    fn clone_box(&self) -> ObjectiveCostType {
        Box::new(Self {
            primary_costs: self.primary_costs.iter().map(|c| c.clone_box()).collect(),
            secondary_costs: self.secondary_costs.iter().map(|c| c.clone_box()).collect(),
            value_func: self.value_func.clone(),
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MultiObjectiveCost {
    /// Creates a new instance of `MultiObjectiveCost`.
    pub fn new(
        primary_costs: ObjectiveCosts,
        secondary_costs: ObjectiveCosts,
        value_func: ObjectiveCostValueFn,
    ) -> Self {
        Self { primary_costs, secondary_costs, value_func }
    }

    fn get_costs<'a>(&self, other: &'a ObjectiveCostType) -> (&'a Vec<ObjectiveCostType>, &'a Vec<ObjectiveCostType>) {
        let other = other.as_any().downcast_ref::<MultiObjectiveCost>().expect("Expecting MultiObjectiveCost");

        let primary_costs = &other.primary_costs;
        assert_eq!(self.primary_costs.len(), primary_costs.len());

        let secondary_costs = &other.secondary_costs;
        assert_eq!(self.secondary_costs.len(), secondary_costs.len());

        (primary_costs, secondary_costs)
    }

    fn analyze(left: &[ObjectiveCostType], right: &[ObjectiveCostType], relaxed_count: usize) -> (Ordering, usize) {
        // NOTE Allow not more than one objective to be relaxed at the same time
        const MAX_RELAXED_COUNT: usize = 1;

        let results = left.iter().zip(right.iter()).map(|(left, right)| left.cmp_relaxed(right)).collect::<Vec<_>>();

        let relaxed_count = results.iter().filter(|(a, r)| *a == Greater && *r == Equal).count() + relaxed_count;
        let result_actual = Self::analyze_results(results.iter().map(|(a, _)| *a));
        let result_relaxed = Self::analyze_results(results.iter().map(|(_, r)| *r));

        let result = match (result_actual, result_relaxed) {
            (Less, _) => Less,
            (_, relaxed) if relaxed_count <= MAX_RELAXED_COUNT => relaxed,
            _ => Greater,
        };

        (result, relaxed_count)
    }

    fn analyze_results(results: impl Iterator<Item = Ordering>) -> Ordering {
        results.fold(Equal, |acc, result| match (acc, result) {
            (Equal, new) => new,
            (Less, Greater) => Greater,
            (Less, _) => Less,
            (Greater, _) => Greater,
        })
    }
}

impl MultiObjective {
    /// Creates a new instance of `MultiObjective`.
    pub fn new(
        primary_objectives: Vec<Box<dyn Objective + Send + Sync>>,
        secondary_objectives: Vec<Box<dyn Objective + Send + Sync>>,
        value_func: ObjectiveCostValueFn,
    ) -> Self {
        assert!(!primary_objectives.is_empty() || !secondary_objectives.is_empty());
        Self { primary_objectives, secondary_objectives, value_func }
    }
}

impl Default for MultiObjective {
    fn default() -> Self {
        Self {
            primary_objectives: vec![Box::new(TotalRoutes::default()), Box::new(TotalUnassignedJobs::default())],
            secondary_objectives: vec![Box::new(TotalTransportCost::default())],
            value_func: Arc::new(|_, secondary| secondary.first().unwrap().value()),
        }
    }
}

impl Objective for MultiObjective {
    fn estimate_cost(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> ObjectiveCostType {
        let primary_costs =
            self.primary_objectives.iter().map(|o| o.estimate_cost(refinement_ctx, insertion_ctx)).collect::<Vec<_>>();
        let secondary_costs = self
            .secondary_objectives
            .iter()
            .map(|o| o.estimate_cost(refinement_ctx, insertion_ctx))
            .collect::<Vec<_>>();

        Box::new(MultiObjectiveCost::new(primary_costs, secondary_costs, self.value_func.clone()))
    }

    fn is_goal_satisfied(
        &self,
        refinement_ctx: &mut RefinementContext,
        insertion_ctx: &InsertionContext,
    ) -> Option<bool> {
        let mut get_satisfaction = |objectives: &Vec<Box<dyn Objective + Send + Sync>>| {
            objectives.iter().filter_map(|o| o.is_goal_satisfied(refinement_ctx, insertion_ctx)).collect::<Vec<_>>()
        };

        let mut results = get_satisfaction(&self.primary_objectives);

        if results.is_empty() {
            results.extend(get_satisfaction(&self.secondary_objectives).into_iter())
        }

        if results.is_empty() {
            None
        } else {
            Some(results.iter().all(|&goal_satisfied| goal_satisfied))
        }
    }
}
