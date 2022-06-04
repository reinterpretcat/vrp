use crate::DataGraph;
use vrp_scientific::core::construction::heuristics::InsertionContext;

impl From<&InsertionContext> for DataGraph {
    fn from(_: &InsertionContext) -> Self {
        todo!("missing conversion")
    }
}
