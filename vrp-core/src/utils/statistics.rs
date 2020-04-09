/// Returns standard deviation.
pub fn get_stdev(values: &[f64]) -> f64 {
    get_variance_mean(values).0.sqrt()
}

/// Returns coefficient variation.
pub fn get_cv(values: &[f64]) -> f64 {
    let (variance, mean) = get_variance_mean(values);
    let sdev = variance.sqrt();

    sdev / mean
}

/// Gets mean of values.
pub fn get_mean(values: &[f64]) -> f64 {
    let sum: f64 = values.iter().sum();
    sum / values.len() as f64
}

/// Returns variance and mean.
fn get_variance_mean(values: &[f64]) -> (f64, f64) {
    let mean = get_mean(values);

    let (first, second) = values.iter().fold((0., 0.), |acc, v| {
        let dev = v - mean;
        (acc.0 + dev * dev, acc.1 + dev)
    });

    ((first - (second * second / values.len() as f64)) / (values.len() as f64 - 1.), mean)
}
