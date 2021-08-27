use super::*;

const PRAGMATIC_PROBLEM_PATH: &str = "../examples/data/pragmatic/simple.basic.problem.json";

struct DummyWrite {}

impl Write for DummyWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn can_run_analyze_clusters() {
    let tmpfile = tempfile::NamedTempFile::new().unwrap();
    let args = vec![
        "analyze",
        "clusters",
        "pragmatic",
        PRAGMATIC_PROBLEM_PATH,
        "--out-result",
        tmpfile.path().to_str().unwrap(),
    ];
    let matches = get_analyze_app().get_matches_from_safe(args).unwrap();

    run_analyze(&matches, |_| BufWriter::new(Box::new(DummyWrite {}))).unwrap();
}
