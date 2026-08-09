use super::*;
use crate::example::{VectorContext, VectorObjective, VectorRosomaxaContext, VectorSolution};
use crate::helpers::example::{create_default_heuristic_context, create_example_objective};
use crate::population::Greedy;
use crate::termination::MaxGeneration;
use crate::utils::{Environment, Quota};
use crate::{TelemetryMode, get_default_population, get_default_selection_size};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CustomTermination;

struct UnreachedQuota;

impl Quota for UnreachedQuota {
    fn is_reached(&self) -> bool {
        false
    }
}

impl Termination for CustomTermination {
    type Context = VectorContext;
    type Objective = VectorObjective;

    fn is_termination(&self, _: &mut Self::Context) -> bool {
        true
    }

    fn estimate(&self, _: &Self::Context) -> Float {
        0.42
    }
}

fn create_builder(
    context: VectorContext,
) -> EvolutionConfigBuilder<VectorContext, VectorObjective, VectorSolution, i32> {
    let heuristic = DynamicSelective::new(vec![], context.environment());

    EvolutionConfigBuilder::default().with_heuristic(Box::new(heuristic)).with_context(context)
}

fn create_context_with_quota(quota: Option<Arc<dyn Quota>>) -> VectorContext {
    let environment = Arc::new(Environment { quota, ..Environment::default() });
    let objective = create_example_objective();
    let selection_size = get_default_selection_size(environment.as_ref());
    let population =
        get_default_population(objective.clone(), VectorRosomaxaContext, environment.clone(), selection_size);

    VectorContext::new(objective, population, TelemetryMode::None, environment)
}

#[test]
fn can_use_custom_termination() {
    let context = create_default_heuristic_context();
    let mut config = create_builder(context).with_termination(Box::new(CustomTermination)).build().unwrap();

    assert_eq!(config.termination.estimate(&config.context), 0.42);
    assert!(config.termination.is_termination(&mut config.context));
}

#[test]
fn can_configure_intensify_operators() {
    struct Noop;
    struct CountingIntensify {
        count: Arc<AtomicUsize>,
    }

    impl HeuristicSearchOperator for Noop {
        type Context = VectorContext;
        type Objective = VectorObjective;
        type Solution = VectorSolution;

        fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
            solution.deep_copy()
        }
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

    let environment = Arc::new(Environment::default());
    let objective = create_example_objective();
    let solution = VectorSolution::new(vec![0., 0.], 0., vec![0., 0.]);
    let population = Box::new(Greedy::new(objective.clone(), 1, Some(solution)));
    let context = VectorContext::new(objective, population, TelemetryMode::None, environment);
    let count = Arc::new(AtomicUsize::new(0));
    let config = EvolutionConfigBuilder::default()
        .with_context(context)
        .with_search_operators(vec![(Arc::new(Noop), "noop".to_string(), 1.)])
        .with_intensify_operators(vec![Arc::new(CountingIntensify { count: count.clone() })])
        .with_termination(Box::new(MaxGeneration::new(1)))
        .build()
        .unwrap();
    let EvolutionConfig { mut strategy, context, termination, .. } = config;

    strategy.run(context, termination).unwrap();

    assert!(count.load(Ordering::Relaxed) > 0);
}

#[test]
fn rejects_phase_gated_variation_without_escape() {
    let result = create_builder(create_default_heuristic_context())
        .with_min_cv(Some(("sample".to_string(), 100, 0.01, false)), 0)
        .build();

    let error = match result {
        Ok(_) => panic!("expected invalid termination configuration"),
        Err(error) => error,
    };

    assert_eq!(
        error.to_string(),
        "non-global variation termination requires max-generations, max-time, an external quota, or an exploitation-phase population"
    );
}

#[test]
fn allows_phase_gated_variation_with_generation_limit() {
    let result = create_builder(create_default_heuristic_context())
        .with_max_generations(Some(1000))
        .with_min_cv(Some(("sample".to_string(), 100, 0.01, false)), 0)
        .build();

    assert!(result.is_ok());
}

#[test]
fn allows_global_variation_without_escape() {
    let result = create_builder(create_default_heuristic_context())
        .with_min_cv(Some(("sample".to_string(), 100, 0.01, true)), 0)
        .build();

    assert!(result.is_ok());
}

#[test]
fn allows_phase_gated_variation_with_external_quota() {
    let quota: Arc<dyn Quota> = Arc::new(UnreachedQuota);
    let result = create_builder(create_context_with_quota(Some(quota)))
        .with_min_cv(Some(("sample".to_string(), 100, 0.01, false)), 0)
        .build();

    assert!(result.is_ok());
}

#[test]
fn allows_phase_gated_variation_for_exploitation_population() {
    let environment = Arc::new(Environment::default());
    let objective = create_example_objective();
    let population = Box::new(Greedy::new(objective.clone(), 1, None));
    let context = VectorContext::new(objective, population, TelemetryMode::None, environment);
    let result = create_builder(context).with_min_cv(Some(("sample".to_string(), 100, 0.01, false)), 0).build();

    assert!(result.is_ok());
}
