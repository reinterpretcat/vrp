#![allow(clippy::unused_unit)]

use super::*;
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use rosomaxa::prelude::{Float, GenericError};
use web_sys::HtmlCanvasElement;

/// Type alias for the result of a drawing function.
pub type DrawResult<T> = Result<T, Box<dyn std::error::Error>>;

mod config;
pub use self::config::*;

mod drawing;
pub use self::drawing::*;

/// Type used on the JS side to convert screen coordinates to chart coordinates.
#[wasm_bindgen]
pub struct Chart {}

#[wasm_bindgen]
impl Chart {
    /// Draws best known fitness progression for benchmark functions.
    pub fn fitness_func(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        draw_fitness_plots(get_canvas_drawing_area(canvas), "func").map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws best known fitness progression for vrp problem.
    pub fn fitness_vrp(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
        draw_fitness_plots(get_canvas_drawing_area(canvas), "vrp").map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws a known two-dimensional benchmark function and the population projected onto its surface.
    pub fn function(
        canvas: HtmlCanvasElement,
        generation: usize,
        pitch: Float,
        yaw: Float,
        function_name: &str,
    ) -> Result<(), JsValue> {
        let axes = get_function_axes(function_name);
        draw_population_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, function_name)
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        Ok(())
    }

    /// Draws the GSOM state for a VRP problem.
    pub fn vrp(canvas: HtmlCanvasElement, generation: usize, pitch: Float, yaw: Float) -> Result<(), JsValue> {
        draw_vrp_population_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw)
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        Ok(())
    }

    /// Draws plot for search estimations.
    pub fn search_iteration(canvas: HtmlCanvasElement, generation: usize, kind: &str) -> Result<(), JsValue> {
        draw_search_iteration_plots(get_canvas_drawing_area(canvas), generation, kind)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws plot for best statistics.
    pub fn search_best_statistics(canvas: HtmlCanvasElement, generation: usize, kind: &str) -> Result<(), JsValue> {
        draw_search_best_statistics_plots(get_canvas_drawing_area(canvas), generation, kind)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws plot for duration statistics.
    pub fn search_duration_statistics(canvas: HtmlCanvasElement, generation: usize, kind: &str) -> Result<(), JsValue> {
        draw_search_duration_statistics_plots(get_canvas_drawing_area(canvas), generation, kind)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws plot for overall statistics.
    pub fn search_overall_statistics(canvas: HtmlCanvasElement, generation: usize, kind: &str) -> Result<(), JsValue> {
        draw_search_overall_statistics_plots(get_canvas_drawing_area(canvas), generation, kind)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws GSOM topology evolution.
    pub fn gsom_statistics(canvas: HtmlCanvasElement, generation: usize) -> Result<(), JsValue> {
        draw_gsom_statistics_plots(get_canvas_drawing_area(canvas), generation)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }
}

fn get_function_axes(function_name: &str) -> Axes {
    const RESOLUTION: usize = 50;

    let config = get_function_config(function_name);
    let fitness_fn = get_fitness_fn_by_name(function_name);
    let x_step = (config.x.end - config.x.start) / RESOLUTION as Float;
    let z_step = (config.z.end - config.z.start) / RESOLUTION as Float;
    let (min, max) = (0..=RESOLUTION)
        .flat_map(|x_idx| {
            let fitness_fn = fitness_fn.clone();
            let config = &config;
            (0..=RESOLUTION).map(move |z_idx| {
                let x = config.x.start + x_step * x_idx as Float;
                let z = config.z.start + z_step * z_idx as Float;
                fitness_fn(&[x, z])
            })
        })
        .chain(config.optima.iter().map(|[_, _, fitness]| *fitness))
        .fold((Float::MAX, Float::MIN), |(min, max), value| (min.min(value), max.max(value)));
    let padding = ((max - min) * 0.05).max(Float::EPSILON);

    Axes { x: (config.x, x_step), y: (min - padding)..(max + padding), z: (config.z, z_step) }
}

/// Draws fitness plot on given area.
pub fn draw_fitness_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    function_name: &str,
) -> Result<(), GenericError> {
    let fitness = get_best_known_fitness();
    let fitness_size = if fitness.is_empty() { return Ok(()) } else { fitness[0].1.len() };

    let (labels, target_idx) = (get_fitness_labels(fitness_size, function_name), fitness_size - 1);

    draw_fitness(area, FitnessDrawConfig { labels, fitness, target_idx }).map_err(From::from)
}

pub fn draw_search_iteration_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
) -> Result<(), GenericError> {
    draw_search_iteration(area, get_search_config(generation, kind)).map_err(From::from)
}

pub fn draw_search_best_statistics_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
) -> Result<(), GenericError> {
    draw_search_best_statistics(area, get_search_config(generation, kind)).map_err(From::from)
}

pub fn draw_search_duration_statistics_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
) -> Result<(), GenericError> {
    draw_search_duration_statistics(area, get_search_config(generation, kind)).map_err(From::from)
}

