use super::*;

#[derive(PartialEq, Eq, Hash, Clone)]
enum GridState {
    Cell { x: i32, y: i32 },
    Outside,
    Terminal,
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum GridAction {
    Move { dx: i32, dy: i32 },
}

struct GridAgent {
    state: GridState,
}

impl State for GridState {
    type Action = GridAction;

    fn actions(&self) -> Option<ActionsEstimate<Self>> {
        None
    }

    fn reward(&self) -> f64 {
        match &self {
            GridState::Cell { .. } => -1.,
            GridState::Outside => -10.,
            GridState::Terminal => 10.,
        }
    }
}

#[test]
fn can_run_grid_episodes() {
    // TODO
    let _agent = GridAgent { state: GridState::Cell { x: 0, y: 0 } };
}
