#[cfg(test)]
#[path = "../../../tests/unit/models/common/load_test.rs"]
mod load_test;

use crate::models::common::{Dimensions, ValueDimension};
use crate::models::Problem;
use rosomaxa::utils::unwrap_from_result;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Sum;
use std::ops::{Add, Mul, Sub};

const CAPACITY_DIMENSION_KEY: &str = "cpc";
const DEMAND_DIMENSION_KEY: &str = "dmd";
const LOAD_DIMENSION_SIZE: usize = 32;

/// Represents a load type used to represent customer's demand or vehicle's load.
pub trait Load: Add + Sub + PartialOrd + Copy + Default + Debug + Send + Sync {
    /// Returns true if it represents an empty load.
    fn is_not_empty(&self) -> bool;

    /// Returns max load value.
    fn max_load(self, other: Self) -> Self;

    /// Returns true if `other` can be loaded into existing capacity.
    fn can_fit(&self, other: &Self) -> bool;

    /// Returns ratio.
    fn ratio(&self, other: &Self) -> f64;
}

/// Specifies constraints on Load operations.
pub trait LoadOps: Load + Add<Output = Self> + Sub<Output = Self> + 'static
where
    Self: Sized,
{
}

/// Represents job demand, both static and dynamic.
pub struct Demand<T: LoadOps> {
    /// Keeps static and dynamic pickup amount.
    pub pickup: (T, T),
    /// Keeps static and dynamic delivery amount.
    pub delivery: (T, T),
}

/// A trait to get or set vehicle's capacity.
pub trait CapacityDimension<T: LoadOps> {
    /// Sets capacity.
    fn set_capacity(&mut self, demand: T) -> &mut Self;
    /// Gets capacity.
    fn get_capacity(&self) -> Option<&T>;
}

/// A trait to get or set demand.
pub trait DemandDimension<T: LoadOps> {
    /// Sets demand.
    fn set_demand(&mut self, demand: Demand<T>) -> &mut Self;
    /// Gets demand.
    fn get_demand(&self) -> Option<&Demand<T>>;
}

impl<T: LoadOps> Demand<T> {
    /// Returns capacity change as difference between pickup and delivery.
    pub fn change(&self) -> T {
        self.pickup.0 + self.pickup.1 - self.delivery.0 - self.delivery.1
    }
}

impl<T: LoadOps> Default for Demand<T> {
    fn default() -> Self {
        Self { pickup: (Default::default(), Default::default()), delivery: (Default::default(), Default::default()) }
    }
}

impl<T: LoadOps> Clone for Demand<T> {
    fn clone(&self) -> Self {
        Self { pickup: self.pickup, delivery: self.delivery }
    }
}

impl<T: LoadOps> Add for Demand<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            pickup: (self.pickup.0 + rhs.pickup.0, self.pickup.1 + rhs.pickup.1),
            delivery: (self.delivery.0 + rhs.delivery.0, self.delivery.1 + rhs.delivery.1),
        }
    }
}

impl<T: LoadOps> CapacityDimension<T> for Dimensions {
    fn set_capacity(&mut self, demand: T) -> &mut Self {
        self.set_value(CAPACITY_DIMENSION_KEY, demand);
        self
    }

    fn get_capacity(&self) -> Option<&T> {
        self.get_value(CAPACITY_DIMENSION_KEY)
    }
}

impl<T: LoadOps> DemandDimension<T> for Dimensions {
    fn set_demand(&mut self, demand: Demand<T>) -> &mut Self {
        self.set_value(DEMAND_DIMENSION_KEY, demand);
        self
    }

    fn get_demand(&self) -> Option<&Demand<T>> {
        self.get_value(DEMAND_DIMENSION_KEY)
    }
}

/// Specifies single dimensional load type.
#[derive(Clone, Copy, Debug, Default)]
pub struct SingleDimLoad {
    /// An actual load value.
    pub value: i32,
}

impl SingleDimLoad {
    /// Creates a new instance of `SingleDimLoad`.
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}

impl LoadOps for SingleDimLoad {}

impl Load for SingleDimLoad {
    fn is_not_empty(&self) -> bool {
        self.value != 0
    }

    fn max_load(self, other: Self) -> Self {
        let value = self.value.max(other.value);
        Self { value }
    }

    fn can_fit(&self, other: &Self) -> bool {
        self.value >= other.value
    }

    fn ratio(&self, other: &Self) -> f64 {
        self.value as f64 / other.value as f64
    }
}

impl Add for SingleDimLoad {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let value = self.value + rhs.value;
        Self { value }
    }
}

