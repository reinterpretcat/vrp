use super::*;
use std::fs::File;

const PRAGMATIC_PROBLEM_PATH: &str = "../examples/data/pragmatic/simple.basic.problem.json";
const PRAGMATIC_MATRIX_PATH: &str = "../examples/data/pragmatic/simple.basic.matrix.json";
const PRAGMATIC_SOLUTION_PATH: &str = "../examples/data/pragmatic/simple.basic.solution.json";

fn reader(path: &str) -> BufReader<File> {
    BufReader::new(File::open(path).expect("cannot open test file"))
}

#[test]
pub fn can_detect_invalid_problem_file() {
    assert_eq!(
        check_pragmatic_solution(reader(PRAGMATIC_MATRIX_PATH),
                                 reader(PRAGMATIC_SOLUTION_PATH), None)
            .expect_err("no error returned"),
        vec!["cannot read problem: 'E0000, cause: 'cannot deserialize problem', action: 'check input json: 'missing field `plan` at line 39 column 1''.'".into()]
    );
}

#[test]
pub fn can_detect_invalid_solution_file() {
    assert_eq!(
        check_pragmatic_solution(reader(PRAGMATIC_PROBLEM_PATH), reader(PRAGMATIC_MATRIX_PATH), None)
            .expect_err("no error returned"),
        vec!["cannot read solution: 'missing field `statistic` at line 39 column 1'".into()]
    );
}

#[test]
pub fn can_detect_invalid_matrix_file() {
    assert_eq!(
        check_pragmatic_solution(reader(PRAGMATIC_PROBLEM_PATH),
                                 reader(PRAGMATIC_SOLUTION_PATH),
                                Some(vec![reader(PRAGMATIC_SOLUTION_PATH)]))
            .expect_err("no error returned"),
        vec!["cannot read matrix: 'E0001, cause: 'cannot deserialize matrix', action: 'check input json: 'missing field `travelTimes` at line 159 column 1''.'".into()]
    );
}
