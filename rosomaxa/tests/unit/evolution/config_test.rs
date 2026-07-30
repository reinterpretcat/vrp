use super::*;
use crate::example::{VectorContext, VectorObjective};
use crate::helpers::example::create_default_heuristic_context;

struct CustomTermination;

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

#[test]
fn can_use_custom_termination() {
    let context = create_default_heuristic_context();
    let heuristic = DynamicSelective::new(vec![], vec![], context.environment());
    let mut config = EvolutionConfigBuilder::default()
        .with_heuristic(Box::new(heuristic))
        .with_context(context)
        .with_termination(Box::new(CustomTermination))
        .build()
        .unwrap();

    assert_eq!(config.termination.estimate(&config.context), 0.42);
    assert!(config.termination.is_termination(&mut config.context));
}