impl Sub for SingleDimLoad {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let value = self.value - rhs.value;
        Self { value }
    }
}

impl Ord for SingleDimLoad {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl PartialOrd for SingleDimLoad {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for SingleDimLoad {}

impl PartialEq for SingleDimLoad {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Mul<f64> for SingleDimLoad {
    type Output = Self;

    fn mul(self, value: f64) -> Self::Output {
        Self::new((self.value as f64 * value).round() as i32)
    }
}

impl Display for SingleDimLoad {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// Specifies multi dimensional load type.
#[derive(Clone, Copy, Debug)]
pub struct MultiDimLoad {
    /// Load data.
    pub load: [i32; LOAD_DIMENSION_SIZE],
    /// Actual used size.
    pub size: usize,
}

impl MultiDimLoad {
    /// Creates a new instance of `MultiDimLoad`.
    pub fn new(data: Vec<i32>) -> Self {
        assert!(data.len() <= LOAD_DIMENSION_SIZE);

        let mut load = [0; LOAD_DIMENSION_SIZE];
        for (idx, value) in data.iter().enumerate() {
            load[idx] = *value;
        }

        Self { load, size: data.len() }
    }

    fn get(&self, idx: usize) -> i32 {
        self.load[idx]
    }

    /// Converts to vector representation.
    pub fn as_vec(&self) -> Vec<i32> {
        if self.size == 0 {
            vec![0]
        } else {
            self.load[..self.size].to_vec()
        }
    }
}

impl Load for MultiDimLoad {
    fn is_not_empty(&self) -> bool {
        self.size == 0 || self.load.iter().any(|v| *v != 0)
    }

    fn max_load(self, other: Self) -> Self {
        let mut result = self;
        result.load.iter_mut().zip(other.load.iter()).for_each(|(a, b)| *a = (*a).max(*b));

        result
    }

    fn can_fit(&self, other: &Self) -> bool {
        self.load.iter().zip(other.load.iter()).all(|(a, b)| a >= b)
    }

    fn ratio(&self, other: &Self) -> f64 {
        self.load.iter().zip(other.load.iter()).fold(0., |acc, (a, b)| (*a as f64 / *b as f64).max(acc))
    }
}

impl LoadOps for MultiDimLoad {}

impl Default for MultiDimLoad {
    fn default() -> Self {
        Self { load: [0; LOAD_DIMENSION_SIZE], size: 0 }
    }
}

impl Add for MultiDimLoad {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        fn sum(acc: MultiDimLoad, rhs: &MultiDimLoad) -> MultiDimLoad {
            let mut dimens = acc;

            for (idx, value) in rhs.load.iter().enumerate() {
                dimens.load[idx] += *value;
            }

            dimens.size = dimens.size.max(rhs.size);

            dimens
        }

        if self.load.len() >= rhs.load.len() {
            sum(self, &rhs)
        } else {
            sum(rhs, &self)
        }
    }
}

impl Sub for MultiDimLoad {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut dimens = self;

        for (idx, value) in rhs.load.iter().enumerate() {
            dimens.load[idx] -= *value;
        }

        dimens.size = dimens.size.max(rhs.size);

        dimens
    }
}

impl PartialOrd for MultiDimLoad {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let size = self.size.max(other.size);
        unwrap_from_result((0..size).try_fold(None, |acc, idx| {
            let result = self.get(idx).cmp(&other.get(idx));
            acc.map_or(Ok(Some(result)), |acc| if acc != result { Err(None) } else { Ok(Some(result)) })
        }))
    }
}

impl Eq for MultiDimLoad {}

impl PartialEq for MultiDimLoad {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).map_or(false, |ordering| ordering == Ordering::Equal)
    }
}

impl Mul<f64> for MultiDimLoad {
    type Output = Self;

    fn mul(self, value: f64) -> Self::Output {
        let mut dimens = self;

        dimens.load.iter_mut().for_each(|item| {
            *item = (*item as f64 * value).round() as i32;
        });

        dimens
    }
}

impl Sum for MultiDimLoad {
    fn sum<I: Iterator<Item = MultiDimLoad>>(iter: I) -> Self {
        iter.fold(MultiDimLoad::default(), |acc, item| item + acc)
    }
}

impl Display for MultiDimLoad {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.load)
    }
}

/// Returns true if any of the jobs has multi dimensional demand.
pub fn has_multi_dim_demand(problem: &Problem) -> bool {
    problem.jobs.all().any(|job| {
        let demand: Option<&Demand<MultiDimLoad>> = job.dimens().get_demand();
        demand.is_some()
    })
}
