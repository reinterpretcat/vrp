use crate::{DataGraph, GraphEdge, GraphNode, ObservationData};
use vrp_scientific::common::CoordIndexExtraProperty;
use vrp_scientific::core::construction::heuristics::InsertionContext;

impl From<&InsertionContext> for DataGraph {
    fn from(insertion_ctx: &InsertionContext) -> Self {
        let coord_index = insertion_ctx.problem.extras.get_coord_index().expect("cannot get coord index!");

        let nodes = coord_index.locations.iter().map(|(x, y)| GraphNode { x: *x as f64, y: *y as f64 }).collect();
        let edges = insertion_ctx
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| {
                route_ctx.route().tour.legs().map(|(activities, _)| match activities {
                    [from, to] => GraphEdge { source: from.place.location, target: to.place.location },
                    _ => unreachable!("leg configuration"),
                })
            })
            .collect();

        DataGraph { nodes, edges }
    }
}

impl From<&ObservationData> for DataGraph {
    fn from(data: &ObservationData) -> Self {
        match data {
            ObservationData::Vrp { graph, .. } => graph.clone(),
            _ => unreachable!(),
        }
    }
}
