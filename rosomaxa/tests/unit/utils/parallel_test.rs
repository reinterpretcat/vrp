use super::*;
use std::collections::HashMap;

#[test]
fn can_use_map_reduce_for_vec() {
    let vec = vec![1, 2, 3];

    let result = map_reduce(&vec, |item| *item, || 0, |a, b| a + b);

    assert_eq!(result, 6);
}

#[test]
fn can_use_map_reduce_for_map() {
    let mut map = HashMap::new();
    map.insert(1, "1");
    map.insert(2, "2");

    let result = map_reduce(&map, |(key, _)| *key, || 0, |a, b| a + b);

    assert_eq!(result, 3);
}

#[test]
fn can_use_map_reduce_for_slice() {
    let vec = vec![1, 2, 3];

    let result = map_reduce(vec.as_slice(), |item| *item, || 0, |a, b| a + b);

    assert_eq!(result, 6);
}

#[test]
fn can_create_cartesian_product() {
    let left = [1, 2];
    let right = ['a', 'b', 'c'];

    let result =
        parallel_collect(cartesian_product(&left, &right), ParallelismPolicy::Default, |(left, right)| (*left, *right));

    assert_eq!(result, vec![(1, 'a'), (1, 'b'), (1, 'c'), (2, 'a'), (2, 'b'), (2, 'c')]);
}

#[test]
fn can_create_empty_cartesian_product() {
    let left = [1, 2];
    let right: [char; 0] = [];

    let result =
        parallel_collect(cartesian_product(&left, &right), ParallelismPolicy::Default, |(left, right)| (*left, *right));

    assert!(result.is_empty());
}

#[test]
fn can_propagate_coarse_parallelism() {
    let nested = parallel_collect(vec![0, 1], ParallelismPolicy::Coarse, |_| {
        let source = vec![0, 1, 2];
        let nested_parallelism = parallel_collect(source, ParallelismPolicy::Coarse, |_| get_current_parallelism());

        (get_current_parallelism(), nested_parallelism)
    });

    nested.into_iter().for_each(|(outer, nested)| {
        assert_eq!(outer, 2);
        assert_eq!(nested, vec![6; 3]);
    });
    assert_eq!(get_current_parallelism(), 1);
}