pub fn draw_search_overall_statistics_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
) -> Result<(), GenericError> {
    draw_search_overall_statistics(area, get_search_config(generation, kind)).map_err(From::from)
}

/// Draws GSOM topology evolution up to the requested generation.
pub fn draw_gsom_statistics_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
) -> Result<(), GenericError> {
    draw_gsom_statistics(area, get_gsom_config(generation)).map_err(From::from)
}

fn get_gsom_config(generation: usize) -> GsomDrawConfig {
    let points = EXPERIMENT_DATA
        .lock()
        .ok()
        .map(|data| {
            data.population_state
                .range(..=generation)
                .filter_map(|(generation, state)| match state {
                    PopulationState::Rosomaxa {
                        rows,
                        cols,
                        mse,
                        learning_rate,
                        fitness_matrices,
                        u_matrix,
                        l_matrix,
                        ..
                    } => {
                        let nodes = u_matrix.len();
                        let occupied_nodes = fitness_matrices.first().map_or(0, MatrixData::len);
                        let active_nodes = l_matrix.values().filter(|hits| **hits > 0.).count();
                        let cells = ((rows.end - rows.start).max(1) as usize)
                            .saturating_mul((cols.end - cols.start).max(1) as usize);

                        Some(GsomStatePoint {
                            generation: *generation,
                            nodes,
                            occupied_nodes,
                            active_nodes,
                            sink_proxies: count_fitness_sinks(fitness_matrices),
                            density: nodes as Float / cells.max(1) as Float,
                            active_ratio: active_nodes as Float / nodes.max(1) as Float,
                            mse: *mse,
                            learning_rate: *learning_rate,
                        })
                    }
                    PopulationState::Unknown { .. } => None,
                })
                .collect()
        })
        .unwrap_or_default();

    GsomDrawConfig { points }
}

pub(crate) fn count_fitness_sinks(fitness_matrices: &[MatrixData]) -> usize {
    let Some(primary) = fitness_matrices.first() else { return 0 };
    let get_fitness = |coordinate: &Coordinate| {
        fitness_matrices.iter().map(|matrix| matrix.get(coordinate).copied()).collect::<Option<Vec<_>>>()
    };
    let compare = |left: &[Float], right: &[Float]| {
        left.iter()
            .zip(right)
            .map(|(left, right)| left.total_cmp(right))
            .find(|order| *order != std::cmp::Ordering::Equal)
            .unwrap_or(std::cmp::Ordering::Equal)
    };

    primary
        .keys()
        .filter(|coordinate| {
            let Coordinate(x, y) = **coordinate;
            let current = get_fitness(coordinate).expect("fitness planes use the same occupied coordinates");
            [Coordinate(x - 1, y), Coordinate(x + 1, y), Coordinate(x, y - 1), Coordinate(x, y + 1)]
                .iter()
                .filter_map(get_fitness)
                .all(|neighbor| compare(&neighbor, &current) != std::cmp::Ordering::Less)
        })
        .count()
}

/// Draws population plots on given area.
pub fn draw_population_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    pitch: Float,
    yaw: Float,
    axes: Axes,
    function_name: &str,
) -> Result<(), GenericError> {
    draw_population(
        area,
        PopulationDrawConfig {
            fitness_labels: get_fitness_labels(get_population_fitness_size(generation), function_name),
            series: get_population_series(generation),
        },
        Some(SolutionDrawConfig {
            caption: String::new(),
            area_ratio: 0.5,
            axes,
            projection: Projection { pitch, yaw, scale: 0.8 },
            series: Series3D {
                surface: Some({
                    let fitness_fn = get_fitness_fn_by_name(function_name);
                    Box::new(move |x, z| (fitness_fn)(&[x, z])) as Box<dyn Fn(Float, Float) -> Float>
                }),
                points: Box::new(move || get_solution_points(generation)),
            },
        }),
    )
    .map_err(From::from)
}

