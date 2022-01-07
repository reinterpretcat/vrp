use super::*;
use crate::helpers::utils::create_test_random;
use std::ops::Range;
use std::sync::{Arc, RwLock};

type ActionCounter = Arc<RwLock<Vec<GridAction>>>;

#[derive(PartialEq, Eq, Hash, Clone)]
enum GridState {
    OnGrid { x: i32, y: i32 },
    Terminal,
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum GridAction {
    Move { dx: i32, dy: i32 },
}

struct GridAgent {
    actions: ActionEstimates<GridState>,
    state: GridState,
    grid: (Range<i32>, Range<i32>),
    terminal: (i32, i32),
    actions_taken: ActionCounter,
}

impl State for GridState {
    type Action = GridAction;

    fn reward(&self) -> f64 {
        match &self {
            GridState::OnGrid { .. } => -1.,
            GridState::Terminal => 10.,
        }
    }
}

impl GridAgent {
    pub fn new(
        actions: ActionEstimates<GridState>,
        state: GridState,
        grid: (Range<i32>, Range<i32>),
        terminal: (i32, i32),
        actions_taken: ActionCounter,
    ) -> Self {
        assert!(actions_taken.read().unwrap().is_empty());
        Self { actions, state, grid, terminal, actions_taken }
    }
}

impl Agent<GridState> for GridAgent {
    fn get_state(&self) -> &GridState {
        &self.state
    }

    fn get_actions(&self, state: &GridState) -> ActionEstimates<GridState> {
        match &state {
            GridState::OnGrid { .. } => self.actions.clone(),
            GridState::Terminal => ActionEstimates::<GridState>::default(),
        }
    }

    fn take_action(&mut self, action: &<GridState as State>::Action) {
        let (x, y) = match self.get_state() {
            GridState::OnGrid { x, y, .. } => (*x, *y),
            GridState::Terminal => unreachable!(),
        };

        let (new_x, new_y) = match action {
            GridAction::Move { dx, dy } => {
                self.actions_taken.write().unwrap().push(action.clone());
                (x + dx, y + dy)
            }
        };

        self.state = if self.terminal.0 == new_x && self.terminal.1 == new_y {
            GridState::Terminal
        } else if self.grid.0.contains(&new_x) && self.grid.1.contains(&new_y) {
            GridState::OnGrid { x: new_x, y: new_y }
        } else {
            GridState::OnGrid { x, y }
        }
    }
}

fn run_simulator(
    simulator: &mut Simulator<GridState>,
    repeat_count: usize,
    agent_count: usize,
    visualize: bool,
    get_agent: impl Fn(ActionCounter) -> GridAgent,
) -> Vec<Vec<Vec<GridAction>>> {
    let mut results = vec![];

    for episode in 0..repeat_count {
        let actions_taken = (0..agent_count).map(|_| Arc::new(RwLock::new(vec![]))).collect::<Vec<_>>();
        let agents = (0..agent_count).map(|idx| Box::new(get_agent(actions_taken[idx].clone()))).collect::<Vec<_>>();

        simulator
            .run_episodes(agents, Parallelism::default(), |_, values| values.iter().sum::<f64>() / values.len() as f64);

        if visualize {
            print_board(simulator, episode);
        }

        let states = actions_taken.iter().map(|at| at.read().unwrap().clone()).collect();
        results.push(states)
    }

    results
}

fn print_board(simulator: &Simulator<GridState>, episode: usize) {
    println!("\nepisode {}: ", episode);
    (0..4).for_each(|y| {
        (0..4).for_each(|x| {
            if let Some((_, value)) = simulator.get_optimal_policy(&GridState::OnGrid { x, y }) {
                print!("|{:>10.7}|", value)
            } else {
                print!("| --none-- |")
            }
        });
        println!()
    });
}

fn create_agent(state: GridState, actions_taken: ActionCounter) -> GridAgent {
    let actions = [
        (GridAction::Move { dx: 1, dy: 0 }, 0.),
        (GridAction::Move { dx: 0, dy: 1 }, 0.),
        (GridAction::Move { dx: -1, dy: 0 }, 0.),
        (GridAction::Move { dx: 0, dy: -1 }, 0.),
    ]
    .iter()
    .cloned()
    .collect::<HashMap<_, _>>();
    let grid = (0..4, 0..4);
    let terminal = (3, 3);

    GridAgent::new(ActionEstimates::from(actions), state, grid.clone(), terminal, actions_taken)
}

parameterized_test! {can_run_grid_episodes_impl, (agent_count, repeat_count, expected_optimal, visualize, policy_strategy), {
    can_run_grid_episodes_impl(agent_count, repeat_count, expected_optimal, visualize, policy_strategy);
}}

can_run_grid_episodes_impl! {
    case01: (1, 1000, Some(100), false, Box::new(Greedy::default())),
    case02: (1, 1000, None, false, Box::new(EpsilonGreedy::new(0.001, create_test_random()))),

    case03: (2, 1000, Some(100), false, Box::new(Greedy::default())),
    case04: (2, 1000, None, false, Box::new(EpsilonGreedy::new(0.001, create_test_random()))),

    case05: (10, 1000, Some(100), false, Box::new(Greedy::default())),
    case06: (10, 1000, None, false, Box::new(EpsilonGreedy::new(0.001, create_test_random()))),
}

fn can_run_grid_episodes_impl(
    agent_count: usize,
    repeat_count: usize,
    expected_optimal: Option<usize>,
    visualize: bool,
    policy_strategy: Box<dyn PolicyStrategy<GridState> + Send + Sync>,
) {
    let learning_strategy = Box::new(QLearning::new(0.2, 0.01));
    let state = GridState::OnGrid { x: 0, y: 0 };
    let mut simulator = Simulator::new(learning_strategy, policy_strategy);

    let actions_taken = run_simulator(&mut simulator, repeat_count, agent_count, visualize, |counter| {
        create_agent(state.clone(), counter)
    });

    assert_eq!(actions_taken.len(), repeat_count);
    // NOTE do not check for EpsilonGreedy test due to its stochastic nature
    if let Some(expected_optimal) = expected_optimal {
        actions_taken.iter().rev().take(expected_optimal).for_each(|agents_actions| {
            assert_eq!(agents_actions.len(), agent_count);
            agents_actions.iter().for_each(|optimal_actions| {
                assert_eq!(optimal_actions.len(), 6);
            });
        });
    }

    for ((x, y), (e_dx, e_dy)) in
        vec![((2, 3), (1, 0)), ((1, 3), (1, 0)), ((0, 3), (1, 0)), ((3, 2), (0, 1)), ((3, 1), (0, 1)), ((3, 0), (0, 1))]
    {
        let (GridAction::Move { dx, dy }, _) = simulator.get_optimal_policy(&GridState::OnGrid { x, y }).unwrap();
        assert_eq!((dx, dy), (e_dx, e_dy));
    }
}
