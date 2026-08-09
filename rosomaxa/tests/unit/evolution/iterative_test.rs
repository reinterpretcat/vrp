use super::*;
use crate::example::{VectorContext, VectorObjective, VectorSolution};
use crate::helpers::example::create_example_objective;
use crate::population::Greedy;
use crate::termination::MaxGeneration;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingHeuristic {
    search_count: Arc<AtomicUsize>,
    diversify_count: Arc<AtomicUsize>,
    intensify_count: Arc<AtomicUsize>,
}

impl HyperHeuristic for CountingHeuristic {
    type Context = VectorContext;
    type Objective = VectorObjective;
    type Solution = VectorSolution;

    fn search(&mut self, _: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        self.search_count.fetch_add(1, Ordering::Relaxed);
        vec![solution.deep_copy()]
    }

    fn search_many(&mut self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        solutions.into_iter().flat_map(|solution| self.search(heuristic_ctx, solution)).collect()
    }

    fn diversify(&self, _: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        self.diversify_count.fetch_add(1, Ordering::Relaxed);
        vec![solution.deep_copy()]
    }

    fn diversify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        solutions.into_iter().flat_map(|solution| self.diversify(heuristic_ctx, solution)).collect()
    }

    fn intensify(&self, _: &Self::Context, solution: &Self::Solution) -> Vec<Self::Solution> {
        self.intensify_count.fetch_add(1, Ordering::Relaxed);
        vec![solution.deep_copy()]
    }

    fn intensify_many(&self, heuristic_ctx: &Self::Context, solutions: Vec<&Self::Solution>) -> Vec<Self::Solution> {
        solutions.into_iter().flat_map(|solution| self.intensify(heuristic_ctx, solution)).collect()
    }
}

impl Display for CountingHeuristic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("counting")
    }
}

#[test]
fn can_intensify_in_exploitation_without_diversification() {
    let environment = Arc::new(Environment::default());
    let objective = create_example_objective();
    let solution = VectorSolution::new(vec![0., 0.], 0., vec![0., 0.]);
    let population = Box::new(Greedy::new(objective.clone(), 1, Some(solution)));
    let context = VectorContext::new(objective, population, TelemetryMode::None, environment);
    let search_count = Arc::new(AtomicUsize::new(0));
    let diversify_count = Arc::new(AtomicUsize::new(0));
    let intensify_count = Arc::new(AtomicUsize::new(0));
    let heuristic = CountingHeuristic {
        search_count: search_count.clone(),
        diversify_count: diversify_count.clone(),
        intensify_count: intensify_count.clone(),
    };
    let mut strategy = Iterative::new(Box::new(heuristic), 1);

    strategy.run(context, Box::new(MaxGeneration::new(1))).unwrap();

    assert_eq!(search_count.load(Ordering::Relaxed), 2);
    assert_eq!(diversify_count.load(Ordering::Relaxed), 0);
    assert_eq!(intensify_count.load(Ordering::Relaxed), 2);
}
