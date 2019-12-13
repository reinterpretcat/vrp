#[cfg(test)]
#[path = "../../tests/unit/extensions/multi_dim_capacity_test.rs"]
mod multi_dim_capacity_test;

use std::cmp::Ordering;
use std::ops::{Add, Mul, Sub};

const CAPACITY_DIMENSION_SIZE: usize = 8;

/// Specifies multi dimensional capacity type.
/// Ordering trait is implemented the following way:
/// Less is returned when at least one dimension is less, others can be equal
/// Equal is returned when all dimensions are equal
/// Greater is returned when at least one dimension is greater than in rhs
#[derive(Clone, Copy, Debug)]
pub struct MultiDimensionalCapacity {
    pub capacity: [i32; CAPACITY_DIMENSION_SIZE],
    pub size: usize,
}

impl MultiDimensionalCapacity {
    pub fn new(data: Vec<i32>) -> Self {
        assert!(data.len() <= CAPACITY_DIMENSION_SIZE);

        let mut capacity = [0; CAPACITY_DIMENSION_SIZE];
        for (idx, value) in data.iter().enumerate() {
            capacity[idx] = *value;
        }

        Self { capacity, size: data.len() }
    }

    fn get(&self, idx: usize) -> i32 {
        self.capacity[idx]
    }

    pub fn as_vec(&self) -> Vec<i32> {
        if self.size == 0 {
            vec![0]
        } else {
            self.capacity[..self.size].to_vec()
        }
    }
}

impl Default for MultiDimensionalCapacity {
    fn default() -> Self {
        Self { capacity: [0; CAPACITY_DIMENSION_SIZE], size: 0 }
    }
}

impl Add for MultiDimensionalCapacity {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        fn sum(acc: MultiDimensionalCapacity, rhs: &MultiDimensionalCapacity) -> MultiDimensionalCapacity {
            let mut dimens = acc;

            for (idx, value) in rhs.capacity.iter().enumerate() {
                dimens.capacity[idx] += value;
            }

            dimens.size = dimens.size.max(rhs.size);

            dimens
        }

        if self.capacity.len() >= rhs.capacity.len() {
            sum(self, &rhs)
        } else {
            sum(rhs, &self)
        }
    }
}

impl Sub for MultiDimensionalCapacity {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut dimens = self;

        for (idx, value) in rhs.capacity.iter().enumerate() {
            dimens.capacity[idx] -= value;
        }

        dimens.size = dimens.size.max(rhs.size);

        dimens
    }
}

impl Ord for MultiDimensionalCapacity {
    fn cmp(&self, other: &Self) -> Ordering {
        let size = self.capacity.len().max(other.capacity.len());
        (0..size).fold(Ordering::Equal, |acc, idx| match acc {
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => {
                if self.get(idx) > other.get(idx) {
                    Ordering::Greater
                } else if self.get(idx) == other.get(idx) {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            }
            Ordering::Less => {
                if self.get(idx) > other.get(idx) {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
        })
    }
}

impl PartialOrd for MultiDimensionalCapacity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for MultiDimensionalCapacity {}

impl PartialEq for MultiDimensionalCapacity {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Mul<f64> for MultiDimensionalCapacity {
    type Output = Self;

    fn mul(self, value: f64) -> Self::Output {
        let mut dimens = self;

        dimens.capacity.iter_mut().for_each(|item| {
            *item = (*item as f64 * value).round() as i32;
        });

        dimens
    }
}
