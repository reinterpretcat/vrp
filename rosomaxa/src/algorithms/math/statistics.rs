use crate::prelude::Float;

/// Returns coefficient variation.
pub fn get_cv(values: &[Float]) -> Float {
    let (variance, mean) = get_variance_mean(values);
    if mean == 0. {
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
pub fn get_mean_slice(values: &[Float]) -> Float {
    if values.is_empty() {
        0.
    } else {
        let sum: Float = values.iter().sum();
        sum / values.len() as Float
    }
}

/// Gets mean of values using given iterator.
pub fn get_mean_iter<Iter>(values: Iter) -> Float
where
    Iter: Iterator<Item = Float>,
{
    let (sum, count) = values.fold((0., 0), |(sum, count), item| (sum + item, count + 1));

    if count == 0 {
        0.
    } else {
        sum / count as Float
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
