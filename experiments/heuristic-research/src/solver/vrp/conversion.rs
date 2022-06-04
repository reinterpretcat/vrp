use crate::DataGraph;
use vrp_scientific::common::CoordIndex;
use vrp_scientific::core::construction::heuristics::InsertionContext;

impl From<&InsertionContext> for DataGraph {
    fn from(insertion_ctx: &InsertionContext) -> Self {
        let coord_index = insertion_ctx
            .problem
            .extras
            .get("coord_index")
            .and_then(|s| s.downcast_ref::<CoordIndex>())
            .expect("cannot get coord index!");

        let nodes = coord_index.locations.iter().map(|(x, y)| (*x as f64, *y as f64)).collect();
        let edges = insertion_ctx
            .solution
            .routes
            .iter()
            .flat_map(|route_ctx| {
                route_ctx.route.tour.legs().map(|(activities, _)| match activities {
                    [from, to] => (from.place.location, to.place.location),
                    _ => unreachable!(),
                })
            })
            .collect();

        DataGraph { nodes, edges }
    }
}
