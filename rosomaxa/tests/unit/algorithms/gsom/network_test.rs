use super::*;
use crate::helpers::algorithms::gsom::{Data, DataStorage, DataStorageFactory};
use crate::helpers::utils::create_test_random;
use crate::utils::Float;
use std::collections::HashSet;

type NetworkType = Network<(), Data, DataStorage, DataStorageFactory>;

fn create_config(node_size: usize) -> NetworkConfig {
    // NOTE these numbers are used in rosomaxa population
    NetworkConfig {
        node_size,
        spread_factor: 0.75,
        distribution_factor: 0.75,
        rebalance_memory: 100,
        learning_rate: 0.1,
        has_initial_error: true,
    }
}

fn get_min_max(items: &[Data]) -> MinMaxWeights {
    let dimension = items[0].values.len();
    items.iter().fold(MinMaxWeights::new(dimension), |mut min_max, data| {
        min_max.update(data.weights());
        min_max
    })
}

pub fn euclidian_distance(a: &[Float], b: &[Float]) -> Float {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
}

fn count_data_stored(nodes: &NodeHashMap<Data, DataStorage>) -> usize {
    nodes.values().map(|node| node.storage.size()).sum::<usize>()
}

fn count_non_empty_nodes(nodes: &NodeHashMap<Data, DataStorage>) -> usize {
    nodes.values().filter(|node| node.storage.iter().next().is_some()).count()
}

fn create_3d_data_grid(size: usize, step: Float) -> Vec<Data> {
    let mut data = Vec::new();
    for x in 0..size {
        for y in 0..size {
            for z in 0..size {
                data.push(Data::new(x as Float * step, y as Float * step, z as Float * step));
            }
        }
    }
    data
}

fn create_random_3d_data(size: usize, range: Float) -> Vec<Data> {
    let random = create_test_random();
    (0..size)
        .map(|_| {
            Data::new(
                random.uniform_real(-range, range),
                random.uniform_real(-range, range),
                random.uniform_real(-range, range),
            )
        })
        .collect()
}

fn create_spiral_data(points: usize, revolutions: Float) -> Vec<Data> {
    (0..points)
        .map(|i| {
            let t = i as Float / points as Float * revolutions * 2.0 * std::f64::consts::PI;
            let r = t / (2.0 * std::f64::consts::PI);
            Data::new(r * t.cos(), r * t.sin(), t / (2.0 * std::f64::consts::PI))
        })
        .collect()
}

#[test]
fn can_iter_min_max_weights_when_is_reset_true() {
    let dimension = 3;
    let min_max_weights = MinMaxWeights::new(dimension);

    let result: Vec<(Float, Float)> = min_max_weights.iter().collect();

    assert_eq!(result, vec![(0.0, 1.0); dimension]);
}

#[test]
fn can_update_min_max_weights() {
    let dimension = 3;
    let mut min_max_weights = MinMaxWeights::new(dimension);

    // Add initial weights
    let weights1 = vec![1.0, 2.0, 3.0];
    min_max_weights.update(&weights1);
    assert_eq!(min_max_weights.min, vec![1.0, 2.0, 3.0]);
    assert_eq!(min_max_weights.max, vec![1.0, 2.0, 3.0]);

    // Add new weights that should update min and max
    let weights2 = vec![0.5, 2.5, 4.0];
    min_max_weights.update(&weights2);
    assert_eq!(min_max_weights.min, vec![0.5, 2.0, 3.0]);
    assert_eq!(min_max_weights.max, vec![1.0, 2.5, 4.0]);

    // Add new weights that should not change min and max
    let weights3 = vec![0.7, 2.3, 3.5];
    min_max_weights.update(&weights3);
    assert_eq!(min_max_weights.min, vec![0.5, 2.0, 3.0]);
    assert_eq!(min_max_weights.max, vec![1.0, 2.5, 4.0]);
}

#[test]
fn can_reset_min_max_weights() {
    let dimension = 3;
    let mut min_max_weights = MinMaxWeights::new(dimension);

    // Add initial weights
    let weights1 = vec![1.0, 2.0, 3.0];
    min_max_weights.update(&weights1);
    assert_eq!(min_max_weights.min, vec![1.0, 2.0, 3.0]);
    assert_eq!(min_max_weights.max, vec![1.0, 2.0, 3.0]);

    // Reset the min_max_weights
    min_max_weights.reset();
    assert!(min_max_weights.is_reset);
    assert_eq!(min_max_weights.iter().collect::<Vec<_>>(), vec![(0.0, 1.0); dimension]);
}

