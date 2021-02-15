use super::*;

const PRAGMATIC_PROBLEM_PATH: &str = "../examples/data/pragmatic/simple.basic.problem.json";
const SOLOMON_PROBLEM_PATH: &str = "../examples/data/scientific/solomon/C101.25.txt";
const LILIM_PROBLEM_PATH: &str = "../examples/data/scientific/lilim/LC101.txt";

struct DummyWrite {}

impl Write for DummyWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run_solve_with_out_writer(matches: &ArgMatches) {
    run_solve(matches, |_| BufWriter::new(Box::new(DummyWrite {})));
}

#[test]
fn can_solve_pragmatic_problem_with_generation_limit() {
    let args = vec!["solve", "pragmatic", PRAGMATIC_PROBLEM_PATH, "--max-generations", "10"];
    let matches = get_solve_app().get_matches_from_safe(args).unwrap();

    run_solve_with_out_writer(&matches);
}

#[test]
fn can_solve_solomon_problem_with_generation_limit() {
    let args = vec!["solve", "solomon", SOLOMON_PROBLEM_PATH, "--max-generations", "10"];
    let matches = get_solve_app().get_matches_from_safe(args).unwrap();

    run_solve_with_out_writer(&matches);
}

#[test]
fn can_solve_lilim_problem_with_time_limit() {
    let args = vec!["solve", "lilim", LILIM_PROBLEM_PATH, "--max-time", "10"];
    let matches = get_solve_app().get_matches_from_safe(args).unwrap();

    run_solve_with_out_writer(&matches);
}

#[test]
fn can_require_problem_path() {
    for format in &["pragmatic", "solomon", "lilim"] {
        get_solve_app().get_matches_from_safe(vec!["solve", format]).unwrap_err();
    }
}

#[test]
fn can_specify_search_mode_setting() {
    for mode in &["deep", "broad"] {
        let args = vec!["solve", "pragmatic", PRAGMATIC_PROBLEM_PATH, "--search-mode", mode];
        get_solve_app().get_matches_from_safe(args).unwrap();
    }
}

#[test]
fn can_specify_heuristic_setting() {
    for (mode, result) in
        vec![("default", Some(())), ("dynamic", Some(())), ("static", Some(())), ("ggg", None), ("multi", None)]
    {
        let args = vec!["solve", "pragmatic", PRAGMATIC_PROBLEM_PATH, "--heuristic", mode];
        assert_eq!(get_solve_app().get_matches_from_safe(args).ok().map(|_| ()), result);
    }
}
