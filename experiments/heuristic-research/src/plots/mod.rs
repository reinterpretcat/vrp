#![allow(clippy::unused_unit)]

use super::*;
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use rosomaxa::prelude::GenericError;
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

    /// Draws plot for rosenbrock function.
    pub fn rosenbrock(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-2.0..2.0, 0.15), y: (0.0..3610.), z: (-2.0..2.0, 0.15) };
        draw_population_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "rosenbrock")
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        Ok(())
    }

    /// Draws plot for rastrigin function.
    pub fn rastrigin(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.12..5.12, 0.2), y: (0.0..80.), z: (-5.12..5.12, 0.2) };
        draw_population_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "rastrigin")
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        Ok(())
    }

    /// Draws plot for himmelblau function.
    pub fn himmelblau(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.0..5.0, 0.2), y: (0.0..700.), z: (-5.0..5.0, 0.2) };
        draw_population_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "himmelblau")
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        Ok(())
    }

    /// Draws plot for ackley function.
    pub fn ackley(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-5.0..5.0, 0.2), y: (0.0..14.), z: (-5.0..5.0, 0.2) };
        draw_population_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "ackley")
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        Ok(())
    }

    /// Draws plot for matyas function.
    pub fn matyas(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let axes = Axes { x: (-10.0..10.0, 0.4), y: (0.0..100.), z: (-10.0..10.0, 0.4) };
        draw_population_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "matyas")
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        Ok(())
    }

    /// Draws plot for VRP problem.
    pub fn vrp(canvas: HtmlCanvasElement, generation: usize, pitch: f64, yaw: f64) -> Result<(), JsValue> {
        let (max_x, max_y, max_z) = get_axis_sizes();
        let axes = Axes { x: (0.0..max_x.max(10.), 0.5), y: (0.0..max_y.max(10.)), z: (0.0..max_z.max(10.), 0.5) };
        draw_population_plots(get_canvas_drawing_area(canvas), generation, pitch, yaw, axes, "vrp")
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
}

/// Draws fitness plot on given area.
pub fn draw_fitness_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    function_name: &str,
) -> Result<(), GenericError> {
    let fitness = get_best_known_fitness();
    let fitness_size = if fitness.is_empty() { return Ok(()) } else { fitness[0].1.len() };

    let (labels, target_idx) = if function_name == "vrp" {
        let labels = if fitness_size == 2 {
            vec!["min-unassigned".to_string(), "min-cost".to_string()]
        } else {
            vec!["min-unassigned".to_string(), "min-tours".to_string(), "min-cost".to_string()]
        };

        (labels, fitness_size - 1)
    } else {
        (vec!["y".to_string()], 0)
    };

    draw_fitness(area, FitnessDrawConfig { labels, fitness, target_idx }).map_err(|err| err.to_string().into())
}

pub fn draw_search_iteration_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
) -> Result<(), GenericError> {
    draw_search_iteration(area, get_search_config(generation, kind)).map_err(|err| err.to_string().into())
}

pub fn draw_search_best_statistics_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
) -> Result<(), GenericError> {
    draw_search_best_statistics(area, get_search_config(generation, kind)).map_err(|err| err.to_string().into())
}

/// Draws population plots on given area.
pub fn draw_population_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    pitch: f64,
    yaw: f64,
    axes: Axes,
    function_name: &str,
) -> Result<(), GenericError> {
    let is_vrp = function_name == "vrp";
    draw_population(
        area,
        PopulationDrawConfig {
            axes: Axes { x: (Default::default(), 0.0), y: Default::default(), z: (Default::default(), 0.0) },
            series: get_population_series(generation),
        },
        if is_vrp && generation != 0 {
            // TODO find a nice way to visualize vrp solutions in 3D plot
            None
        } else {
            Some(SolutionDrawConfig {
                axes,
                projection: Projection { pitch, yaw, scale: 0.8 },
                series: Series3D {
                    surface: {
                        let fitness_fn = {
                            if function_name == "vrp" {
                                Arc::new(|_: &[f64]| 0.)
                            } else {
                                get_fitness_fn_by_name(function_name)
                            }
                        };
                        Box::new(move |x, z| (fitness_fn)(&[x, z]))
                    },
                    points: Box::new(move || get_solution_points(generation)),
                },
            })
        },
    )
    .map_err(|err| err.to_string().into())
}

