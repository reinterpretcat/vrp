use crate::prelude::Float;
use crate::utils::{compare_floats, IntoFloat};
use std::cmp::Ordering;
use std::ops::Add;

/// Returns coefficient variation.
pub fn get_cv(values: &[Float]) -> Float {
    let (variance, mean) = get_variance_mean(values);
    if compare_floats(mean, 0.) == Ordering::Equal {
        return 0.;
    }
    let sdev = variance.sqrt();

    sdev / mean
}

/// Returns coefficient of variation without NaN (1 is returned instead).
pub fn get_cv_safe(values: &[Float]) -> Float {
    let value = get_cv(values);

    if value.is_nan() {
        1.
    } else {
        value
    }
}

/// Gets mean of values using given slice.
pub fn get_mean_slice<T>(values: &[T]) -> Float
where
    T: Default + Add<Output = T> + IntoFloat + Copy,
{
    if values.is_empty() {
        Float::default()
    } else {
        get_mean_iter(values.iter().copied())
    }
}

/// Gets mean of values using given iterator.
pub fn get_mean_iter<T, Iter>(values: Iter) -> Float
where
    T: Default + Add<Output = T> + IntoFloat + Copy,
    Iter: Iterator<Item = T>,
{
    let (sum, count) = values.fold((T::default(), 0), |(sum, count), item| (sum + item, count + 1));

    if count == 0 {
        Float::default()
    } else {
        sum.into_float() / count as Float
    }
}

/// Returns variance.
pub fn get_variance(values: &[Float]) -> Float {
    get_variance_mean(values).0
}

/// Returns standard deviation.
pub fn get_stdev(values: &[Float]) -> Float {
    get_variance_mean(values).0.sqrt()
}

/// Returns variance and mean.
fn get_variance_mean(values: &[Float]) -> (Float, Float) {
    let mean = get_mean_slice(values);

    let (first, second) = values.iter().fold((0., 0.), |acc, v| {
        let dev = v - mean;
        (acc.0 + dev * dev, acc.1 + dev)
    });

    // NOTE Bessel's correction is not used here
    ((first - (second * second / values.len() as Float)) / (values.len() as Float), mean)
}
