#[cfg(test)]
#[path = "../../tests/unit/tsplib/reader_test.rs"]
mod reader_test;

use crate::common::*;
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::sync::Arc;
use vrp_core::models::common::*;
use vrp_core::models::problem::*;
use vrp_core::models::*;

/// A trait to read tsplib95 problem. Please note that it is very basic implementation of the format specification.
pub trait TsplibProblem {
    /// Reads tsplib95 problem.
    fn read_tsplib(self, is_rounded: bool) -> Result<Problem, String>;
}

impl<R: Read> TsplibProblem for BufReader<R> {
    fn read_tsplib(self, is_rounded: bool) -> Result<Problem, String> {
        TsplibReader::new(self).read_problem(is_rounded)
    }
}

impl TsplibProblem for String {
    fn read_tsplib(self, is_rounded: bool) -> Result<Problem, String> {
        TsplibReader::new(BufReader::new(self.as_bytes())).read_problem(is_rounded)
    }
}

struct TsplibReader<R: Read> {
    buffer: String,
    reader: BufReader<R>,
    dimension: Option<usize>,
    vehicle_capacity: Option<usize>,
    coord_index: CoordIndex,
}

impl<R: Read> TextReader for TsplibReader<R> {
    fn create_goal_context(
        &self,
        activity: Arc<SimpleActivityCost>,
        transport: Arc<dyn TransportCost + Send + Sync>,
    ) -> Result<GoalContext, String> {
        create_goal_context_distance_only(activity, transport)
    }

    fn read_definitions(&mut self) -> Result<(Vec<Job>, Fleet), String> {
        self.read_meta()?;

        let (coordinates, demands) = self.read_customer_data()?;
        let depot_id = self.read_depot_data()?;
        self.read_expected_line("EOF")?;

        let dimension = self.dimension.unwrap();

        let jobs = coordinates.iter().filter(|(id, _)| **id != depot_id).try_fold::<_, _, Result<_, String>>(
            Vec::with_capacity(dimension),
            |mut jobs, (id, (x, y))| {
                let demand = demands.get(id).cloned().ok_or_else(|| format!("cannot find demand for id: '{id}'"))?;

                jobs.push(self.create_job(&(*id - 1).to_string(), (*x, *y), demand));

                Ok(jobs)
            },
        )?;

        let depot_coord =
            *coordinates.get(&depot_id).ok_or_else(|| format!("cannot find coordinate for depot id: '{depot_id}'"))?;

        let fleet = create_fleet_with_distance_costs(
            dimension,
            self.vehicle_capacity.unwrap(),
            self.coord_index.collect(depot_coord),
            TimeWindow::max(),
        );

        Ok((jobs, fleet))
    }

    fn create_transport(&self, is_rounded: bool) -> Result<Arc<dyn TransportCost + Send + Sync>, String> {
        self.coord_index.create_transport(is_rounded)
    }

    fn create_extras(&self) -> Extras {
        get_extras(self.coord_index.clone())
    }
}

type ProblemData = (HashMap<i32, (i32, i32)>, HashMap<i32, i32>);

impl<R: Read> TsplibReader<R> {
    fn new(reader: BufReader<R>) -> Self {
        Self {
            buffer: String::new(),
            reader,
            dimension: None,
            vehicle_capacity: None,
            coord_index: CoordIndex::default(),
        }
    }

    fn read_meta(&mut self) -> Result<(), String> {
        self.skip_lines(2)?;

        let problem_type = self.read_key_value("TYPE")?;
        if problem_type != "CVRP" {
            return Err(format!("expecting 'CVRP' as TYPE, got '{problem_type}'"));
        }

        self.dimension = Some(
            self.read_key_value("DIMENSION")
                .and_then(|dimen| parse_int(&dimen, "cannot parse DIMENSION").map(|v| v as usize))?,
        );

        let edge_type = self.read_key_value("EDGE_WEIGHT_TYPE")?;
        if edge_type != "EUC_2D" {
            return Err(format!("expecting 'EUC_2D' as EDGE_WEIGHT_TYPE, got '{edge_type}'"));
        }

        self.vehicle_capacity = Some(
            self.read_key_value("CAPACITY")
                .and_then(|capacity| parse_int(&capacity, "cannot parse CAPACITY").map(|v| v as usize))?,
        );

        Ok(())
    }

