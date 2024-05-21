use crate::plots::*;

mod draw_fitness;
mod draw_population;
mod draw_search;
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
            draw_solution::draw_on_area(&area, &solution_config)?;
        }
        (PopulationSeries::Rosomaxa { .. }, Some(solution_config)) => {
            let (left, right) = area.split_horizontally(50.percent_width());
            draw_solution::draw_on_area(&left, &solution_config)?;
            draw_population::draw_on_area(&right, &population_config)?;
        }
        _ => draw_population::draw_on_area(&area, &population_config)?,
    }

    area.present()?;

    Ok(())
}

pub fn draw_fitness<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    fitness_config: FitnessDrawConfig,
) -> DrawResult<()> {
    draw_fitness::draw_on_area(&area, &fitness_config)
}

pub fn draw_search_iteration<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    heuristic_config: SearchDrawConfig,
) -> DrawResult<()> {
    draw_search::draw_search_iteration(&area, &heuristic_config)
}

pub fn draw_search_best_statistics<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    heuristic_config: SearchDrawConfig,
) -> DrawResult<()> {
    draw_search::draw_search_best_statistics(&area, &heuristic_config)
}

pub fn draw_search_duration_statistics<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    heuristic_config: SearchDrawConfig,
) -> DrawResult<()> {
    draw_search::draw_search_duration_statistics(&area, &heuristic_config)
}

pub fn draw_search_overall_statistics<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    heuristic_config: SearchDrawConfig,
) -> DrawResult<()> {
    draw_search::draw_search_overall_statistics(&area, &heuristic_config)
}