/// Draws the learned GSOM topology for a VRP population.
pub fn draw_vrp_population_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    pitch: Float,
    yaw: Float,
) -> Result<(), GenericError> {
    draw_population(
        area,
        PopulationDrawConfig {
            fitness_labels: get_fitness_labels(get_population_fitness_size(generation), "vrp"),
            series: get_population_series(generation),
        },
        get_vrp_footprint_config(generation, pitch, yaw),
    )
    .map_err(From::from)
}

fn get_vrp_footprint_config(generation: usize, pitch: Float, yaw: Float) -> Option<SolutionDrawConfig> {
    let (snapshot_generation, footprint) = EXPERIMENT_DATA.lock().ok().and_then(|data| {
        data.on_generation
            .range(..=generation)
            .next_back()
            .map(|(generation, (footprint, _))| (*generation, footprint.clone()))
    })?;
    let dimension = footprint.dimension().max(1) as Float;
    let max_value = footprint.max_value().max(1) as Float;
    let edge_count = footprint.edge_count();
    let surface = footprint.clone();

    Some(SolutionDrawConfig {
        caption: format!("edge footprint · gen {snapshot_generation} · {edge_count} edges"),
        area_ratio: 0.28,
        axes: Axes { x: (0.0..dimension, 1.), y: 0.0..(max_value + 1.), z: (0.0..dimension, 1.) },
        projection: Projection { pitch, yaw, scale: 0.75 },
        series: Series3D {
            surface: Some(Box::new(move |from, to| surface.get(from as usize, to as usize) as Float)),
            points: Box::new(Vec::new),
        },
    })
}

fn get_fitness_labels(size: usize, function_name: &str) -> Vec<String> {
    if function_name != "vrp" {
        return (0..size).map(|idx| if idx == 0 { "fitness".to_string() } else { format!("fitness {idx}") }).collect();
    }

    match size {
        2 => vec!["unassigned".to_string(), "cost".to_string()],
        3 => vec!["unassigned".to_string(), "tours".to_string(), "cost".to_string()],
        _ => (0..size).map(|idx| format!("objective {idx}")).collect(),
    }
}

fn get_population_fitness_size(generation: usize) -> usize {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .and_then(|data| {
            data.population_state.range(..=generation).next_back().map(|(_, state)| match state {
                PopulationState::Rosomaxa { fitness_matrices, .. } => fitness_matrices.len(),
                PopulationState::Unknown { fitness_values } => fitness_values.len(),
            })
        })
        .unwrap_or_default()
}

fn get_canvas_drawing_area(canvas: HtmlCanvasElement) -> DrawingArea<CanvasBackend, Shift> {
    CanvasBackend::with_canvas_object(canvas).unwrap().into_drawing_area()
}

