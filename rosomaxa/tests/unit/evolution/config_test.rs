use super::*;
use crate::example::{VectorContext, VectorObjective, VectorRosomaxaContext, VectorSolution};
use crate::helpers::example::{create_default_heuristic_context, create_example_objective};
use crate::population::Greedy;
use crate::utils::{Environment, Quota};
use crate::{TelemetryMode, get_default_population, get_default_selection_size};
use std::sync::Arc;

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
    let heuristic = DynamicSelective::new(vec![], vec![], context.environment());

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
