use super::*;
use crate::Environment;

#[derive(Clone, Hash, Eq, PartialEq)]
struct TestAction {
    marker: usize,
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct TestState {
    state: usize,
}

impl State for TestState {
    type Action = TestAction;

    fn reward(&self) -> f64 {
        self.state as f64
    }
}

#[test]
fn can_select_action_with_weights() {
    let random = Environment::default().random;
    let mut estimates = ActionEstimates::<TestState>::default();
    estimates.insert(TestAction { marker: 0 }, 10.);
    estimates.insert(TestAction { marker: 1 }, 2.);
    estimates.insert(TestAction { marker: 2 }, 1.);
    estimates.insert(TestAction { marker: 3 }, 0.1);
    estimates.insert(TestAction { marker: 4 }, 0.);
    estimates.insert(TestAction { marker: 5 }, -0.1);
    estimates.recalculate_min_max();

    assert_eq!(estimates.max_estimate().unwrap().1, 10.);
    assert_eq!(estimates.min_estimate().unwrap().1, -0.1);

    let frequencies = (0..10000).fold(HashMap::new(), |mut acc, _| {
        let result = estimates.weighted(random.as_ref()).unwrap();
        acc.entry(result.marker).and_modify(|e| *e += 1).or_insert(1);

        acc
    });

    let mut frequencies = frequencies.into_iter().collect::<Vec<_>>();
    frequencies.sort_by(|(a, _), (b, _)| a.cmp(b));

    let result = frequencies.windows(2).fold(true, |acc, window| {
        acc && match window {
            &[(_, a), (_, b)] => a > b,
            _ => unreachable!(),
        }
    });
    assert!(result);
}
