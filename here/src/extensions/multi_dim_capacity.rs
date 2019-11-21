#[cfg(test)]
#[path = "../../tests/unit/extensions/multi_dim_capacity_test.rs"]
mod multi_dim_capacity_test;

use std::cmp::Ordering;
use std::ops::{Add, Sub};

/// Specifies multi dimensional capacity type.
/// Ordering trait is implemented the following way:
/// Less is returned when at least one dimension is less, others can be equal
/// Equal is returned when all dimensions are equal
/// Greater is returned when at least one dimension is greater than in rhs
#[derive(Clone, Debug)]
pub struct MultiDimensionalCapacity {
    pub capacity: Vec<i32>,
}

impl MultiDimensionalCapacity {
    pub fn new(capacity: Vec<i32>) -> Self {
        Self { capacity }
    }

    fn get(&self, idx: usize) -> i32 {
        *self.capacity.get(idx).unwrap_or(&0)
    }
}

impl Add for MultiDimensionalCapacity {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        fn sum(acc: MultiDimensionalCapacity, rhs: &MultiDimensionalCapacity) -> MultiDimensionalCapacity {
            assert!(acc.capacity.len() >= rhs.capacity.len());

            let mut dimens = acc;

            for (idx, value) in rhs.capacity.iter().enumerate() {
                dimens.capacity[idx] += value;
            }

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
        let mut dimens = if self.capacity.len() >= rhs.capacity.len() {
            self
        } else {
            let mut dimens = self;
            dimens.capacity.resize(rhs.capacity.len(), 0);

            dimens
        };

        for (idx, value) in rhs.capacity.iter().enumerate() {
            dimens.capacity[idx] -= value;
        }

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
