use super::{IndividualNetwork, RosomaxaConfig, RosomaxaContext, RosomaxaSolution};
use crate::HeuristicStatistics;
use crate::evolution::objectives::HeuristicObjective;
use crate::population::elitism::Alternative;
use crate::utils::Float;
use std::f64::consts::{E, PI};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum NetworkMaintenanceAction {
    RefreshNormalization,
    CheckDistortion,
}

/// Keeps smoothing responsive without letting repeated full-map replay dominate ordinary training.
pub(super) struct NetworkMaintenance {
    /// Inputs added since the last distortion check; smoothing and compaction replay does not contribute.
    pub(super) new_input_count: usize,
    /// Inputs added since the normalization ranges were rebuilt from retained solutions.
    pub(super) normalization_input_count: usize,
    /// Small maps wait for this many observations before checking distortion.
    min_observation_count: usize,
    /// Consecutive smoothing grows the evidence window; a stable observation gradually shrinks it again.
    pub(super) observation_multiplier: usize,
    max_observation_multiplier: usize,
}

impl NetworkMaintenance {
    pub(super) fn new(config: &RosomaxaConfig) -> Self {
        Self {
            new_input_count: 0,
            normalization_input_count: 0,
            min_observation_count: config.max_network_size.div_ceil(6),
            observation_multiplier: 1,
            max_observation_multiplier: get_max_smoothing_observation_multiplier(config.node_size),
        }
    }

    pub(super) fn add_observations(&mut self, count: usize) {
        self.new_input_count = self.new_input_count.saturating_add(count);
        self.normalization_input_count = self.normalization_input_count.saturating_add(count);
    }

    fn base_observation_count(&self, network_size: usize) -> usize {
        network_size.max(self.min_observation_count)
    }

    pub(super) fn next_action(&mut self, network_size: usize) -> Option<NetworkMaintenanceAction> {
        let observation_count = self.base_observation_count(network_size);
        let is_distortion_due = self.new_input_count >= observation_count.saturating_mul(self.observation_multiplier);
        let is_normalization_due = self.normalization_input_count >= observation_count;

        if is_distortion_due || is_normalization_due {
            self.normalization_input_count = 0;
        }

        if is_distortion_due {
            Some(NetworkMaintenanceAction::CheckDistortion)
        } else if is_normalization_due {
            Some(NetworkMaintenanceAction::RefreshNormalization)
        } else {
            None
        }
    }

    pub(super) fn on_smoothing(&mut self) {
        self.new_input_count = 0;
        self.normalization_input_count = 0;
        self.observation_multiplier =
            self.observation_multiplier.saturating_mul(2).min(self.max_observation_multiplier);
    }

    pub(super) fn on_stable_observation(&mut self) {
        self.new_input_count = 0;
        self.normalization_input_count = 0;
        self.observation_multiplier = self.observation_multiplier.div_ceil(2).max(1);
    }
}

pub(super) fn optimize_network<C, O, S>(
    external_ctx: &C,
    network: &mut IndividualNetwork<C, O, S>,
    maintenance: &mut NetworkMaintenance,
    statistics: &HeuristicStatistics,
    config: &RosomaxaConfig,
) where
    C: RosomaxaContext<Solution = S>,
    O: HeuristicObjective<Solution = S> + Alternative,
    S: RosomaxaSolution<Context = C>,
{
    network.set_learning_rate(get_learning_rate(statistics.termination_estimate));

    let keep_size = get_keep_size(config.max_network_size, statistics.termination_estimate);
    if network.size() > keep_size {
        // Compaction already rebuilds the map, so handle it before periodic smoothing and start a fresh
        // evidence window.
        network.compact(external_ctx);
        network.smooth(external_ctx, 1, |i| i.on_update(external_ctx));
        maintenance.on_smoothing();
        return;
    }

    match maintenance.next_action(network.size()) {
        Some(NetworkMaintenanceAction::RefreshNormalization) => {
            // Keep feature ranges representative of retained solutions even while expensive replay is backed off.
            network.refresh_normalization();
        }
        Some(NetworkMaintenanceAction::CheckDistortion) => {
            // Distortion has to be measured using the current retained population, not historical outliers.
            network.refresh_normalization();

            // Let a young map learn enough topology before smoothing can reset the errors which drive GSOM growth.
            let can_smooth = network.size() >= get_min_network_size(config.max_network_size);
            // Set the MSE threshold to a fraction of the maximum possible normalized distance.
            let threshold = 0.5 / (network.dimension() as Float).sqrt();
            let should_smooth = can_smooth && network.mse() > threshold;

            if should_smooth {
                network.smooth(external_ctx, 1, |i| i.on_update(external_ctx));
                maintenance.on_smoothing();
            } else {
                maintenance.on_stable_observation();
            }
        }
        None => {}
    }
}

/// Gets the minimum useful network size derived from its configured capacity.
pub(super) fn get_min_network_size(max_network_size: usize) -> usize {
    (max_network_size / 3).max(4).min(max_network_size)
}

/// Caps adaptive smoothing backoff. Four ordinary assignments per retained node item keep steady-state replay work
/// near one quarter of ordinary GSOM assignment work for the usual small node capacities. The upper bound keeps
/// maintenance reachable when a custom configuration uses larger node storage.
pub(super) fn get_max_smoothing_observation_multiplier(node_size: usize) -> usize {
    node_size.saturating_mul(4).clamp(4, 16)
}

/// Gets the network size at which compaction is triggered.
/// Slowly decreases the trigger from the configured maximum to two thirds of it. As compaction retains roughly half
/// of the lattice, the map does not fall far below one third of its configured capacity.
pub(super) fn get_keep_size(max_network_size: usize, termination_estimate: Float) -> usize {
    #![allow(clippy::unnecessary_cast)]
    let termination_estimate = termination_estimate.clamp(0., 0.8) as f64;
    // Sigmoid: https://www.wolframalpha.com/input?i=plot+1+*+%281%2F%281%2Be%5E%28-10+*%28x+-+0.5%29%29%29%29%2C+x%3D0+to+1
    let rate = 1. / (1. + E.powf(-10. * (termination_estimate - 0.5)));
    let min_compaction_size = get_min_network_size(max_network_size).saturating_mul(2).min(max_network_size);
    let network_size_range = max_network_size - min_compaction_size;

    min_compaction_size + (network_size_range as Float * (1. - rate) as Float) as usize
}

/// Gets learning rate decay using cosine annealing.
/// `Cosine Annealing` is a type of learning rate schedule that has the effect of starting with a large
/// learning rate that is relatively rapidly decreased to a minimum value before being increased rapidly again.
pub(super) fn get_learning_rate(termination_estimate: Float) -> Float {
    #![allow(clippy::unnecessary_cast)]

    const PERIOD: Float = 0.25;
    const MIN_LEARNING_RATE: Float = 0.1;
    const MAX_LEARNING_RATE: Float = 1.0;

    assert!((0. ..=1.).contains(&termination_estimate), "termination estimate must be in [0, 1]");

    let min_lr = MIN_LEARNING_RATE;
    let max_lr = MAX_LEARNING_RATE;

    let progress = termination_estimate % PERIOD;
    let progress = progress / PERIOD;
    let progress_pi = (progress as f64 * PI) as Float;

    min_lr + 0.5 * (max_lr - min_lr) * (1. + progress_pi.cos())
}
