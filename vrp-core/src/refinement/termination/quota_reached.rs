use crate::refinement::termination::Termination;
use crate::refinement::{Individuum, RefinementContext};

/// Stops when quota is reached.
pub struct QuotaReached {}

impl Default for QuotaReached {
    fn default() -> Self {
        Self {}
    }
}

impl Termination for QuotaReached {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext, _: (&Individuum, bool)) -> bool {
        refinement_ctx.get_quota().map_or(false, |quota| quota.is_reached())
    }
}
