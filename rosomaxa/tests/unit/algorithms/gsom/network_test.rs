use crate::algorithms::gsom::{Coordinate, Network};
use crate::helpers::algorithms::gsom::{create_test_network, Data, DataStorage, DataStorageFactory};
use crate::utils::{DefaultRandom, Random};

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

fn get_coord_data(coord: (i32, i32), relative: (i32, i32), network: &NetworkType) -> (Coordinate, Vec<f64>) {
    let node = network.nodes.get(&Coordinate(coord.0, coord.1)).unwrap().read();
    let topology = node.unwrap().topology.clone();

    let node = match relative {
        (0, 1) => topology.up,
        (0, -1) => topology.down,
        (1, 0) => topology.right,
        (-1, 0) => topology.left,
        _ => unreachable!(),
    }
    .unwrap();

    let node = node.read().unwrap();
    let coordinate = node.coordinate.clone();
    let weights = node.weights.clone();

    (coordinate, weights)
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
fn can_insert_nodes_updating_neighbourhood() {
    type NetworkType = Network<Data, DataStorage, DataStorageFactory>;
    let mut network = create_test_network(false);
    let add_node = |x: i32, y: i32, network: &mut NetworkType| {
        network.insert(Coordinate(x, y), &[x as f64, y as f64]);
    };
    // 10-11
    // 00-10

    add_node(-1, 0, &mut network);
    assert_eq!(get_coord_data((0, 0), (-1, 0), &network).0, Coordinate(-1, 0));
}
