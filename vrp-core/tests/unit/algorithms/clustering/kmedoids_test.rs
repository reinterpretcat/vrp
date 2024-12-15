use super::*;
use rosomaxa::prelude::DefaultRandom;

parameterized_test! {can_create_clusters, (k, expected), {
    can_create_clusters_impl(k, expected);
}}

can_create_clusters! {
    case01: (2, vec![(7, vec![6, 7, 8]), (3, vec![0, 1, 2, 3, 4, 5])]),
    case02: (3, vec![(3, vec![0, 1, 2, 3, 4, 5]), (6, vec![6]), (7, vec![7, 8])]),
    case03: (4, vec![(0, vec![0, 1, 2]), (4, vec![3, 4, 5]), (6, vec![6]), (7, vec![7, 8])]),
}

pub fn can_create_clusters_impl(k: usize, expected: Vec<(usize, Vec<usize>)>) {
    #[rustfmt::skip]
    let distances = vec![
      vec![0.0, 0.8341, 0.3686, 8.0639, 8.8835, 8.6478, 12.2809, 12.8486, 13.313],
      vec![0.8341, 0.0, 0.983, 7.7073, 8.5811, 8.3868, 11.4483, 12.0148, 12.4789],
      vec![0.3686, 0.983, 0.0, 7.7881, 8.5899, 8.3431, 12.3873, 12.9251, 13.3981],
      vec![8.0639, 7.7073, 7.7881, 0.0, 1.0305, 1.193, 11.5335, 11.3159, 11.8799],
      vec![8.8835, 8.5811, 8.5899, 1.0305, 0.0, 0.5042, 12.3838, 12.1047, 12.6662],
      vec![8.6478, 8.3868, 8.3431, 1.193, 0.5042, 0.0, 12.7067, 12.4625, 13.0255],
      vec![12.2809, 11.4483, 12.3873, 11.5335, 12.3838, 12.7067, 0.0, 1.1931, 1.2783],
      vec![12.8486, 12.0148, 12.9251, 11.3159, 12.1047, 12.4625, 1.1931, 0.0, 0.5647],
      vec![13.313, 12.4789, 13.3981, 11.8799, 12.6662, 13.0255, 1.2783, 0.5647, 0.0],
    ];
    let data = (0..distances.len()).collect::<Vec<usize>>();
    let random = DefaultRandom::new_repeatable();

    let clusters = create_k_medoids(&data, k, &random, move |p1: &usize, p2: &usize| distances[*p1][*p2]);

    for (medoid, points) in expected {
        assert_eq!(clusters[&medoid], points);
    }
}
