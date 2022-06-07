use super::*;

#[test]
fn can_create_bundled_edges() {
    let graph = DataGraph {
        nodes: vec![
            GraphNode { x: 0., y: 0. },
            GraphNode { x: 1., y: 0. },
            GraphNode { x: 0., y: 1. },
            GraphNode { x: 1., y: 1. },
        ],
        edges: vec![
            GraphEdge { source: 0, target: 1 },
            GraphEdge { source: 1, target: 2 },
            GraphEdge { source: 2, target: 3 },
        ],
    };
    let result = Fdeb::new(graph).calculate();

    assert!(!result.is_empty())
}
