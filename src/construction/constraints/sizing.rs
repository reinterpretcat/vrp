use crate::construction::constraints::{ConstraintModule, ConstraintVariant};
use crate::construction::states::{RouteContext, SolutionContext};
use std::slice::Iter;

pub struct SizingConstraintModule {}

impl SizingConstraintModule {
    pub fn new(code: i32) -> Self {
        Self {}
    }
}

impl ConstraintModule for SizingConstraintModule {
    fn accept_route_state(&self, ctx: &mut RouteContext) {
        unimplemented!()
    }

    fn accept_solution_state(&self, ctx: &mut SolutionContext) {
        unimplemented!()
    }

    fn state_keys(&self) -> Iter<i32> {
        unimplemented!()
    }

    fn get_constraints(&self) -> Iter<ConstraintVariant> {
        unimplemented!()
    }
}
