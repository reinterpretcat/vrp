use super::*;

#[test]
fn can_parse_network_state() {
    let data = "(0,1,-1,2,3,[(0,-1,1.23,5,2,[0.0,0.0,-4],),(-1,2,2.33,2,3,[0.0,0.0,5.0],),])";

    let rows_shape = (0, 1);
    let cols_shape = (-1, 2);
    let num_weights = 3;
    let nodes = vec![
        (0, -1, 1.23, 5, 2, vec![0., 0., -4.], "".to_string()),
        (-1, 2, 2.33, 2, 3, vec![0., 0., 5.0], "".to_string()),
    ];

    let network_state = try_parse_network_state(&data.to_string()).unwrap();

    assert_eq!(network_state.shape.0.start, rows_shape.0);
    assert_eq!(network_state.shape.0.end, rows_shape.1);
    assert_eq!(network_state.shape.1.start, cols_shape.0);
    assert_eq!(network_state.shape.1.end, cols_shape.1);
    assert_eq!(network_state.shape.2, num_weights);
    assert_eq!(
        network_state
            .nodes
            .iter()
            .map(|node| {
                (
                    node.coordinate.0,
                    node.coordinate.1,
                    node.unified_distance,
                    node.total_hits,
                    node.last_hits,
                    node.weights.clone(),
                    node.dump.clone(),
                )
            })
            .collect::<Vec<_>>(),
        nodes
    );
}