fn get_best_known_fitness() -> Vec<(usize, Vec<Float>)> {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .map(|data| {
            data.population_state
                .range(..=data.generation)
                .map(|(generation, state)| match state {
                    PopulationState::Rosomaxa { fitness_values, .. } | PopulationState::Unknown { fitness_values } => {
                        (*generation, fitness_values.clone())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn get_solution_points(generation: usize) -> Vec<ColoredDataPoint3D> {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .map(|data| {
            let mut data_points: Vec<ColoredDataPoint3D> = vec![];

            if let Some((_, (_, points))) = data.on_generation.range(..=generation).next_back() {
                data_points.extend(to_data_point(points).map(|point| (point.clone(), PointType::Circle, BLACK)));
            }

            if let Some((_, points)) = data.on_add.range(..=generation).next_back() {
                data_points.extend(to_data_point(points).map(|point| (point.clone(), PointType::Triangle, RED)));
            }

            if let Some((_, points)) = data.on_select.range(..=generation).next_back() {
                data_points.extend(to_data_point(points).map(|point| (point.clone(), PointType::Triangle, BLUE)));
            }

            data_points
        })
        .unwrap_or_default()
}

fn get_search_config(generation: usize, kind: &str) -> SearchDrawConfig {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .and_then(|data| {
            let names_rev = data.heuristic_state.names.iter().map(|(k, v)| (*v, k)).collect::<HashMap<_, _>>();
            let (&nearest_generation, current) = data.heuristic_state.search_states.range(..=generation).next_back()?;
            let names_size = names_rev.len();
            let best_state = data.heuristic_state.states.get(kind);
            let mut best = vec![0_usize; names_size];
            let mut overall = vec![0_usize; names_size];
            let mut durations = vec![(0_usize, 0_usize); names_size];

            data.heuristic_state.search_states.range(..=nearest_generation).for_each(|(_, states)| {
                states.iter().for_each(|SearchResult(name_idx, _, (_, to_state), duration)| {
                    if let Some(value) = overall.get_mut(*name_idx) {
                        *value += 1;
                    }
                    if best_state.is_some_and(|best_state| best_state == to_state)
                        && let Some(value) = best.get_mut(*name_idx)
                    {
                        *value += 1;
                    }
                    if let Some((total, count)) = durations.get_mut(*name_idx) {
                        *total = total.saturating_add(*duration);
                        *count += 1;
                    }
                });
            });

            let with_names = |values: Vec<usize>| {
                values
                    .into_iter()
                    .enumerate()
                    .filter_map(|(idx, value)| names_rev.get(&idx).map(|name| ((*name).clone(), value)))
                    .collect::<Vec<_>>()
            };
            let estimations = current
                .iter()
                .filter_map(|SearchResult(name_idx, reward, _, _)| {
                    names_rev.get(name_idx).map(|name| ((*name).clone(), *reward))
                })
                .collect();
            let best = if best_state.is_some() { with_names(best) } else { Vec::new() };
            let overall = with_names(overall);
            let durations = durations
                .into_iter()
                .enumerate()
                .filter_map(|(idx, (total, count))| {
                    names_rev.get(&idx).map(|name| ((*name).clone(), total.checked_div(count).unwrap_or_default()))
                })
                .collect();

            Some(SearchDrawConfig { estimations, best, overall, durations })
        })
        .unwrap_or_default()
}

fn to_data_point(observations: &[ObservationData]) -> impl Iterator<Item = &DataPoint3D> + '_ {
    observations.iter().filter_map(|o| match o {
        ObservationData::Function(point) => Some(point),
        _ => None,
    })
}

fn get_population_series(generation: usize) -> PopulationSeries {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .and_then(|data| {
            let current_generation =
                data.population_state.range(..=generation).next_back().map(|(generation, _)| *generation);
            data.population_state.range(..=generation).rev().find_map(|(snapshot_generation, state)| match state {
                PopulationState::Rosomaxa {
                    rows,
                    cols,
                    mse,
                    learning_rate,
                    fitness_matrices,
                    u_matrix,
                    t_matrix,
                    l_matrix,
                    m_matrix,
                    ..
                } => {
                    let get_series = |matrix: &MatrixData| Series2D { matrix: matrix.clone() };

                    Some(PopulationSeries::Rosomaxa {
                        generation: *snapshot_generation,
                        is_stale: current_generation.is_some_and(|generation| generation > *snapshot_generation),
                        rows: rows.clone(),
                        cols: cols.clone(),
                        mse: *mse,
                        learning_rate: *learning_rate,
                        fitness_matrices: fitness_matrices.iter().map(get_series).collect(),
                        u_matrix: get_series(u_matrix),
                        t_matrix: get_series(t_matrix),
                        l_matrix: get_series(l_matrix),
                        m_matrix: get_series(m_matrix),
                    })
                }
                PopulationState::Unknown { .. } => None,
            })
        })
        .unwrap_or(PopulationSeries::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_local_fitness_sinks_lexicographically() {
        let coordinates = [Coordinate(0, 0), Coordinate(1, 0), Coordinate(2, 0)];
        let primary = coordinates.into_iter().zip([1., 1., 2.]).collect();
        let secondary = coordinates.into_iter().zip([2., 1., 0.]).collect();

        assert_eq!(count_fitness_sinks(&[primary, secondary]), 1);
    }

    #[test]
    fn treats_disconnected_occupied_regions_as_separate_sinks() {
        let primary = [(Coordinate(0, 0), 1.), (Coordinate(2, 0), 2.)].into_iter().collect();

        assert_eq!(count_fitness_sinks(&[primary]), 2);
    }
}
