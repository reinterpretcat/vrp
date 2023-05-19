use super::*;
use crate::population::Greedy;
use crate::utils::Timer;
use crate::DynHeuristicPopulation;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

// TODO this is an experimental implementation

/// Specifies async heuristic factory.
pub type AsyncHeuristicFactory<H> = Box<dyn Fn() -> H>;
/// Specifies async context factory.
pub type AsyncContextFactory<C, O, S> = Box<dyn Fn(&C, Box<DynHeuristicPopulation<O, S>>) -> C>;

/// Specifies parameters for asynchronous heuristic.
pub struct AsyncParams {
    /// Size of actors processing solutions in parallel.
    pub actors_size: usize,
    /// A size of actor's channel buffer
    pub channel_buffer: usize,
    /// Selection size.
    pub selection_size: usize,
}

/// An asynchronous simple evolution algorithm which maintains a single population and improves it iteratively.
pub struct AsyncIterative<H, C, O, S> {
    params: AsyncParams,
    desired_solutions_amount: usize,
    objective: Arc<O>,
    heuristic_factory: AsyncHeuristicFactory<H>,
    context_factory: AsyncContextFactory<C, O, S>,
}

impl<H, C, O, S> EvolutionStrategy for AsyncIterative<H, C, O, S>
where
    H: HyperHeuristic<Context = C, Objective = O, Solution = S> + Send + 'static,
    C: HeuristicContext<Objective = O, Solution = S> + Send + 'static,
    O: HeuristicObjective<Solution = S> + Send + 'static,
    S: HeuristicSolution + Send + 'static,
{
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn run(
        &mut self,
        mut heuristic_ctx: Self::Context,
        termination: Box<dyn Termination<Context = Self::Context, Objective = Self::Objective>>,
    ) -> EvolutionResult<Self::Solution> {
        let (host_sender, mut host_receiver) = mpsc::channel(self.params.channel_buffer);

        // NOTE we use multi-thread runtime as calling heuristic search method could be expensive.
        let runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();

        runtime.block_on(async {
            let actors = (0..self.params.actors_size)
                .map(|_| {
                    let heuristic = (self.heuristic_factory)();
                    let context = self.fork_context(&heuristic_ctx);

                    HeuristicActorHandle::new(heuristic, context)
                })
                .collect::<Vec<_>>();

            let mut selected_solutions = Vec::with_capacity(self.params.selection_size);
            let mut processed_solutions = Vec::with_capacity(self.params.selection_size);
            let mut generation_time = Timer::start();
            let mut actors = HeuristicActors::new(actors, host_sender);

            loop {
                let is_terminated = termination.is_termination(&mut heuristic_ctx);
                let is_quota_reached = heuristic_ctx.environment().quota.as_ref().map_or(false, |q| q.is_reached());

                if is_terminated || is_quota_reached {
                    break;
                }

                // get take size considering actors availability and selection size
                let take = if heuristic_ctx.statistics().generation == 0 {
                    self.params.selection_size
                } else {
                    actors.available()
                };

                // solutions are processed in batch
                if selected_solutions.is_empty() {
                    selected_solutions.extend(heuristic_ctx.selected().map(|solution| solution.deep_copy()));
                }

                actors.search(selected_solutions.drain(0..take.min(selected_solutions.len()))).await;

                if let Some((solutions, idx)) = host_receiver.recv().await {
                    actors.free_actor(idx);
                    processed_solutions.extend(solutions.into_iter());
                }

                if processed_solutions.len() == self.params.selection_size {
                    let termination_estimate = termination.estimate(&heuristic_ctx);
                    heuristic_ctx.on_generation(
                        processed_solutions.drain(0..).collect(),
                        termination_estimate,
                        generation_time.clone(),
                    );
                    generation_time = Timer::start();
                    actors
                        .new_generation((0..self.params.actors_size).map(|_| self.fork_context(&heuristic_ctx)))
                        .await;
                }
            }
        });

        let (population, telemetry_metrics) = heuristic_ctx.on_result()?;

        let solutions =
            population.ranked().map(|(solution, _)| solution.deep_copy()).take(self.desired_solutions_amount).collect();

        Ok((solutions, telemetry_metrics))
    }
}

impl<H, C, O, S> AsyncIterative<H, C, O, S>
where
    H: HyperHeuristic<Context = C, Objective = O, Solution = S> + Send + 'static,
    C: HeuristicContext<Objective = O, Solution = S> + Send + 'static,
    O: HeuristicObjective<Solution = S> + Send + 'static,
    S: HeuristicSolution + Send + 'static,
{
    /// Creates a new instance of `AsyncIterative` evolution strategy.
    pub fn new(
        params: AsyncParams,
        desired_solutions_amount: usize,
        objective: Arc<O>,
        heuristic_factory: AsyncHeuristicFactory<H>,
        context_factory: AsyncContextFactory<C, O, S>,
    ) -> Self {
        Self { params, desired_solutions_amount, objective, heuristic_factory, context_factory }
    }

    fn fork_context(&self, heuristic_ctx: &C) -> C {
        let objective = self.objective.clone();
        let population = Greedy::new(objective, 1, heuristic_ctx.ranked().next().map(|(best, _)| best.deep_copy()));

        (self.context_factory)(heuristic_ctx, Box::new(population))
    }
}

