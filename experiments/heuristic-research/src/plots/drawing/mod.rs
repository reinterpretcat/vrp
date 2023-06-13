use crate::plots::*;

mod draw_population;
use self::draw_population::*;

mod draw_solution;
use self::draw_solution::*;

/// Draws chart on canvas according to the drawing configs.
pub fn draw<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    population_config: PopulationDrawConfig,
    solution_config: Option<SolutionDrawConfig>,
) -> DrawResult<()> {
    area.fill(&WHITE)?;

    match (&population_config.series, solution_config) {
        (PopulationSeries::Unknown, Some(solution_config)) => {
            draw_solution(&area, &solution_config)?;
        }
        (PopulationSeries::Rosomaxa { .. }, Some(solution_config)) => {
            let (left, right) = area.split_horizontally(50.percent_width());
            draw_solution(&left, &solution_config)?;
            draw_population(&right, &population_config)?;
        }
        _ => draw_population(&area, &population_config)?,
    }

    area.present()?;

    Ok(())
}
