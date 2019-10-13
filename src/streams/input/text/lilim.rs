#[cfg(test)]
#[path = "../../../../tests/unit/streams/input/text/lilim_test.rs"]
mod lilim_test;

#[path = "./helpers.rs"]
mod helpers;
use self::helpers::*;

use crate::models::common::TimeWindow;
use crate::models::problem::{Fleet, Vehicle, VehicleDetail};
use crate::models::Problem;
use crate::streams::input::text::StringReader;
use crate::utils::{MatrixFactory, TryCollect};
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Read};

pub fn read_lilim_format<R: Read>(mut reader: BufReader<R>) -> Result<Problem, String> {
    LilimReader { buffer: String::new(), reader, matrix: MatrixFactory::new() }.read_problem()
}

pub trait LilimProblem {
    fn parse_lilim(&self) -> Result<Problem, String>;
}

impl LilimProblem for File {
    fn parse_lilim(&self) -> Result<Problem, String> {
        read_lilim_format(BufReader::new(self))
    }
}

impl LilimProblem for String {
    fn parse_lilim(&self) -> Result<Problem, String> {
        read_lilim_format(BufReader::new(StringReader::new(self.as_str())))
    }
}

struct VehicleLine {
    number: usize,
    capacity: usize,
    ignored: usize,
}

struct JobLine {
    id: usize,
    location: (usize, usize),
    demand: usize,
    start: usize,
    end: usize,
    service: usize,
    relation: usize,
}

struct LilimReader<R: Read> {
    buffer: String,
    reader: BufReader<R>,
    matrix: MatrixFactory,
}

impl<R: Read> LilimReader<R> {
    pub fn read_problem(&mut self) -> Result<Problem, String> {
        unimplemented!()
    }

    fn read_fleet(&mut self) -> Result<Fleet, String> {
        let vehicle = self.read_vehicle()?;
        let depot = self.read_customer()?;

        Ok(create_fleet_with_distance_costs(
            vehicle.number,
            vehicle.capacity,
            self.matrix.collect(depot.location),
            TimeWindow { start: depot.start as f64, end: depot.end as f64 },
        ))
    }

    fn read_vehicle(&mut self) -> Result<VehicleLine, String> {
        self.read_line()?;
        let (number, capacity, ignored) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<usize>().unwrap())
            .try_collect()
            .ok_or("Cannot parse vehicle number or/and capacity".to_string())?;

        Ok(VehicleLine { number, capacity, ignored })
    }

    fn read_customer(&mut self) -> Result<JobLine, String> {
        self.read_line()?;
        let (id, x, y, demand, start, end, service, _, relation) = self
            .buffer
            .split_whitespace()
            .map(|line| line.parse::<usize>().unwrap())
            .try_collect()
            .ok_or("Cannot read customer line".to_string())?;
        Ok(JobLine { id, location: (x, y), demand, start, end, service, relation })
    }

    fn read_line(&mut self) -> Result<usize, String> {
        self.buffer.clear();
        self.reader.read_line(&mut self.buffer).map_err(|err| err.to_string())
    }
}