#[test]
fn can_create_network() {
    // Setup test data
    let initial_data = vec![
        Data::new(1., 2., 0.),
        Data::new(2., 3., 0.),
        Data::new(3., 4., 0.),
        Data::new(4., 5., 0.),
        Data::new(5., 6., 0.),
    ];
    let config = create_config(10);
    let random = create_test_random();

    let network =
        NetworkType::new(&(), initial_data.clone(), config.clone(), random.clone(), |_| DataStorageFactory).unwrap();

    // Verify network properties
    assert_eq!(network.dimension, 3);
    assert!((network.growing_threshold - -3. * 0.75_f64.log2()).abs() < 1e-6);
    assert_eq!(network.distribution_factor, config.distribution_factor);
    assert_eq!(network.learning_rate, config.learning_rate);
    assert_eq!(network.rebalance_memory, config.rebalance_memory);

    // Verify initial nodes setup
    assert!(network.size() >= 4); // Should have at least 4 initial nodes
    assert!(network.size() <= 16); // Should not exceed 16 initial nodes
    assert_eq!(count_data_stored(&network.nodes), initial_data.len());

    // Check node properties
    for node in network.get_nodes() {
        assert_eq!(node.weights.len(), 3);
        assert!(node.error >= 0.);
    }
}

#[test]
fn can_create_initial_nodes() {
    let context = ();
    let data = vec![
        Data::new(1., 1., 0.), //
        Data::new(2., 2., 0.),
        Data::new(3., 3., 0.), //
        Data::new(4., 4., 0.),
    ];
    let rebalance_memory = 5;
    let storage_factory = DataStorageFactory;
    let random = create_test_random();
    let noise = Noise::new_with_ratio(1.0, (1., 1.), random);

    let (nodes, min_max_weights) =
        NetworkType::create_initial_nodes(&context, data.clone(), rebalance_memory, &storage_factory, noise).unwrap();

    // Verify nodes
    assert!(nodes.len() >= 4);
    assert!(nodes.len() <= 16);

    // Check min-max weights
    assert_eq!(min_max_weights.min.len(), 3);
    assert_eq!(min_max_weights.max.len(), 3);
    assert!(min_max_weights.min[0] <= 1.0); // Min values
    assert!(min_max_weights.min[1] <= 1.0);
    assert!(min_max_weights.max[0] >= 4.0); // Max values
    assert!(min_max_weights.max[1] >= 4.0);

    // Verify node properties
    for node in nodes.values() {
        assert_eq!(node.weights.len(), 3, "weight dimension");
        assert!(node.storage.size() <= rebalance_memory, "storage size");

        // Check coordinate bounds based on grid size
        let grid_size = (nodes.len() as f64).sqrt().ceil() as i32;
        assert!(node.coordinate.0 >= 0 && node.coordinate.0 < grid_size);
        assert!(node.coordinate.1 >= 0 && node.coordinate.1 < grid_size);
    }

    // Verify all data points are assigned to nodes
    assert_eq!(count_data_stored(&nodes), data.len());
}

#[test]
fn can_select_initial_samples() {
    let sample_size = 4;
    let data = vec![
        Data::new(0.0, 0.0, 0.),
        Data::new(1.0, 0.0, 0.),
        Data::new(0.0, 1.0, 0.),
        Data::new(1.0, 1.0, 0.),
        Data::new(0.5, 0.5, 0.),
        Data::new(0.2, 0.8, 0.),
        Data::new(0.8, 0.2, 0.),
        Data::new(0.3, 0.7, 0.),
        Data::new(0.7, 0.3, 0.),
        Data::new(0.4, 0.6, 0.),
    ];
    let min_max = get_min_max(&data);
    let random = create_test_random();

    let selected = NetworkType::select_initial_samples(&data, sample_size, &min_max, random.as_ref()).unwrap();

    // Verify sample size constraints
    assert_eq!(selected.len(), ((data.len() as f64 * 0.1).ceil() as usize).clamp(4, 16));
    // Verify uniqueness
    let unique_indices: HashSet<_> = selected.iter().collect();
    assert_eq!(unique_indices.len(), selected.len());
    // Verify all indices are valid
    assert!(selected.iter().all(|&idx| idx < data.len()));

    // Verify distance maximization
    let min_distances = (0..selected.len())
        .flat_map(|i| {
            (i + 1..selected.len()).map({
                let data = &data;
                let selected = &selected;
                move |j| {
                    let data_i = &data[selected[i]];
                    let data_j = &data[selected[j]];
                    euclidian_distance(data_i.weights(), data_j.weights())
                }
            })
        })
        .collect::<Vec<_>>();
    // Check that selected points maintain some minimum distance from each other
    assert!(min_distances.iter().all(|&dist| dist >= 0.5));
}

