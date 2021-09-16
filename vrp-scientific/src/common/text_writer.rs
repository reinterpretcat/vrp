#[cfg(test)]
#[path = "../../tests/unit/common/text_writer_test.rs"]
mod text_writer_test;

use std::io::{BufWriter, Error, ErrorKind, Write};
use vrp_core::models::common::IdDimension;
use vrp_core::models::Solution;

pub(crate) fn write_text_solution<W: Write>(writer: BufWriter<W>, solution: &Solution, cost: f64) -> Result<(), Error> {
    let mut writer = writer;

    if !solution.unassigned.is_empty() {
        return Err(Error::new(ErrorKind::Other, "cannot write text solution with unassigned jobs."));
    }

    solution.routes.iter().zip(1..).for_each(|(r, i)| {
        let customers = r
            .tour
            .all_activities()
            .filter(|a| a.job.is_some())
            .map(|a| a.retrieve_job().unwrap())
            .map(|job| job.dimens().get_id().unwrap().clone())
            .collect::<Vec<String>>()
            .join(" ");
        writer.write_all(format!("Route {}: {}\n", i, customers).as_bytes()).unwrap();
    });

    writer.write_all(format!("Cost {:.2}", cost).as_bytes())?;

    Ok(())
}
