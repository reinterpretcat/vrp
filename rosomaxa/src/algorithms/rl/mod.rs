//! This module contains implementation of some reinforcement learning algorithms.

mod slot_machine;
pub(crate) use self::slot_machine::{BernoulliParams, BernoulliPosterior};
pub use self::slot_machine::{SlotAction, SlotFeedback, SlotMachine};
