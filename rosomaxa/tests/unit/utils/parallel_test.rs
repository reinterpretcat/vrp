use super::*;
use std::collections::HashMap;

#[test]
fn can_use_map_reduce_for_vec() {
    let vec = vec![1, 2, 3];

    let result = map_reduce(&vec, ParallelismScope::Local, |item| *item, || 0, |a, b| a + b);

    assert_eq!(result, 6);
}

#[test]
fn can_use_map_reduce_for_map() {
    let mut map = HashMap::new();
    map.insert(1, "1");
    map.insert(2, "2");

    let result = map_reduce(&map, ParallelismScope::Local, |(key, _)| *key, || 0, |a, b| a + b);

    assert_eq!(result, 3);
}

#[test]
fn can_use_map_reduce_for_slice() {
    let vec = vec![1, 2, 3];

    let result = map_reduce(vec.as_slice(), ParallelismScope::Local, |item| *item, || 0, |a, b| a + b);

    assert_eq!(result, 6);
}

#[test]
fn can_create_cartesian_product() {
    let left = [1, 2];
    let right = ['a', 'b', 'c'];

    let result = parallel_collect(
        cartesian_product(&left, &right, ParallelismPolicy::Default),
        ParallelismScope::Local,
        ParallelismPolicy::Default,
        |(left, right)| (*left, *right),
    );

    assert_eq!(result, vec![(1, 'a'), (1, 'b'), (1, 'c'), (2, 'a'), (2, 'b'), (2, 'c')]);
}

#[test]
fn can_create_empty_cartesian_product() {
    let left = [1, 2];
    let right: [char; 0] = [];

    let result = parallel_collect(
        cartesian_product(&left, &right, ParallelismPolicy::adaptive(4)),
        ParallelismScope::Local,
        ParallelismPolicy::Default,
        |(left, right)| (*left, *right),
    );

    assert!(result.is_empty());
}

#[test]
#[should_panic(expected = "tasks per worker must be greater than zero")]
fn cannot_create_adaptive_policy_without_tasks() {
    ParallelismPolicy::adaptive(0);
}

#[test]
fn can_propagate_parallelism_scope() {
    let nested = parallel_collect(vec![0, 1], ParallelismScope::Coarse, ParallelismPolicy::Default, |_| {
        let source = vec![0, 1, 2];
        let collected = parallel_collect(&source, ParallelismScope::Coarse, ParallelismPolicy::Default, |_| {
            get_current_parallelism()
        });
        let collected_owned =
            parallel_into_collect(source.clone(), ParallelismScope::Coarse, |_| get_current_parallelism());
        let mapped = map_reduce(&source, ParallelismScope::Coarse, |_| get_current_parallelism(), || 0, usize::max);
        let folded = fold_reduce(&source, ParallelismScope::Coarse, || 0, |_, _| get_current_parallelism(), usize::max);
        let mut visited = source.clone();
        parallel_foreach_mut(&mut visited, ParallelismScope::Coarse, |value| *value = get_current_parallelism());

        (get_current_parallelism(), collected, collected_owned, mapped, folded, visited)
    });

    nested.into_iter().for_each(|(outer, collected, collected_owned, mapped, folded, visited)| {
        assert_eq!(outer, 2);
        assert_eq!(collected, vec![6; 3]);
        assert_eq!(collected_owned, vec![6; 3]);
        assert_eq!(mapped, 6);
        assert_eq!(folded, 6);
        assert_eq!(visited, vec![6; 3]);
    });
    assert_eq!(get_current_parallelism(), 1);
}
