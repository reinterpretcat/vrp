use super::*;
use std::marker::PhantomData;

/// Provides way to control GSOM size under control.
/// TODO investigate different ideas, such as:
/// - freeze network growth for some time
/// - try to avoid broken network connectivity between nodes
/// - different "magic" coefficients
/// - etc.
pub(super) struct NetworkOptimization<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    rebalance_memory: usize,
    init_learning_rate: f64,
    last_optimization: usize,
    phantom_o: PhantomData<O>,
    phantom_s: PhantomData<S>,
}

impl<O, S> NetworkOptimization<O, S>
where
    O: HeuristicObjective<Solution = S> + Shuffled,
    S: HeuristicSolution + RosomaxaWeighted + DominanceOrdered,
{
    /// Creates an instance of `NetworkOptimization`.
    pub fn new(rebalance_memory: usize, init_learning_rate: f64) -> Self {
        Self {
            rebalance_memory,
            last_optimization: 0,
            init_learning_rate,
            phantom_o: Default::default(),
            phantom_s: Default::default(),
        }
    }

    /// Tries to optimize network size. Returns true, if compact procedure was called.
    pub(super) fn optimize_network(&mut self, network: &mut IndividualNetwork<O, S>, statistics: &HeuristicStatistics) {
        let can_grow = self.can_grow(network, statistics);

        if statistics.generation % self.rebalance_memory == 0 {
            network.smooth(1);
        }

        // no need to shrink network
        if can_grow {
            network.set_can_grow(true);
            return;
        }
        /*
        // not enough generation proceeded from the last shrink, let's freeze network growth instead
        if self.last_optimization + self.rebalance_memory > statistics.generation {
            network.set_can_grow(can_grow);
            return;
        }*/

        if self.init_learning_rate < 1. {
            network.set_learning_rate(statistics.termination_estimate.clamp(self.init_learning_rate, 1.));
        }

        network.compact();
        network.smooth(1);
        network.set_can_grow(true);
        self.last_optimization = statistics.generation;
    }

    fn can_grow(&self, network: &IndividualNetwork<O, S>, statistics: &HeuristicStatistics) -> bool {
        let ratio = get_network_size_ratio(statistics);
        let keep_size = self.rebalance_memory + (self.rebalance_memory as f64 * ratio) as usize;

        network.size() <= keep_size
    }
}

/// Gets a ratio for network size.
fn get_network_size_ratio(statistics: &HeuristicStatistics) -> f64 {
    // https://www.wolframalpha.com/input?i=plot+2+*+%281+-+1%2F%281%2Be%5E%28-10+*%28x+-+0.5%29%29%29%29%2C+x%3D0+to+1
    let x = match statistics.improvement_1000_ratio {
        v if v < 0.25 => v,
        _ => statistics.termination_estimate,
    }
    .clamp(0., 1.);

    2. * (1. - 1. / (1. + std::f64::consts::E.powf(-10. * (x - 0.5))))
}