/// Defines messages which can be set to actors.
enum EvolutionMessage<C, S> {
    Search { solution: S, respond_to: oneshot::Sender<Vec<S>> },
    NewGeneration { context: C, respond_to: oneshot::Sender<()> },
}

/// A heuristic actor which doing an actual refinement.
struct HeuristicActor<H, C, O, S>
where
    H: HyperHeuristic<Context = C, Objective = O, Solution = S>,
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    heuristic: H,
    context: C,
    receiver: mpsc::Receiver<EvolutionMessage<C, S>>,
}

impl<H, C, O, S> HeuristicActor<H, C, O, S>
where
    H: HyperHeuristic<Context = C, Objective = O, Solution = S>,
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    fn new(heuristic: H, context: C, receiver: mpsc::Receiver<EvolutionMessage<C, S>>) -> Self {
        HeuristicActor { heuristic, context, receiver }
    }

    fn handle_message(&mut self, msg: EvolutionMessage<C, S>) {
        match msg {
            EvolutionMessage::Search { solution, respond_to } => {
                let mut solutions = self.heuristic.search(&self.context, &solution);

                if self.context.selection_phase() == SelectionPhase::Exploration {
                    let diversified = self.heuristic.diversify(&self.context, &solution);
                    solutions.extend(diversified.into_iter());
                }

                let _ = respond_to.send(solutions);
            }
            EvolutionMessage::NewGeneration { context, respond_to } => {
                self.context = context;
                let _ = respond_to.send(());
            }
        }
    }
}

struct HeuristicActorHandle<C, O, S> {
    sender: mpsc::Sender<EvolutionMessage<C, S>>,
    phantom: PhantomData<O>,
}

impl<C, O, S> Clone for HeuristicActorHandle<C, O, S> {
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone(), phantom: Default::default() }
    }
}

impl<C, O, S> HeuristicActorHandle<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + Send + 'static,
    O: HeuristicObjective<Solution = S> + Send + 'static,
    S: HeuristicSolution + Send + 'static,
{
    pub fn new<H>(heuristic: H, context: C) -> Self
    where
        H: HyperHeuristic<Context = C, Objective = O, Solution = S> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(8);
        let mut actor = HeuristicActor::new(heuristic, context, receiver);
        tokio::spawn(async move {
            while let Some(msg) = actor.receiver.recv().await {
                actor.handle_message(msg);
            }
        });

        Self { sender, phantom: Default::default() }
    }

    pub async fn search(self, solution: S) -> Vec<S> {
        let (send, recv) = oneshot::channel();
        let message = EvolutionMessage::Search { solution, respond_to: send };

        let _ = self.sender.send(message).await;
        recv.await.expect("actor task has been killed")
    }

    pub async fn new_generation(self, context: C) {
        let (send, recv) = oneshot::channel();
        let message = EvolutionMessage::NewGeneration { context, respond_to: send };

        let _ = self.sender.send(message).await;
        recv.await.expect("actor task has been killed")
    }
}

struct HeuristicActors<C, O, S> {
    actors: Vec<HeuristicActorHandle<C, O, S>>,
    usage: Vec<usize>,
    host_sender: mpsc::Sender<(Vec<S>, usize)>,
}

impl<C, O, S> HeuristicActors<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S> + 'static,
    O: HeuristicObjective<Solution = S> + 'static,
    S: HeuristicSolution + Send + 'static,
{
    pub fn new(actors: Vec<HeuristicActorHandle<C, O, S>>, host_sender: mpsc::Sender<(Vec<S>, usize)>) -> Self {
        let size = actors.len();
        Self { actors, usage: vec![0; size], host_sender }
    }

    pub async fn search<I>(&mut self, solutions: I)
    where
        I: IntoIterator<Item = S>,
    {
        solutions.into_iter().zip((0..).map(|_| (self.get_actor(), self.host_sender.clone()))).for_each(
            |(solution, ((actor, idx), host_sender))| {
                tokio::spawn(async move {
                    let solutions = actor.search(solution).await;
                    host_sender.send((solutions, idx)).await
                });
            },
        )
    }

    pub async fn new_generation<I>(&mut self, contexts: I)
    where
        I: IntoIterator<Item = C>,
    {
        self.actors.iter().cloned().zip(contexts.into_iter()).for_each(|(actor, context)| {
            tokio::spawn(async move { actor.new_generation(context).await });
        })
    }

    fn get_actor(&mut self) -> (HeuristicActorHandle<C, O, S>, usize) {
        let (idx, _) = self.usage.iter().enumerate().min_by(|(_, &a), (_, b)| a.cmp(b)).unwrap();
        self.usage[idx] += 1;

        (self.actors[idx].clone(), idx)
    }

    fn free_actor(&mut self, idx: usize) {
        self.usage[idx] -= 1;
    }

    fn available(&self) -> usize {
        self.usage.iter().filter(|&&u| u == 0).count()
    }
}
