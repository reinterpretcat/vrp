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
    run_solve(matches, |_| BufWriter::new(Box::new(DummyWrite {}))).unwrap();
}

fn get_solomon_matches(params: &[&str]) -> ArgMatches {
    let args = [&["solve", "solomon", SOLOMON_PROBLEM_PATH], params].concat();

    get_solve_app().try_get_matches_from(args).unwrap()
}

#[test]
fn can_solve_pragmatic_problem_with_generation_limit() {
    let args = vec!["solve", "pragmatic", PRAGMATIC_PROBLEM_PATH, "--max-generations", "1"];
    let matches = get_solve_app().try_get_matches_from(args).unwrap();

    run_solve_with_out_writer(&matches);
}

#[test]
fn can_solve_lilim_problem_with_multiple_limits() {
    let args = vec!["solve", "lilim", LILIM_PROBLEM_PATH, "--max-time", "300", "--max-generations", "1"];
    let matches = get_solve_app().try_get_matches_from(args).unwrap();

    run_solve_with_out_writer(&matches);
}

#[test]
fn can_solve_solomon_problem_with_generation_limit() {
    run_solve_with_out_writer(&get_solomon_matches(&["--max-generations", "1"]));
}

#[test]
fn can_require_problem_path() {
    for format in &["pragmatic", "solomon", "lilim", "tsplib"] {
        get_solve_app().try_get_matches_from(vec!["solve", format]).unwrap_err();
    }
}

#[test]
fn can_specify_search_mode_setting() {
    for mode in &["deep", "broad"] {
        let args = vec!["solve", "pragmatic", PRAGMATIC_PROBLEM_PATH, "--search-mode", mode];
        get_solve_app().try_get_matches_from(args).unwrap();
    }
}

#[test]
fn can_specify_experimental_setting() {
    let args = vec!["solve", "pragmatic", PRAGMATIC_PROBLEM_PATH, "--experimental"];
    get_solve_app().try_get_matches_from(args).unwrap();
}

#[test]
fn can_specify_round_setting() {
    let args = vec!["solve", "solomon", SOLOMON_PROBLEM_PATH, "--round"];
    get_solve_app().try_get_matches_from(args).unwrap();
}

#[test]
fn can_specify_heuristic_setting() {
    for &(mode, result) in
        &[("default", Some(())), ("dynamic", Some(())), ("static", Some(())), ("ggg", None), ("multi", None)]
    {
        let args = vec!["solve", "pragmatic", PRAGMATIC_PROBLEM_PATH, "--heuristic", mode];
        assert_eq!(get_solve_app().try_get_matches_from(args).ok().map(|_| ()), result);
    }
}

#[test]
fn can_specify_parallelism() {
    for (params, result) in vec![
        (vec!["--parallelism", "3,1"], Ok(3_usize)),
        (vec!["--parallelism", "3"], Err("cannot parse parallelism parameter".to_string())),
    ] {
        let matches = get_solomon_matches(params.as_slice());

        let thread_pool_size = get_environment(&matches, None).map(|e| e.parallelism.thread_pool_size());

        assert_eq!(thread_pool_size, result);
    }
}

#[test]
fn can_use_init_size() {
    for (params, result) in vec![
        (vec!["--init-size", "1"], Ok(Some(1))),
        (vec!["--init-size", "0"], Err("init size must be an integer bigger than 0, got '0'".to_string())),
        (vec![], Ok(None)),
    ] {
        let matches = get_solomon_matches(params.as_slice());

        let init_size = get_init_size(&matches);

        assert_eq!(init_size, result);
    }
}

#[test]
fn can_specify_cv() {
    for (params, result) in vec![
        (vec!["--min-cv", "sample,200,0.05,true"], Ok(Some(("sample".to_string(), 200, 0.05, true)))),
        (vec!["--min-cv", "period,100,0.01,false"], Ok(Some(("period".to_string(), 100, 0.01, false)))),
        (vec!["--min-cv", "sample,200,0,tru"], Err("cannot parse min_cv parameter".to_string())),
        (vec!["--min-cv", "sampl,200,0,true"], Err("cannot parse min_cv parameter".to_string())),
        (vec!["--min-cv", "perio,200,0,true"], Err("cannot parse min_cv parameter".to_string())),
        (vec!["--min-cv", "200,0"], Err("cannot parse min_cv parameter".to_string())),
        (vec!["--min-cv", "0"], Err("cannot parse min_cv parameter".to_string())),
        (vec![], Ok(None)),
    ] {
        let matches = get_solomon_matches(params.as_slice());

        let min_cv = get_min_cv(&matches);

        assert_eq!(min_cv, result);
    }
}
