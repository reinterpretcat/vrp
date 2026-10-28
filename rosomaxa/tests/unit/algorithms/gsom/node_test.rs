use crate::algorithms::gsom::{Coordinate, Node};
use crate::helpers::algorithms::gsom::{Data, DataStorage, create_test_network};
use crate::utils::Float;

fn create_test_node(hit_memory_size: usize) -> Node<Data, DataStorage> {
    Node::new(Coordinate(0, 0), &[1., 2.], 0., hit_memory_size, DataStorage::default())
}

#[test]
fn can_track_last_hits() {
    let hit_memory_size = 100;
    let mut node = create_test_node(hit_memory_size);

    node.new_hit(1);
    assert_eq!(node.get_last_hits(1), 1);
    assert_eq!(node.get_last_hits(2), 1);

    node.new_hit(3);
    assert_eq!(node.get_last_hits(3), 2);

    node.new_hit(hit_memory_size);
    assert_eq!(node.get_last_hits(hit_memory_size), 3);

    node.new_hit(hit_memory_size + 1);
    assert_eq!(node.get_last_hits(hit_memory_size + 1), 3);

    node.new_hit(hit_memory_size + 100);
    assert_eq!(node.get_last_hits(hit_memory_size + 100), 2);
}

#[test]
fn can_calculate_unified_distance() {
    let network = create_test_network(false);
    let node = network.iter_nodes().next().unwrap();
    let (sum, count) = node
        .neighbours(&network, 1)
        .filter_map(|(coordinate, _)| coordinate.and_then(|coordinate| network.find(&coordinate)))
        .fold((0., 0), |(sum, count), neighbor| {
            (sum + network.distance(node.weights.as_slice(), neighbor.weights.as_slice()), count + 1)
        });
    let expected = if count > 0 { sum / count as Float } else { 0. };

    assert_eq!(node.unified_distance(&network, 1), expected);
}
