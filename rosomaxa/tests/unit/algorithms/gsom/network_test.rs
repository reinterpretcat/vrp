use crate::algorithms::gsom::{Coordinate, Network};
use crate::helpers::algorithms::gsom::{Data, DataStorage, DataStorageFactory};
use crate::utils::Random;

type NetworkType = Network<Data, DataStorage, DataStorageFactory>;

mod common {
    use super::*;
    use crate::helpers::algorithms::gsom::create_test_network;
    use crate::utils::{compare_floats, DefaultRandom};
    use std::cmp::Ordering;

    #[test]
    fn can_train_network() {
        let mut network = create_test_network(false);
        let samples = vec![Data::new(1.0, 0.0, 0.0), Data::new(0.0, 1.0, 0.0), Data::new(0.0, 0.0, 1.0)];

        // train
        let random = DefaultRandom::default();
        for j in 1..4 {
            network.smooth(4);

            for i in 1..500 {
                let idx = random.uniform_int(0, samples.len() as i32 - 1) as usize;
                network.store(samples[idx].clone(), j * i + i);
            }
        }

        assert!(!network.nodes.len() >= 4);
        samples.iter().for_each(|sample| {
            let node = network.find_bmu(sample);

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

    fn get_coord_data(coord: (i32, i32), offset: (i32, i32), network: &NetworkType) -> (Coordinate, Vec<f64>) {
        let node = network.nodes.get(&Coordinate(coord.0 + offset.0, coord.1 + offset.1)).unwrap();
        let coordinate = node.coordinate;
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
        network
            .nodes
            .get(&Coordinate(0, 0))
            .unwrap()
            .neighbours(&network, 1)
            .filter_map(|(coord, _)| coord)
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|coord| {
                let node = network.nodes.get_mut(&coord).unwrap();
                node.error = 42.;
            });

        // -1+1  0+1  +1+1
        // -1+0  0 0  +1 0
        // -1-1  0-1  +1-1
        assert_eq!(network.nodes.len(), 9);
        network.nodes.iter().filter(|(coord, _)| **coord != Coordinate(0, 0)).for_each(|(coord, node)| {
            if compare_floats(node.error, 42.) != Ordering::Equal {
                unreachable!("node is not updated: ({},{}), value: {}", coord.0, coord.1, node.error);
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
                .neighbours(&network, radius)
                .filter(|(node, _)| node.is_some())
                .count();
            if count != expected_count {
                unreachable!("unexpected neighbourhood for: ({},{}), {} vs {}", x, y, count, expected_count)
            }
        });
    }
}

mod node_growing {
    use super::*;
    use crate::algorithms::gsom::{NetworkConfig, Node};
    use crate::prelude::RandomGen;
    use std::sync::Arc;

    fn create_trivial_network(has_initial_error: bool) -> NetworkType {
        struct DummyRandom {}
        impl Random for DummyRandom {
            fn uniform_int(&self, _: i32, _: i32) -> i32 {
                unreachable!()
            }

            fn uniform_real(&self, _: f64, _: f64) -> f64 {
                unreachable!()
            }

            fn is_head_not_tails(&self) -> bool {
                unreachable!()
            }

            fn is_hit(&self, _: f64) -> bool {
                false
            }

            fn weighted(&self, _: &[usize]) -> usize {
                unreachable!()
            }

            fn get_rng(&self) -> RandomGen {
                RandomGen::new_repeatable()
            }
        }
        Network::new(
            [
                Data::new(1., 4., 8.), // n00
                Data::new(2., 5., 9.), // n01
                Data::new(3., 8., 7.), // n11
                Data::new(9., 3., 2.), // n10
            ],
            NetworkConfig {
                spread_factor: 0.25,
                distribution_factor: 0.25,
                learning_rate: 0.1,
                rebalance_memory: 500,
                has_initial_error,
            },
            Arc::new(DummyRandom {}),
            DataStorageFactory,
        )
    }

    fn get_node(coord: (i32, i32), network: &NetworkType) -> Option<&Node<Data, DataStorage>> {
        network.nodes.get(&Coordinate(coord.0, coord.1))
    }

    fn round_weights(weights: &[f64]) -> Vec<f64> {
        weights.iter().map(|w| (w * 1000.).round() / 1000.).collect()
    }

    parameterized_test! {can_grow_initial_nodes_properly, (target_coord, expected_new_nodes), {
        can_grow_initial_nodes_properly_impl(target_coord, expected_new_nodes);
    }}

    can_grow_initial_nodes_properly! {
        case01: ((0, 0), vec![((-1, 0), vec![-6.623, 4.874, 13.497]), ((0, -1), vec![0.073, 2.963, 6.817])]),
        case02: ((0, 1), vec![((-1, 0), vec![1.042, 2.0, 10.623]), ((0, 1), vec![2.963, 5.853, 9.707])]),
        case03: ((1, 0), vec![((1, 0), vec![16.45, 2.0, -3.78]), ((0, -1), vec![14.455, -1.832, -2.791])]),
        case04: ((1, 1), vec![((1, 0), vec![3.927, 10.67, 4.89]), ((0, 1), vec![-2.791, 12.539, 11.581])]),
    }

    fn can_grow_initial_nodes_properly_impl(target_coord: (i32, i32), expected_new_nodes: Vec<((i32, i32), Vec<f64>)>) {
        let mut network = create_trivial_network(true);

        network.update(&Coordinate(target_coord.0, target_coord.1), &Data::new(2., 2., 2.), 2., true);

        assert_eq!(network.nodes.len(), 6);
        expected_new_nodes.into_iter().for_each(|((offset_x, offset_y), weights)| {
            let node = get_node((target_coord.0 + offset_x, target_coord.1 + offset_y), &network).unwrap();
            assert_eq!(node.error, 0.);
            assert_eq!(round_weights(node.weights.as_slice()), weights);
        });
    }

    #[test]
    fn can_grow_new_nodes_properly() {
        let w1_coord = Coordinate(1, 2);
        let mut network = create_trivial_network(true);
        network.insert(w1_coord, &[3., 6., 10.]);

        network.update(&Coordinate(w1_coord.0, w1_coord.1), &Data::new(2., 2., 2.), 6., true);

        [
            ((2, 2), vec![2.948, 3.895, 12.423]),
            ((0, 2), vec![2.917, 3.833, 12.083]),
            ((1, 3), vec![2.929, 3.858, 12.222]),
        ]
        .into_iter()
        .for_each(|(coord, weights)| {
            let node = get_node(coord, &network).unwrap();
            let actual = round_weights(node.weights.as_slice());
            assert_eq!(actual, weights);
        });
    }

    #[test]
    fn can_calculate_mse() {
        let mut network = create_trivial_network(false);
        let mse = network.mse();
        assert_eq!(mse, 0.);

        network.smooth(1);
        let mse = network.mse();
        assert!((mse - 0.0001138).abs() < 1E7);
    }

    parameterized_test! {can_grow_nodes_with_proper_weights, (coord, expected), {
        can_grow_nodes_with_proper_weights_impl(coord, expected);
    }}

    can_grow_nodes_with_proper_weights! {
        case01_a_case_left_down: ((0, 0), vec![(Coordinate(-1, 0), vec![-7., 5., 14.]), (Coordinate(0, -1), vec![0., 3., 7.])]),
        case02_a_case_right_top: ((1, 1), vec![(Coordinate(1, 2), vec![-3., 13., 12.]), (Coordinate(2, 1), vec![4., 11., 5.])]),
        case03_ac_cases: ((1, -1), vec![(Coordinate(0, -1), vec![1., -1., 4.]), (Coordinate(1, -2), vec![1., -1., 4.]), (Coordinate(2, -1), vec![1., -1., 4.])]),
        case04_ba_cases_left: ((0, 1), vec![(Coordinate(-1, 1), vec![1.5, 4., 11.5]), (Coordinate(0, 2), vec![3., 6., 10.])]),
        case05_bd_cases: ((-2, 1), vec![(Coordinate(-3, 1), vec![5., 4.5, 8.]), (Coordinate(-2, 0), vec![5., 4.5, 8.]), (Coordinate(-2, 2), vec![5., 4.5, 8.]), (Coordinate(-1, 1), vec![1.5, 4., 11.5])]),
    }

    fn can_grow_nodes_with_proper_weights_impl(coord: (i32, i32), expected: Vec<(Coordinate, Vec<f64>)>) {
        // n(-2,1)(1., 3., 14.) xx  n01(2., 5., 9.) n11(3., 8., 7.)
        //                          n00(1., 4., 8.) n10(9., 3., 2.)
        //                                         n1-1(5., 1., 3.)
        let mut network = create_trivial_network(false);
        network.insert(Coordinate(-2, 1), &[1., 3., 14.]);
        network.insert(Coordinate(1, -1), &[5., 1., 3.]);

        let mut nodes = network.grow_nodes(&Coordinate(coord.0, coord.1));
        nodes.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(nodes, expected);
    }
}
