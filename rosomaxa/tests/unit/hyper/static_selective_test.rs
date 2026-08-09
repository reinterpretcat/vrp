use super::*;
use crate::example::{VectorContext, VectorObjective, VectorSolution};
use crate::helpers::example::create_default_heuristic_context;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Noop;

impl HeuristicSearchOperator for Noop {
    type Context = VectorContext;
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        solution.deep_copy()
    }
}

impl HeuristicDiversifyOperator for Noop {
    type Context = VectorContext;
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn diversify(&self, _: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        vec![solution.deep_copy()]
    }
}

struct CountingIntensify {
    count: Arc<AtomicUsize>,
}

impl HeuristicIntensifyOperator for CountingIntensify {
    type Context = VectorContext;
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn intensify(&self, _: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        self.count.fetch_add(1, Ordering::Relaxed);
        vec![solution.deep_copy()]
    }
}

#[test]
fn can_run_intensify_operator() {
    let search_group: HeuristicSearchGroup<VectorContext, VectorObjective, VectorSolution> =
        vec![(Arc::new(Noop), (Box::new(|_, _| true), PhantomData))];
    let diversify_group: HeuristicDiversifyGroup<VectorContext, VectorObjective, VectorSolution> = vec![Arc::new(Noop)];
    let count = Arc::new(AtomicUsize::new(0));
    let heuristic = StaticSelective::new(search_group)
        .with_diversify_operators(diversify_group)
        .with_intensify_operators(vec![Arc::new(CountingIntensify { count: count.clone() })]);
    let solution = VectorSolution::new(vec![0., 0.], 0., vec![0., 0.]);

    let offspring = heuristic.intensify_many(&create_default_heuristic_context(), vec![&solution]);

    assert_eq!(offspring.len(), 1);
    assert_eq!(count.load(Ordering::Relaxed), 1);
}
