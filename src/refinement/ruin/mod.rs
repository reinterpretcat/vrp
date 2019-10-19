use crate::construction::states::InsertionContext;

/// Specifies ruin strategy.
pub trait Ruin {
    fn ruin_solution(&self, mut insertion_ctx: InsertionContext) -> InsertionContext;
}

mod adjusted_string_removal;
pub use self::adjusted_string_removal::AdjustedStringRemoval;

mod random_route_removal;
pub use self::random_route_removal::RandomRouteRemoval;

pub struct RuinComposite {
    ruins: Vec<Box<dyn Ruin>>,
}

impl Ruin for RuinComposite {
    fn ruin_solution(&self, mut insertion_ctx: InsertionContext) -> InsertionContext {
        //let individuum = refinement_ctx.individuum()?;
        //let mut insertion_cxt = create_insertion_context(&refinement_ctx.problem, individuum, &refinement_ctx.random);
        // let solution = individuum.0.as_ref();

        unimplemented!()
    }
}
