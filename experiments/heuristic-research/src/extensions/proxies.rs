use rosomaxa::example::*;
use rosomaxa::prelude::*;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::time::Duration;

/// A type alias for vector based population.
pub type VectorPopulation =
    Box<dyn HeuristicPopulation<Objective = VectorObjective, Individual = VectorSolution> + Send + Sync>;

/// Keeps track of senders.
pub struct Senders {
    /// Called on new individual addition.
    pub on_add: SyncSender<VectorSolution>,
    /// Called on new individuals addition.
    pub on_add_all: SyncSender<Vec<VectorSolution>>,
    /// Called on individual selection.
    pub on_select: SyncSender<VectorSolution>,
    /// Called on generation.
    pub on_generation: SyncSender<(HeuristicStatistics, Vec<(VectorSolution, usize)>)>,
    /// Specifies thread delay duration.
    pub delay: Option<Duration>,
}

/// Keeps track of receivers.
pub struct Receivers {
    /// Called on new individual addition.
    pub on_add: Receiver<VectorSolution>,
    /// Called on new individuals addition.
    pub on_add_all: Receiver<Vec<VectorSolution>>,
    /// Called on individual selection.
    pub on_select: Receiver<VectorSolution>,
    /// Called on generation.
    pub on_generation: Receiver<(HeuristicStatistics, Vec<(VectorSolution, usize)>)>,
}

/// Creates channels to get population invocation callbacks.
pub fn create_channels(bound: usize, delay: Option<Duration>) -> (Senders, Receivers) {
    let (on_add_sender, on_add_receiver) = sync_channel(bound);
    let (on_add_all_sender, on_add_all_receiver) = sync_channel(bound);
    let (on_select_sender, on_select_receiver) = sync_channel(bound);
    let (on_generation_sender, on_generation_receiver) = sync_channel(bound);

    let senders = Senders {
        on_add: on_add_sender,
        on_add_all: on_add_all_sender,
        on_select: on_select_sender,
        on_generation: on_generation_sender,
        delay,
    };

    let receiver = Receivers {
        on_add: on_add_receiver,
        on_add_all: on_add_all_receiver,
        on_select: on_select_receiver,
        on_generation: on_generation_receiver,
    };

    (senders, receiver)
}

impl Senders {
    /// Calls thread delay if it is configured.
    pub fn delay(&self) {
        if let Some(delay) = self.delay {
            std::thread::sleep(delay);
        }
    }
}

/// A population type which provides way to intercept some of population data.
pub struct ProxyPopulation {
    inner: VectorPopulation,
    senders: Senders,
}

impl ProxyPopulation {
    /// Creates a new instance of `ProxyPopulation`.
    pub fn new(inner: VectorPopulation, senders: Senders) -> Self {
        Self { inner, senders }
    }
}

impl HeuristicPopulation for ProxyPopulation {
    type Objective = VectorObjective;
    type Individual = VectorSolution;

    fn add_all(&mut self, individuals: Vec<Self::Individual>) -> bool {
        self.senders.on_add_all.send(individuals.iter().map(|i| i.deep_copy()).collect()).unwrap();
        self.senders.delay();
        self.inner.add_all(individuals)
    }

    fn add(&mut self, individual: Self::Individual) -> bool {
        self.senders.on_add.send(individual.deep_copy()).unwrap();
        self.senders.delay();
        self.inner.add(individual)
    }

    fn on_generation(&mut self, statistics: &HeuristicStatistics) {
        let individuals = self.inner.ranked().map(|(individual, rank)| (individual.deep_copy(), rank)).collect();
        self.senders.on_generation.send((statistics.clone(), individuals)).unwrap();
        self.senders.delay();
        self.inner.on_generation(statistics)
    }

    fn cmp(&self, a: &Self::Individual, b: &Self::Individual) -> Ordering {
        self.inner.cmp(a, b)
    }

    fn select<'a>(&'a self) -> Box<dyn Iterator<Item = &Self::Individual> + 'a> {
        Box::new(self.inner.select().map(|individual| {
            self.senders.on_select.send(individual.deep_copy()).unwrap();
            individual
        }))
    }

    fn ranked<'a>(&'a self) -> Box<dyn Iterator<Item = (&Self::Individual, usize)> + 'a> {
        self.inner.ranked()
    }

    fn size(&self) -> usize {
        self.inner.size()
    }

    fn selection_phase(&self) -> SelectionPhase {
        self.inner.selection_phase()
    }
}

impl Display for ProxyPopulation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
