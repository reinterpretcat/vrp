#[cfg(test)]
#[path = "../../tests/unit/common/text_writer_test.rs"]
mod text_writer_test;

use std::io::{BufWriter, Error, ErrorKind, Write};
use vrp_core::models::Solution;
use vrp_core::models::problem::JobIdDimension;

pub(crate) fn write_text_solution<W: Write>(solution: &Solution, writer: &mut BufWriter<W>) -> Result<(), Error> {
    if !solution.unassigned.is_empty() {
        return Err(Error::new(ErrorKind::Other, "cannot write text solution with unassigned jobs."));
    }

    let cost = solution.cost;

    solution.routes.iter().zip(1..).for_each(|(r, i)| {
        let customers = r
            .tour
            .all_activities()
            .filter(|a| a.job.is_some())
            .map(|a| a.retrieve_job().unwrap())
            .map(|job| job.dimens().get_job_id().unwrap().clone())
            .collect::<Vec<String>>()
            .join(" ");
        writer.write_all(format!("Route {i}: {customers}\n").as_bytes()).unwrap();
    });

    writer.write_all(format!("Cost {cost:.2}").as_bytes())?;

    Ok(())
}
