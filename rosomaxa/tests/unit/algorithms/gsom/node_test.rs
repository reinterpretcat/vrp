use crate::algorithms::gsom::{Coordinate, Node};
use crate::helpers::algorithms::gsom::{Data, DataStorage};

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
