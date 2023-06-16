use crate::plots::*;

mod draw_fitness;
mod draw_population;
mod draw_solution;

/// Draws chart on canvas according to the drawing configs.
pub fn draw_population<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    population_config: PopulationDrawConfig,
    solution_config: Option<SolutionDrawConfig>,
) -> DrawResult<()> {
    area.fill(&WHITE)?;

    match (&population_config.series, solution_config) {
        (PopulationSeries::Unknown, Some(solution_config)) => {
            self::draw_solution::draw_solution(&area, &solution_config)?;
        }
        (PopulationSeries::Rosomaxa { .. }, Some(solution_config)) => {
            let (left, right) = area.split_horizontally(50.percent_width());
            self::draw_solution::draw_solution(&left, &solution_config)?;
            self::draw_population::draw_population(&right, &population_config)?;
        }
        _ => self::draw_population::draw_population(&area, &population_config)?,
    }

    area.present()?;

    Ok(())
}

pub fn draw_fitness<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    fitness_config: FitnessDrawConfig,
) -> DrawResult<()> {
    self::draw_fitness::draw_fitness(&area, &fitness_config)
}
