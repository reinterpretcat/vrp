use crate::format::solution::{Statistic, Timing};
use std::ops::Add;

impl Add for Statistic {
    type Output = Statistic;

    fn add(self, rhs: Self) -> Self::Output {
        Statistic {
            cost: self.cost + rhs.cost,
            distance: self.distance + rhs.distance,
            duration: self.duration + rhs.duration,
            times: Timing {
                driving: self.times.driving + rhs.times.driving,
                serving: self.times.serving + rhs.times.serving,
                waiting: self.times.waiting + rhs.times.waiting,
                break_time: self.times.break_time + rhs.times.break_time,
                commuting: self.times.commuting + rhs.times.commuting,
                parking: self.times.parking + rhs.times.parking,
            },
        }
    }
}
