use crate::utils::compare_floats;
use std::cmp::Ordering;

/// Returns coefficient variation.
pub fn get_cv(values: &[f64]) -> f64 {
    let (variance, mean) = get_variance_mean(values);
    if compare_floats(mean, 0.) == Ordering::Equal {
        return 0.;
    }
    let sdev = variance.sqrt();

    sdev / mean
}

/// Returns coefficient of variation without NaN (1 is returned instead).
pub fn get_cv_safe(values: &[f64]) -> f64 {
    let value = get_cv(values);

    if value.is_nan() {
        1.
    } else {
        value
    }
}

/// Gets mean of values using given slice.
pub fn get_mean_slice(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.
    } else {
        let sum: f64 = values.iter().sum();
        sum / values.len() as f64
    }
}

/// Gets mean of values using given iterator.
pub fn get_mean_iter<Iter>(values: Iter) -> f64
where
    Iter: Iterator<Item = f64>,
{
    let (sum, count) = values.fold((0., 0), |(sum, count), item| (sum + item, count + 1));

    if count == 0 {
        0.
    } else {
        sum / count as f64
    }
}

/// Returns variance.
pub fn get_variance(values: &[f64]) -> f64 {
    get_variance_mean(values).0
}

/// Returns standard deviation.
pub fn get_stdev(values: &[f64]) -> f64 {
    get_variance_mean(values).0.sqrt()
}

/// Returns variance and mean.
fn get_variance_mean(values: &[f64]) -> (f64, f64) {
    let mean = get_mean_slice(values);

    let (first, second) = values.iter().fold((0., 0.), |acc, v| {
        let dev = v - mean;
        (acc.0 + dev * dev, acc.1 + dev)
    });

    // NOTE Bessel's correction is not used here
    ((first - (second * second / values.len() as f64)) / (values.len() as f64), mean)
}