fn get_canvas_drawing_area(canvas: HtmlCanvasElement) -> DrawingArea<CanvasBackend, Shift> {
    CanvasBackend::with_canvas_object(canvas).unwrap().into_drawing_area()
}

fn get_best_known_fitness() -> Vec<(usize, Vec<f64>)> {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .map(|data| {
            (0..=data.generation)
                .filter_map(|generation| match data.population_state.get(&generation) {
                    Some(PopulationState::Rosomaxa { fitness_values, .. })
                    | Some(PopulationState::Unknown { fitness_values }) => Some((generation, fitness_values.clone())),
                    _ => None,
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

            if let Some((_, points)) = data.on_generation.get(&generation) {
                data_points.extend(to_data_point(points).map(|point| (point.clone(), PointType::Circle, BLACK)));
            }

            if let Some(points) = data.on_add.get(&generation) {
                data_points.extend(to_data_point(points).map(|point| (point.clone(), PointType::Triangle, RED)));
            }

            if let Some(points) = data.on_select.get(&generation) {
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
            let _states_rev = data.heuristic_state.states.iter().map(|(k, v)| (*v, k)).collect::<HashMap<_, _>>();

            // get statistics how often the best solution was found by specific heuristic till current generation
            let statistics = if let Some(best_state_idx) = data.heuristic_state.states.get(kind) {
                (0..generation).fold(vec![0; names_rev.len()], |mut acc, gen| {
                    let gen_idx = gen + 1;

                    data.heuristic_state
                        .search_states
                        .get(&gen_idx)
                        .iter()
                        .flat_map(|states| states.iter())
                        .filter(|SearchResult(_, _, (_, to_state_idx), _)| to_state_idx == best_state_idx)
                        .map(|SearchResult(name_idx, ..)| *name_idx)
                        .for_each(|name_idx| {
                            acc[name_idx] += 1;
                        });

                    acc
                })
            } else {
                vec![0; names_rev.len()]
            }
            .into_iter()
            .enumerate()
            .map(|(name_idx, hits)| (names_rev.get(&name_idx).unwrap().to_string(), hits))
            .collect();

            data.heuristic_state.search_states.get(&generation).map(|states| {
                let estimations = states
                    .iter()
                    // NOTE: just show all transitions
                    //.filter(|SearchResult(_, _, (_, to_idx), _)| states_rev.get(to_idx).unwrap().as_str() == kind)
                    .map(|SearchResult(name_idx, reward, _, _)| (names_rev.get(name_idx).unwrap().to_string(), *reward))
                    .collect::<Vec<_>>();

                SearchDrawConfig { estimations, statistics }
            })
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
        .and_then(|data| match data.population_state.get(&generation) {
            Some(PopulationState::Rosomaxa {
                rows,
                cols,
                mean_distance,
                fitness_values,
                fitness_matrices,
                u_matrix,
                t_matrix,
                l_matrix,
                n_matrix,
            }) => {
                let get_series = |matrix: &MatrixData| {
                    let matrix = matrix.clone();
                    Series2D { matrix_fn: Box::new(move || matrix.clone()) }
                };

                Some(PopulationSeries::Rosomaxa {
                    rows: rows.clone(),
                    cols: cols.clone(),
                    mean_distance: *mean_distance,
                    fitness_values: fitness_values.clone(),
                    fitness_matrices: fitness_matrices.iter().map(get_series).collect(),
                    u_matrix: get_series(u_matrix),
                    t_matrix: get_series(t_matrix),
                    l_matrix: get_series(l_matrix),
                    n_matrix: get_series(n_matrix),
                })
            }
            _ => None,
        })
        .unwrap_or(PopulationSeries::Unknown)
}

fn get_axis_sizes() -> (f64, f64, f64) {
    #[derive(Serialize)]
    struct Axis {
        pub x: f64,
        pub y: f64,
        pub z: f64,
    }

    EXPERIMENT_DATA.lock().unwrap().on_generation.iter().fold((0_f64, 0_f64, 0_f64), |acc, (_, (_, data))| {
        data.iter().fold(acc, |(max_x, max_y, max_z), data| {
            let &DataPoint3D(x, y, z) = match data {
                ObservationData::Function(point) => point,
                ObservationData::Vrp { point, .. } => point,
            };
            (max_x.max(x), max_y.max(y), max_z.max(z))
        })
    })
}
