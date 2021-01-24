use super::*;
use std::ops::Range;
use std::sync::{Arc, RwLock};

#[derive(PartialEq, Eq, Hash, Clone)]
enum GridState {
    OnGrid { x: i32, y: i32 },
    OffGrid { x: i32, y: i32 },
    Terminal,
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum GridAction {
    Move { dx: i32, dy: i32 },
}

struct GridAgent {
    actions: ActionsEstimate<GridState>,
    state: GridState,
    grid: (Range<i32>, Range<i32>),
    terminal: (i32, i32),
    actions_taken: Arc<RwLock<usize>>,
}

impl State for GridState {
    type Action = GridAction;

    fn reward(&self) -> f64 {
        match &self {
            GridState::OnGrid { .. } => -1.,
            GridState::OffGrid { .. } => -10.,
            GridState::Terminal => 10.,
        }
    }
}

impl GridAgent {
    pub fn new(
        actions: ActionsEstimate<GridState>,
        state: GridState,
        grid: (Range<i32>, Range<i32>),
        terminal: (i32, i32),
        actions_taken: Arc<RwLock<usize>>,
    ) -> Self {
        assert_eq!(*actions_taken.read().unwrap(), 0);
        Self { actions, state, grid, terminal, actions_taken }
    }
}

impl Agent<GridState> for GridAgent {
    fn get_state(&self) -> &GridState {
        &self.state
    }

    fn get_actions(&self, state: &GridState) -> ActionsEstimate<GridState> {
        match &state {
            GridState::OnGrid { .. } => self.actions.clone(),
            GridState::OffGrid { .. } => self.actions.clone(),
            GridState::Terminal => ActionsEstimate::<GridState>::default(),
        }
    }

    fn take_action(&mut self, action: &<GridState as State>::Action) {
        let (x, y) = match self.get_state() {
            GridState::OnGrid { x, y, .. } => (*x, *y),
            GridState::OffGrid { x, y, .. } => (*x, *y),
            GridState::Terminal => unreachable!(),
        };

        let (new_x, new_y) = match action {
            GridAction::Move { dx, dy } => {
                let value = *self.actions_taken.read().unwrap() + 1;
                *self.actions_taken.write().unwrap() = value;

                (x + dx, y + dy)
            }
        };

        self.state = if self.terminal.0 == new_x && self.terminal.1 == new_y {
            GridState::Terminal
        } else if self.grid.0.contains(&new_x) && self.grid.1.contains(&new_y) {
            GridState::OnGrid { x: new_x, y: new_y }
        } else {
            GridState::OffGrid { x, y }
        }
    }
}

#[test]
fn can_run_grid_episodes() {
    // TODO check epsilon greedy
    let mut simulator = Simulator::new(Box::new(QLearning::new(0.2, 0.01)), Box::new(Greedy::default()));
    let actions = [
        (GridAction::Move { dx: 1, dy: 0 }, 0.),
        (GridAction::Move { dx: 0, dy: 1 }, 0.),
        (GridAction::Move { dx: -1, dy: 0 }, 0.),
        (GridAction::Move { dx: 0, dy: 1 }, 0.),
    ]
    .iter()
    .cloned()
    .collect::<HashMap<_, _>>();
    let state = GridState::OnGrid { x: 0, y: 0 };
    let grid = (0..4, 0..4);
    let terminal = (3, 3);

    for _ in 0..500 {
        let actions_taken = Arc::new(RwLock::new(0));
        let agent = GridAgent::new(actions.clone(), state.clone(), grid.clone(), terminal, actions_taken.clone());

        simulator.run_episodes(vec![Box::new(agent)]);

        println!("{}", actions_taken.read().unwrap());
    }
}
