//! Contains format readers and writers.

use crate::get_locations_serialized;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::sync::Arc;
use vrp_core::models::{Problem, Solution};
use vrp_core::prelude::{GenericError, Random};
use vrp_pragmatic::format::solution::{write_pragmatic, PragmaticOutputType};
use vrp_scientific::tsplib::{TsplibProblem, TsplibSolution};

/// A reader for problem.
#[allow(clippy::type_complexity)]
pub struct ProblemReader(pub Box<dyn Fn(File, Option<Vec<File>>) -> Result<Problem, GenericError>>);

/// A reader for initial solution.
pub struct InitSolutionReader(pub Box<dyn Fn(File, Arc<Problem>) -> Result<Solution, GenericError>>);

#[allow(clippy::type_complexity)]
/// A writer for solution.
pub struct SolutionWriter(
    pub  Box<
        dyn Fn(
            &Problem,
            Solution,
            BufWriter<Box<dyn Write>>,
            Option<BufWriter<Box<dyn Write>>>,
        ) -> Result<(), GenericError>,
    >,
);

/// A writer for locations.
#[allow(clippy::type_complexity)]
pub struct LocationWriter(pub Box<dyn Fn(File, BufWriter<Box<dyn Write>>) -> Result<(), GenericError>>);

#[allow(clippy::type_complexity)]
type FormatMap<'a> = HashMap<&'a str, (ProblemReader, InitSolutionReader, SolutionWriter, LocationWriter)>;

/// Gets available format readers/writers.
pub fn get_formats<'a>(is_rounded: bool, random: Arc<dyn Random>) -> FormatMap<'a> {
    let mut formats = FormatMap::default();

    add_scientific(&mut formats, is_rounded, random.clone());
    add_pragmatic(&mut formats, random);

    formats
}

fn add_scientific(formats: &mut FormatMap, is_rounded: bool, random: Arc<dyn Random>) {
    if cfg!(feature = "scientific-format") {
        use vrp_scientific::common::read_init_solution;
        use vrp_scientific::lilim::{LilimProblem, LilimSolution};
        use vrp_scientific::solomon::{SolomonProblem, SolomonSolution};

        formats.insert(
            "solomon",
            (
                ProblemReader(Box::new(move |problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_solomon(is_rounded)
                })),
                InitSolutionReader(Box::new({
                    let random = random.clone();
                    move |file, problem| read_init_solution(BufReader::new(file), problem, random.clone())
                })),
                SolutionWriter(Box::new(|_, solution, mut writer, _| solution.write_solomon(&mut writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        );
        formats.insert(
            "lilim",
            (
                ProblemReader(Box::new(move |problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_lilim(is_rounded)
                })),
                InitSolutionReader(Box::new(|_file, _problem| unimplemented!())),
                SolutionWriter(Box::new(|_, solution, mut writer, _| solution.write_lilim(&mut writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        );
        formats.insert(
            "tsplib",
            (
                ProblemReader(Box::new(move |problem: File, matrices: Option<Vec<File>>| {
                    assert!(matrices.is_none());
                    BufReader::new(problem).read_tsplib(is_rounded)
                })),
                InitSolutionReader(Box::new(move |file, problem| {
                    read_init_solution(BufReader::new(file), problem, random.clone())
                })),
                SolutionWriter(Box::new(|_, solution, mut writer, _| solution.write_tsplib(&mut writer))),
                LocationWriter(Box::new(|_, _| unimplemented!())),
            ),
        );
    }
}

fn add_pragmatic(formats: &mut FormatMap, random: Arc<dyn Random>) {
    use vrp_pragmatic::format::problem::{deserialize_problem, PragmaticProblem};
    use vrp_pragmatic::format::solution::read_init_solution as read_init_pragmatic;

    formats.insert(
        "pragmatic",
        (
            ProblemReader(Box::new(|problem: File, matrices: Option<Vec<File>>| {
                if let Some(matrices) = matrices {
                    let matrices = matrices.into_iter().map(BufReader::new).collect();
                    (BufReader::new(problem), matrices).read_pragmatic()
                } else {
                    BufReader::new(problem).read_pragmatic()
                }
                .map_err(From::from)
            })),
            InitSolutionReader(Box::new(move |file, problem| {
                read_init_pragmatic(BufReader::new(file), problem, random.clone())
            })),
            SolutionWriter(Box::new(|problem, solution, mut default_writer, geojson_writer| {
                geojson_writer
                    .map_or(Ok(()), |mut geojson_writer| {
                        write_pragmatic(problem, &solution, PragmaticOutputType::OnlyGeoJson, &mut geojson_writer)
                    })
                    .and_then(|_| write_pragmatic(problem, &solution, Default::default(), &mut default_writer))
            })),
            LocationWriter(Box::new(|problem, writer| {
                let mut writer = writer;
                deserialize_problem(BufReader::new(problem))
                    .map_err(From::from)
                    .and_then(|problem| get_locations_serialized(&problem))
                    .and_then(|locations| writer.write_all(locations.as_bytes()).map_err(From::from))
            })),
        ),
    );
}