#[test]
fn can_create_network_large_grid() {
    let random = create_test_random();
    for _ in 1..10 {
        let initial_data = create_3d_data_grid(3, 0.5); // 27 points
        let config = create_config(2);
        let network = NetworkType::new(&(), initial_data.clone(), config, random.clone(), |_| DataStorageFactory)
            .expect("Network creation failed");

        let non_empty_nodes = count_non_empty_nodes(&network.nodes);
        assert_eq!(network.dimension, 3);
        assert!(network.size() >= 4, "too small {}", network.size());
        assert!(network.size() <= 100, "too big {}", network.size());
        assert!(non_empty_nodes > 4, "empty nodes: {} from {}", network.size() - non_empty_nodes, network.size());
    }
}

#[test]
fn can_create_network_random_data() {
    let initial_data = create_random_3d_data(50, 10.);
    let config = create_config(50);
    let random = create_test_random();

    let network = NetworkType::new(&(), initial_data.clone(), config, random, |_| DataStorageFactory)
        .expect("Network creation failed");

    // Verify node coverage
    assert_eq!(count_data_stored(&network.nodes), initial_data.len());
    // Check network metrics
    assert!(network.mse() > 0.);
    assert!(network.max_unified_distance() > 0.);
}

#[test]
fn can_create_network_with_spiral_distribution() {
    let size = 100;
    let initial_data = create_spiral_data(size, 3.0);
    let config = create_config(size);
    let random = create_test_random();
    let network = NetworkType::new(&(), initial_data.clone(), config, random, |_| DataStorageFactory).unwrap();

    let non_empty_nodes = count_non_empty_nodes(&network.nodes);
    assert!(non_empty_nodes >= (size / 10), "too sparse {}", non_empty_nodes);
    assert!(network.size() <= (size * 2), "too big {}", network.size());
    let distances: Vec<_> = network
        .get_nodes()
        .flat_map(|node| node.storage.data.iter().map(|data| euclidian_distance(&node.weights, data.weights())))
        .collect();
    let avg_distance = distances.iter().sum::<Float>() / distances.len() as Float;
    assert!(avg_distance < 0.66, "too big average: {}", avg_distance);
}

#[test]
fn can_create_initial_nodes_uniform_distribution() {
    let size = 4;
    let data = create_3d_data_grid(size, 1.0); // 8 points
    let noise = Noise::new_with_ratio(0., (1., 1.), create_test_random());

    let (nodes, min_max) = NetworkType::create_initial_nodes(&(), data.clone(), 5, &DataStorageFactory, noise)
        .expect("Failed to create initial nodes");

    // Verify min-max bounds
    assert_eq!(min_max.min.len(), 3);
    assert_eq!(min_max.max.len(), 3);
    assert!(min_max.min.iter().all(|&x| x >= 0.));
    assert!(min_max.max.iter().all(|&x| x <= (size - 1) as f64));

    // Check node distribution
    let mut coord_set = HashSet::new();
    for node in nodes.values() {
        coord_set.insert(node.coordinate);
        assert_eq!(node.weights.len(), 3);
        assert!(!node.storage.data.is_empty());
    }

    // Verify grid arrangement
    let grid_size = (nodes.len() as f64).sqrt().ceil() as i32;
    assert!(coord_set.iter().all(|c| c.0 < grid_size && c.1 < grid_size));
}

#[test]
fn can_create_initial_nodes_with_outliers() {
    let mut data = vec![
        Data::new(0.0, 0.0, 0.0),
        Data::new(0.1, 0.1, 0.1),
        Data::new(0.2, 0.2, 0.2),
        Data::new(10., 10., 10.),
        Data::new(-10., -10., -10.),
    ];
    data.extend((0..20).map(|i| {
        let x = i as Float * 0.1;
        Data::new(x, x * x, x * x * x)
    }));

    let noise = Noise::new_with_ratio(0., (1., 1.), create_test_random());

    let (nodes, min_max) =
        NetworkType::create_initial_nodes(&(), data.clone(), 10, &DataStorageFactory, noise).unwrap();

    assert!(min_max.iter().all(|(min, max)| min < max));
    assert!(nodes.values().all(|node| !node.storage.data.is_empty()));

    let find_fn = |threshold| {
        nodes.values().flat_map(|node| node.storage.data.iter()).any(|data| data.values.iter().all(|&w| w == threshold))
    };
    assert!(find_fn(10.), "cannot handle max outlier");
    assert!(find_fn(-10.), "cannot handle min outlier");
}

parameterized_test! {can_select_initial_samples_edge_cases, (data, sampling), {
    can_select_initial_samples_edge_cases_impl(data, sampling)
}}

