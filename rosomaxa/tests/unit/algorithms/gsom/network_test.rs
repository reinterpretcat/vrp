use crate::algorithms::gsom::{Coordinate, Network};
use crate::helpers::algorithms::gsom::{create_test_network, Data, DataStorage, DataStorageFactory};
use crate::utils::{compare_floats, DefaultRandom, Random};
use std::cmp::Ordering;

#[test]
fn can_train_network() {
    let mut network = create_test_network(false);
    let samples = vec![Data::new(1.0, 0.0, 0.0), Data::new(0.0, 1.0, 0.0), Data::new(0.0, 0.0, 1.0)];

    // train
    let random = DefaultRandom::default();
    for _ in 1..4 {
        for _ in 1..500 {
            let sample_i = random.uniform_int(0, samples.len() as i32 - 1) as usize;
            network.train(samples[sample_i].clone(), true);
        }

        network.retrain(10, &|node| !node.read().unwrap().storage.data.is_empty());
    }

    assert!(!network.nodes.len() >= 3);
    assert_eq!(network.nodes.len(), network.size());
    samples.iter().for_each(|sample| {
        let node = network.find_bmu(sample);
        let node = node.read().unwrap();

        assert_eq!(node.storage.data.first().unwrap().values, sample.values);
        assert_eq!(node.weights.iter().map(|v| v.round()).collect::<Vec<_>>(), sample.values);
    });
}

parameterized_test! {can_use_initial_error_parameter, (has_initial_error, size), {
    can_use_initial_error_parameter_impl(has_initial_error, size);
}}

can_use_initial_error_parameter! {
    case01: (false, 4),
    case02: (true, 6),
}

fn can_use_initial_error_parameter_impl(has_initial_error: bool, size: usize) {
    let mut network = create_test_network(has_initial_error);

    network.train(Data::new(1.0, 0.0, 0.0), true);

    assert_eq!(network.size(), size);
}

type NetworkType = Network<Data, DataStorage, DataStorageFactory>;

fn get_coord_data(coord: (i32, i32), offset: (i32, i32), network: &NetworkType) -> (Coordinate, Vec<f64>) {
    let node = network.nodes.get(&Coordinate(coord.0 + offset.0, coord.1 + offset.1)).unwrap();
    let node = node.read().unwrap();

    let coordinate = node.coordinate.clone();
    let weights = node.weights.clone();

    (coordinate, weights)
}

fn add_node(x: i32, y: i32, network: &mut NetworkType) {
    network.insert(Coordinate(x, y), &[x as f64, y as f64]);
}

fn update_zero_neighborhood(network: &mut NetworkType) {
    add_node(-1, 1, network);
    add_node(-1, 0, network);
    add_node(-1, -1, network);
    add_node(0, -1, network);
    add_node(1, -1, network);
}

#[test]
fn can_insert_initial_node_neighborhood() {
    let network = create_test_network(false);
    assert_eq!(network.nodes.len(), 4);

    assert_eq!(get_coord_data((0, 0), (1, 0), &network).0, Coordinate(1, 0));
    assert_eq!(get_coord_data((0, 0), (0, 1), &network).0, Coordinate(0, 1));

    assert_eq!(get_coord_data((1, 0), (-1, 0), &network).0, Coordinate(0, 0));
    assert_eq!(get_coord_data((1, 0), (0, 1), &network).0, Coordinate(1, 1));

    assert_eq!(get_coord_data((1, 1), (-1, 0), &network).0, Coordinate(0, 1));
    assert_eq!(get_coord_data((1, 1), (0, -1), &network).0, Coordinate(1, 0));

    assert_eq!(get_coord_data((0, 1), (0, -1), &network).0, Coordinate(0, 0));
    assert_eq!(get_coord_data((0, 1), (1, 0), &network).0, Coordinate(1, 1));
}

#[test]
fn can_create_and_update_extended_neighbourhood() {
    let mut network = create_test_network(false);
    update_zero_neighborhood(&mut network);
    network.nodes.get(&Coordinate(0, 0)).unwrap().read().unwrap().neighbours(&network, 1).for_each(|(node, _)| {
        let node = node.unwrap();
        let mut node = node.write().unwrap();
        node.error = 42.;
    });

    // -1+1  0+1  +1+1
    // -1+0  0 0  +1 0
    // -1-1  0-1  +1-1
    assert_eq!(network.nodes.len(), 9);
    network.nodes.iter().filter(|(coord, _)| **coord != Coordinate(0, 0)).for_each(|(coord, node)| {
        let error = node.read().unwrap().error;
        if compare_floats(error, 42.) != Ordering::Equal {
            unreachable!("node is not updated: ({},{}), value: {}", coord.0, coord.1, error);
        }
    });
    [
        (1, (0, 0), 8),
        (1, (0, -1), 5),
        (1, (0, 1), 5),
        (1, (1, 0), 5),
        (1, (-1, 0), 5),
        (1, (-1, 1), 3),
        (1, (1, 1), 3),
        (1, (-1, -1), 3),
        (1, (1, -1), 3),
    ]
    .into_iter()
    .for_each(|(radius, (x, y), expected_count)| {
        let count = network
            .nodes
            .get(&Coordinate(x, y))
            .unwrap()
            .read()
            .unwrap()
            .neighbours(&network, radius)
            .filter(|(node, _)| node.is_some())
            .count();
        if count != expected_count {
            unreachable!("unexpected neighbourhood for: ({},{}), {} vs {}", x, y, count, expected_count)
        }
    });
}
