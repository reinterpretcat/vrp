use crate::extensions::multi_dim_capacity::MultiDimensionalCapacity;

fn from_vec(capacity: Vec<i32>) -> MultiDimensionalCapacity {
    MultiDimensionalCapacity::from_vec(capacity)
}

#[test]
fn can_sum_multi_dimens() {
    assert_eq!((from_vec(vec![1, 0, 2]) + from_vec(vec![3, 1, 0])).capacity, vec![4, 1, 2]);
    assert_eq!((from_vec(vec![1, 0, 0]) + from_vec(vec![0, 0, 0])).capacity, vec![1, 0, 0]);

    assert_eq!((from_vec(vec![1]) + from_vec(vec![0, 0, 2])).capacity, vec![1, 0, 2]);
    assert_eq!((from_vec(vec![0, 0, 2]) + from_vec(vec![1])).capacity, vec![1, 0, 2]);

    assert_ne!((from_vec(vec![1, 0, 2]) + from_vec(vec![3, 1, 0])).capacity, vec![3, 1, 2]);
}

#[test]
fn can_sub_multi_dimens() {
    assert_eq!((from_vec(vec![3, 0, 2]) - from_vec(vec![1, 1, 4])).capacity, vec![2, -1, -2]);
    assert_eq!((from_vec(vec![3, 0, 2]) - from_vec(vec![0, 0, 0])).capacity, vec![3, 0, 2]);

    assert_eq!((from_vec(vec![1]) - from_vec(vec![0, 0, 2])).capacity, vec![1, 0, -2]);
    assert_eq!((from_vec(vec![0, 0, 2]) - from_vec(vec![1])).capacity, vec![-1, 0, 2]);

    assert_ne!((from_vec(vec![1, 0, 2]) - from_vec(vec![3, 1, 0])).capacity, vec![3, 1, 2]);
}