    fn read_customer_data(&mut self) -> Result<ProblemData, String> {
        let dimension = self.dimension.unwrap();

        // read coordinates
        self.read_expected_line("NODE_COORD_SECTION")?;

        let mut coordinates = HashMap::with_capacity(self.dimension.unwrap());
        for _ in 0..dimension {
            let line = self.read_line()?.trim();
            let data = line.split_whitespace().collect::<Vec<_>>();

            if data.len() != 3 {
                return Err(format!("unexpected coord data: '{line}'"));
            }

            let coord = (parse_int(data[1], "cannot parse coord.0")?, parse_int(data[2], "cannot parse coord.1")?);

            coordinates.insert(parse_int(data[0], "cannot parse id")?, coord);
        }

        // read demand
        self.read_expected_line("DEMAND_SECTION")?;

        let mut demands = HashMap::with_capacity(self.dimension.unwrap());
        for _ in 0..dimension {
            let line = self.read_line()?.trim();
            let data = line.split_whitespace().collect::<Vec<_>>();

            if data.len() != 2 {
                return Err(format!("unexpected demand data: '{line}'"));
            }

            demands.insert(parse_int(data[0], "cannot parse id")?, parse_int(data[1], "cannot parse demand")?);
        }

        Ok((coordinates, demands))
    }

    fn read_depot_data(&mut self) -> Result<i32, String> {
        self.read_expected_line("DEPOT_SECTION")?;
        let depot_id = parse_int(self.read_line()?.trim(), "cannot parse depot id")?;
        self.read_expected_line("-1")?;

        Ok(depot_id)
    }

    fn read_key_value(&mut self, expected_key: &str) -> Result<String, String> {
        let line = self.read_line()?;
        let key_value = line.split(':').map(|v| v.to_string()).collect::<Vec<_>>();

        if key_value.len() != 2 {
            return Err(format!("expected colon separated string, got: '{line}'"));
        }

        let actual_key = key_value[0].trim();
        if actual_key.trim() != expected_key {
            return Err(format!("unexpected key, expecting: '{expected_key}', got: '{actual_key}'"));
        }

        Ok(key_value[1].trim().to_string())
    }

    fn read_expected_line(&mut self, expected: &str) -> Result<(), String> {
        let line = self.read_line()?.trim();
        if line != expected {
            Err(format!("expecting {expected}, got: '{line}'"))
        } else {
            Ok(())
        }
    }

    fn read_line(&mut self) -> Result<&String, String> {
        read_line(&mut self.reader, &mut self.buffer)?;
        Ok(&self.buffer)
    }

    fn skip_lines(&mut self, count: usize) -> Result<(), String> {
        skip_lines(count, &mut self.reader, &mut self.buffer)
    }

    fn create_job(&mut self, id: &str, location: (i32, i32), demand: i32) -> Job {
        let mut dimens = create_dimens_with_id("", id);
        dimens.set_demand(Demand::<SingleDimLoad> {
            pickup: (SingleDimLoad::default(), SingleDimLoad::default()),
            delivery: (SingleDimLoad::new(demand), SingleDimLoad::default()),
        });
        Job::Single(Arc::new(Single {
            places: vec![Place {
                location: Some(self.coord_index.collect(location)),
                duration: 0.,
                times: vec![TimeSpan::Window(TimeWindow::max())],
            }],
            dimens,
        }))
    }
}

fn parse_int(data: &str, err_msg: &str) -> Result<i32, String> {
    data.parse::<f64>()
        // NOTE observed that some input files might have coordinates like 28.00000
        .map(|value| value.round() as i32)
        .map_err(|err| format!("{err_msg}: '{err}'"))
}
