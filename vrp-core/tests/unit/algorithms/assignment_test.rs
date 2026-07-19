use super::*;

fn total(cost: &[Vec<f64>], assign: &[usize]) -> f64 {
    assign.iter().enumerate().map(|(i, &j)| cost[i][j]).sum()
}

fn is_permutation(assign: &[usize]) -> bool {
    let mut seen = vec![false; assign.len()];
    for &j in assign {
        if j >= assign.len() || seen[j] {
            return false;
        }
        seen[j] = true;
    }
    true
}

#[test]
fn empty_matrix_yields_empty_assignment() {
    assert!(min_cost_assignment(&[]).is_empty());
}

#[test]
fn two_by_two_picks_the_diagonal() {
    // Cheapest is row0->col0 (1) + row1->col1 (1) = 2.
    assert_eq!(min_cost_assignment(&[vec![1.0, 2.0], vec![2.0, 1.0]]), vec![0, 1]);
    // Cheapest is row0->col1 (1) + row1->col0 (1) = 2.
    assert_eq!(min_cost_assignment(&[vec![2.0, 1.0], vec![1.0, 2.0]]), vec![1, 0]);
}

#[test]
fn three_by_three_finds_the_unique_optimum() {
    // Unique optimum: row0->col1 (2), row1->col2 (3), row2->col0 (4) = 9.
    let cost = vec![vec![9.0, 2.0, 7.0], vec![6.0, 4.0, 3.0], vec![4.0, 8.0, 9.0]];
    let assign = min_cost_assignment(&cost);
    assert_eq!(assign, vec![1, 2, 0]);
    assert_eq!(total(&cost, &assign), 9.0);
}

#[test]
fn degenerate_all_equal_returns_some_valid_permutation() {
    let cost = vec![vec![5.0; 3]; 3];
    let assign = min_cost_assignment(&cost);
    assert!(is_permutation(&assign));
    assert_eq!(total(&cost, &assign), 15.0);
}
