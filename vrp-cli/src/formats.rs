use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::sync::Arc;
use vrp_core::models::{Problem, Solution};
use vrp_pragmatic::json::problem::PragmaticProblem;
use vrp_pragmatic::json::solution::PragmaticSolution;
use vrp_scientific::common::read_init_solution;
use vrp_scientific::lilim::{LilimProblem, LilimSolution};
use vrp_scientific::solomon::{SolomonProblem, SolomonSolution};

pub struct ProblemReader(pub Box<dyn Fn(File, Option<Vec<File>>) -> Result<Problem, String>>);

pub struct InitSolutionReader(pub Box<dyn Fn(File, Arc<Problem>) -> Option<Solution>>);

pub struct SolutionWriter(
    pub  Box<
        dyn Fn(&Problem, Solution, BufWriter<Box<dyn Write>>, Option<BufWriter<Box<dyn Write>>>) -> Result<(), String>,
    >,
);

pub struct LocationWriter(pub Box<dyn Fn(File, BufWriter<Box<dyn Write>>) -> Result<(), String>>);

pub fn get_formats<'a>() -> HashMap<&'a str, (ProblemReader, InitSolutionReader, SolutionWriter, LocationWriter)> {
    vec![
        (
            "solomon",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    problem.read_solomon()
                })),
                InitSolutionReader(Box::new(|file, problem| read_init_solution(BufReader::new(file), problem).ok())),
                SolutionWriter(Box::new(|_, solution, writer, _| solution.write_solomon(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        ),
        (
            "lilim",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    problem.read_lilim()
                })),
                InitSolutionReader(Box::new(|_file, _problem| None)),
                SolutionWriter(Box::new(|_, solution, writer, _| solution.write_lilim(writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        ),
        (
            "pragmatic",
            (
                ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_some());
                    (problem, matrices.unwrap()).read_pragmatic()
                })),
                InitSolutionReader(Box::new(|_file, _problem| None)),
                SolutionWriter(Box::new(|problem, solution, default_writer, geojson_writer| {
                    geojson_writer
                        .map_or(Ok(()), |geojson_writer| solution.write_geo_json(problem, geojson_writer))
                        .and_then(|_| solution.write_pragmatic_json(problem, default_writer))
                })),
                LocationWriter(Box::new(|problem, writer| {
                    let mut writer = writer;
                    vrp_pragmatic::get_locations(BufReader::new(problem))
                        .and_then(|locations| writer.write_all(locations.as_bytes()).map_err(|err| err.to_string()))
                })),
            ),
        ),
    ]
    .into_iter()
    .collect()
}