can_select_initial_samples_edge_cases! {
    case01_single_cluster: (vec![
        Data::new(0.0, 0.0, 0.0),
        Data::new(0.1, 0.1, 0.1),
        Data::new(0.2, 0.2, 0.2),
        Data::new(0.15, 0.15, 0.15),
        Data::new(0.05, 0.05, 0.05),
        Data::new(0.25, 0.25, 0.25),
        Data::new(0.12, 0.12, 0.12),
        Data::new(0.18, 0.18, 0.18),
        ], (4, 0.05)),
    case02_two_distinct_clusters: (vec![
        Data::new(0.0, 0.0, 0.0),
        Data::new(0.1, 0.1, 0.1),
        Data::new(0.15, 0.15, 0.15),
        Data::new(0.2, 0.2, 0.2),
        Data::new(10.0, 10.0, 10.0),
        Data::new(10.1, 10.1, 10.1),
        Data::new(10.15, 10.15, 10.15),
        Data::new(10.2, 10.2, 10.2),
        ], (2, 17.)),
    case03_points_on_axes_and_origin: (vec![
        Data::new(1.0, 0.0, 0.0),
        Data::new(0.0, 1.0, 0.0),
        Data::new(0.0, 0.0, 1.0),
        Data::new(0.0, 0.0, 0.0),
        ], (4, 1.)),
}

fn can_select_initial_samples_edge_cases_impl(data: Vec<Data>, sampling: (usize, f64)) {
    let min_max = get_min_max(&data);
    let (sampling_size, expected_min_distance) = sampling;

    let random = create_test_random();
    let selected = NetworkType::select_initial_samples(&data, sampling_size, &min_max, random.as_ref())
        .expect("Failed to select samples");

    assert_eq!(selected.len(), sampling_size);
    assert_eq!(HashSet::<_>::from_iter(selected.iter().copied()).len(), selected.len());

    // Verify minimum distance between selected samples
    let min_distance = selected
        .iter()
        .enumerate()
        .flat_map(|(i, &idx1)| {
            selected.iter().skip(i + 1).map({
                let data = &data;
                move |&idx2| euclidian_distance(data[idx1].weights(), data[idx2].weights())
            })
        })
        .min_by(|a, b| a.total_cmp(b))
        .unwrap_or(f64::MAX);

    assert!(
        min_distance >= expected_min_distance,
        "Minimum distance {} is less than expected threshold {} for test case",
        min_distance,
        expected_min_distance
    );
}

#[test]
fn can_select_initial_samples_with_duplicates() {
    let sample_size = 4;
    let data =
        vec![Data::new(1.0, 1.0, 1.0), Data::new(1.0, 1.0, 1.0), Data::new(2.0, 2.0, 2.0), Data::new(2.0, 2.0, 2.0)];
    let min_max = get_min_max(&data);

    let random = create_test_random();
    let selected = NetworkType::select_initial_samples(&data, sample_size, &min_max, random.as_ref()).unwrap();

    let unique_points: HashSet<_> = selected
        .iter()
        .map(|&idx| {
            let weights = data[idx].weights();
            // NOTE: scale up to collect into hashset as Float doesnt' implement Eq
            (
                (weights[0] * 1000.).round() as i64,
                (weights[1] * 1000.).round() as i64,
                (weights[2] * 1000.).round() as i64,
            )
        })
        .collect();

    assert!(unique_points.len() >= 2);
    assert!(selected.len() >= 4);
}

#[test]
fn can_create_new_network_empty_regions() {
    let mut initial_data = Vec::new();
    // Create clusters with empty regions between them
    for cluster in &[(-5.0, -5.0), (5.0, 5.0), (-5.0, 5.0), (5.0, -5.0)] {
        for dx in -1..=1 {
            for dy in -1..=1 {
                let x = cluster.0 + dx as Float * 0.1;
                let y = cluster.1 + dy as Float * 0.1;
                initial_data.push(Data::new(x, y, (x * x + y * y).sqrt()));
            }
        }
    }
    let config = create_config(12);
    let random = create_test_random();

    let network = NetworkType::new(&(), initial_data, config, random, |_| DataStorageFactory).unwrap();

    let nodes_vec: Vec<_> = network.get_nodes().collect();
    let total_pairs = (nodes_vec.len() * (nodes_vec.len() - 1)) / 2;
    let failed_pairs = (0..nodes_vec.len())
        .flat_map(|i| {
            (i + 1..nodes_vec.len()).map({
                let nodes_vec = &nodes_vec;
                move |j| euclidian_distance(&nodes_vec[i].weights, &nodes_vec[j].weights)
            })
        })
        .filter(|&dist| dist <= 1.)
        .count();
    let failure_fraction = failed_pairs as f64 / total_pairs as f64;
    assert!(failure_fraction < 0.1, "Too many node pairs are too close: {}", failure_fraction);
}
