#![allow(clippy::unused_unit)]

use super::*;
use plotters::coord::Shift;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use rosomaxa::prelude::{Float, GenericError};
use std::collections::{HashSet, VecDeque};
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

    /// Draws state-specific Thompson posterior means.
    pub fn search_iteration(
        canvas: HtmlCanvasElement,
        generation: usize,
        kind: &str,
        window: usize,
    ) -> Result<(), JsValue> {
        let config = get_search_config(generation, kind, window);
        resize_search_canvas(&canvas, config.posterior.len());
        draw_search_iteration(get_canvas_drawing_area(canvas), config)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws state-specific empirical success rates.
    pub fn search_best_statistics(
        canvas: HtmlCanvasElement,
        generation: usize,
        kind: &str,
        window: usize,
    ) -> Result<(), JsValue> {
        let config = get_search_config(generation, kind, window);
        resize_search_canvas(&canvas, config.success_rates.len());
        draw_search_best_statistics(get_canvas_drawing_area(canvas), config)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws state-specific mean durations.
    pub fn search_duration_statistics(
        canvas: HtmlCanvasElement,
        generation: usize,
        kind: &str,
        window: usize,
    ) -> Result<(), JsValue> {
        let config = get_search_config(generation, kind, window);
        resize_search_canvas(&canvas, config.durations.len());
        draw_search_duration_statistics(get_canvas_drawing_area(canvas), config)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws state-specific operator calls.
    pub fn search_overall_statistics(
        canvas: HtmlCanvasElement,
        generation: usize,
        kind: &str,
        window: usize,
    ) -> Result<(), JsValue> {
        let config = get_search_config(generation, kind, window);
        resize_search_canvas(&canvas, config.calls.len());
        draw_search_overall_statistics(get_canvas_drawing_area(canvas), config)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draws GSOM topology evolution.
    pub fn gsom_statistics(canvas: HtmlCanvasElement, generation: usize) -> Result<(), JsValue> {
        draw_gsom_statistics_plots(get_canvas_drawing_area(canvas), generation)
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }
}

fn resize_search_canvas(canvas: &HtmlCanvasElement, operators: usize) {
    const MIN_HEIGHT: u32 = 420;
    const MAX_HEIGHT: u32 = 5000;
    const ROW_HEIGHT: u32 = 25;
    const CHART_PADDING: u32 = 120;

    let height = (operators as u32).saturating_mul(ROW_HEIGHT).saturating_add(CHART_PADDING);
    canvas.set_height(height.clamp(MIN_HEIGHT, MAX_HEIGHT));
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
    window: usize,
) -> Result<(), GenericError> {
    draw_search_iteration(area, get_search_config(generation, kind, window)).map_err(From::from)
}

pub fn draw_search_best_statistics_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
    window: usize,
) -> Result<(), GenericError> {
    draw_search_best_statistics(area, get_search_config(generation, kind, window)).map_err(From::from)
}

pub fn draw_search_duration_statistics_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
    window: usize,
) -> Result<(), GenericError> {
    draw_search_duration_statistics(area, get_search_config(generation, kind, window)).map_err(From::from)
}

pub fn draw_search_overall_statistics_plots<B: DrawingBackend + 'static>(
    area: DrawingArea<B, Shift>,
    generation: usize,
    kind: &str,
    window: usize,
) -> Result<(), GenericError> {
    draw_search_overall_statistics(area, get_search_config(generation, kind, window)).map_err(From::from)
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
    let matrices = fitness_matrices.iter().collect::<Vec<_>>();
    get_fitness_basins(matrices.as_slice()).sinks.len()
}

/// Describes the discrete downhill flow induced by objective values on occupied GSOM nodes.
pub(crate) struct FitnessBasins {
    /// Best cardinal neighbour for each node, or the node itself at a sink.
    pub next: HashMap<Coordinate, Coordinate>,
    /// Sink reached by following `next` from each occupied node.
    pub sink_by_coordinate: HashMap<Coordinate, Coordinate>,
    /// Number of downhill lattice steps from each node to its sink.
    pub depth_by_coordinate: HashMap<Coordinate, usize>,
    /// Unique sinks in stable coordinate order.
    pub sinks: Vec<Coordinate>,
}

/// Partitions occupied nodes into map-local fitness basins using steepest cardinal descent.
pub(crate) fn get_fitness_basins(fitness_matrices: &[&MatrixData]) -> FitnessBasins {
    let Some(primary) = fitness_matrices.first() else {
        return FitnessBasins {
            next: HashMap::new(),
            sink_by_coordinate: HashMap::new(),
            depth_by_coordinate: HashMap::new(),
            sinks: Vec::new(),
        };
    };

    let compare = |left: &Coordinate, right: &Coordinate| {
        fitness_matrices
            .iter()
            .map(|matrix| {
                matrix
                    .get(left)
                    .expect("fitness planes use the same occupied coordinates")
                    .total_cmp(matrix.get(right).expect("fitness planes use the same occupied coordinates"))
            })
            .find(|order| *order != std::cmp::Ordering::Equal)
            // A stable tie break turns an equal-fitness plateau into one acyclic basin.
            .unwrap_or_else(|| left.cmp(right))
    };

    let next = primary
        .keys()
        .map(|coordinate| {
            let Coordinate(x, y) = *coordinate;
            let best =
                [*coordinate, Coordinate(x - 1, y), Coordinate(x + 1, y), Coordinate(x, y - 1), Coordinate(x, y + 1)]
                    .into_iter()
                    .filter(|candidate| primary.contains_key(candidate))
                    .min_by(compare)
                    .expect("the current coordinate is occupied");

            (*coordinate, best)
        })
        .collect::<HashMap<_, _>>();

    let mut sink_by_coordinate = HashMap::with_capacity(next.len());
    let mut depth_by_coordinate = HashMap::with_capacity(next.len());

    for start in next.keys().copied() {
        if sink_by_coordinate.contains_key(&start) {
            continue;
        }

        let mut path = Vec::new();
        let mut current = start;
        loop {
            if let Some(&sink) = sink_by_coordinate.get(&current) {
                let mut depth = depth_by_coordinate[&current];
                for coordinate in path.into_iter().rev() {
                    depth += 1;
                    sink_by_coordinate.insert(coordinate, sink);
                    depth_by_coordinate.insert(coordinate, depth);
                }
                break;
            }

            let next_coordinate = next[&current];
            if next_coordinate == current {
                sink_by_coordinate.insert(current, current);
                depth_by_coordinate.insert(current, 0);

                let mut depth = 0;
                for coordinate in path.into_iter().rev() {
                    depth += 1;
                    sink_by_coordinate.insert(coordinate, current);
                    depth_by_coordinate.insert(coordinate, depth);
                }
                break;
            }

            path.push(current);
            current = next_coordinate;
        }
    }

    let mut sinks = sink_by_coordinate.values().copied().collect::<Vec<_>>();
    sinks.sort_unstable();
    sinks.dedup();

    FitnessBasins { next, sink_by_coordinate, depth_by_coordinate, sinks }
}

/// Describes fitness basins after short-lived map minima are merged into persistent ancestors.
pub(crate) struct PersistentFitnessBasins {
    /// Raw downhill flow, retained to show how the simplified regions were formed.
    pub next: HashMap<Coordinate, Coordinate>,
    /// Persistent sink assigned to each occupied coordinate.
    pub sink_by_coordinate: HashMap<Coordinate, Coordinate>,
    /// Lattice distance from each coordinate to its persistent sink.
    pub depth_by_coordinate: HashMap<Coordinate, usize>,
    /// Minima retained after simplification.
    pub sinks: Vec<Coordinate>,
    /// All minima before simplification.
    pub raw_sinks: Vec<Coordinate>,
}

/// Keeps the most persistent minima in the objective filtration and merges the remaining watershed regions.
pub(crate) fn get_persistent_fitness_basins(
    fitness_matrices: &[&MatrixData],
    keep_size: usize,
) -> PersistentFitnessBasins {
    let raw = get_fitness_basins(fitness_matrices);
    let Some(primary) = fitness_matrices.first() else {
        return PersistentFitnessBasins {
            next: raw.next,
            sink_by_coordinate: HashMap::new(),
            depth_by_coordinate: HashMap::new(),
            sinks: Vec::new(),
            raw_sinks: Vec::new(),
        };
    };

    let compare = |left: &Coordinate, right: &Coordinate| {
        fitness_matrices
            .iter()
            .map(|matrix| {
                matrix
                    .get(left)
                    .expect("fitness planes use the same occupied coordinates")
                    .total_cmp(matrix.get(right).expect("fitness planes use the same occupied coordinates"))
            })
            .find(|order| *order != std::cmp::Ordering::Equal)
            .unwrap_or_else(|| left.cmp(right))
    };

    let mut coordinates = primary.keys().copied().collect::<Vec<_>>();
    coordinates.sort_unstable_by(compare);
    let coordinate_idx =
        coordinates.iter().enumerate().map(|(idx, coordinate)| (*coordinate, idx)).collect::<HashMap<_, _>>();
    let mut parents = (0..coordinates.len()).collect::<Vec<_>>();
    let births = (0..coordinates.len()).collect::<Vec<_>>();
    let mut active = vec![false; coordinates.len()];
    let mut merge_parent = HashMap::new();
    let mut persistence_by_sink = HashMap::new();

    fn find(parents: &mut [usize], mut idx: usize) -> usize {
        let mut root = idx;
        while parents[root] != root {
            root = parents[root];
        }
        while parents[idx] != idx {
            let parent = parents[idx];
            parents[idx] = root;
            idx = parent;
        }
        root
    }

    for (rank, coordinate) in coordinates.iter().copied().enumerate() {
        let idx = coordinate_idx[&coordinate];
        active[idx] = true;
        let Coordinate(x, y) = coordinate;

        for neighbor in [Coordinate(x - 1, y), Coordinate(x + 1, y), Coordinate(x, y - 1), Coordinate(x, y + 1)] {
            let Some(&neighbor_idx) = coordinate_idx.get(&neighbor).filter(|&&neighbor_idx| active[neighbor_idx])
            else {
                continue;
            };
            let left_root = find(&mut parents, idx);
            let right_root = find(&mut parents, neighbor_idx);
            if left_root == right_root {
                continue;
            }

            let (older_root, younger_root) =
                if births[left_root] < births[right_root] { (left_root, right_root) } else { (right_root, left_root) };
            let older_birth = births[older_root];
            let younger_birth = births[younger_root];
            let older_sink = coordinates[older_birth];
            let younger_sink = coordinates[younger_birth];

            parents[younger_root] = older_root;
            merge_parent.insert(younger_sink, older_sink);
            persistence_by_sink.insert(younger_sink, rank.saturating_sub(younger_birth));
        }
    }

    // A minimum in a disconnected component never dies in this filtration and must remain representable.
    let roots = raw.sinks.iter().copied().filter(|sink| !merge_parent.contains_key(sink)).collect::<HashSet<_>>();
    for sink in &roots {
        let birth = coordinate_idx[sink];
        persistence_by_sink.insert(*sink, coordinates.len().saturating_sub(birth));
    }

    let target_size = keep_size.max(roots.len()).min(raw.sinks.len());
    let mut ranked_sinks = raw.sinks.clone();
    ranked_sinks.sort_unstable_by(|left, right| {
        persistence_by_sink[right].cmp(&persistence_by_sink[left]).then_with(|| compare(left, right))
    });
    let mut retained = roots;
    for sink in ranked_sinks {
        if retained.len() == target_size {
            break;
        }
        retained.insert(sink);
    }

    let resolve_sink = |raw_sink: &Coordinate| {
        let mut sink = *raw_sink;
        while !retained.contains(&sink) {
            sink = merge_parent[&sink];
        }
        sink
    };
    let sink_by_coordinate = raw
        .sink_by_coordinate
        .iter()
        .map(|(coordinate, sink)| (*coordinate, resolve_sink(sink)))
        .collect::<HashMap<_, _>>();
    let mut sinks = retained.into_iter().collect::<Vec<_>>();
    sinks.sort_unstable();

    let mut depth_by_coordinate = HashMap::with_capacity(sink_by_coordinate.len());
    let mut queue = VecDeque::new();
    for sink in &sinks {
        depth_by_coordinate.insert(*sink, 0);
        queue.push_back(*sink);
    }
    while let Some(coordinate) = queue.pop_front() {
        let depth = depth_by_coordinate[&coordinate];
        let sink = sink_by_coordinate[&coordinate];
        let Coordinate(x, y) = coordinate;
        for neighbor in [Coordinate(x - 1, y), Coordinate(x + 1, y), Coordinate(x, y - 1), Coordinate(x, y + 1)] {
            if sink_by_coordinate.get(&neighbor).is_some_and(|neighbor_sink| *neighbor_sink == sink)
                && !depth_by_coordinate.contains_key(&neighbor)
            {
                depth_by_coordinate.insert(neighbor, depth + 1);
                queue.push_back(neighbor);
            }
        }
    }
    // A raw watershed remains well-defined even if a merge saddle was assigned to the neighboring component.
    raw.depth_by_coordinate.iter().for_each(|(coordinate, depth)| {
        depth_by_coordinate.entry(*coordinate).or_insert(*depth);
    });

    PersistentFitnessBasins { next: raw.next, sink_by_coordinate, depth_by_coordinate, sinks, raw_sinks: raw.sinks }
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

fn get_search_config(generation: usize, kind: &str, window: usize) -> SearchDrawConfig {
    EXPERIMENT_DATA
        .lock()
        .ok()
        .and_then(|data| create_search_config(&data.heuristic_state, generation, kind, window))
        .unwrap_or_default()
}

fn create_search_config(
    state: &HyperHeuristicState,
    generation: usize,
    kind: &str,
    window: usize,
) -> Option<SearchDrawConfig> {
    let names_rev = state.names.iter().map(|(name, idx)| (*idx, name)).collect::<HashMap<_, _>>();
    let state_idx = *state.states.get(kind)?;
    let (&current_generation, current) = state.heuristic_states.range(..=generation).next_back()?;
    let baseline = if window == 0 || current_generation <= window {
        None
    } else {
        state.heuristic_states.range(..=current_generation.saturating_sub(window)).next_back()
    };
    let baseline_generation = baseline.map(|(generation, _)| *generation).unwrap_or_default();
    let baseline = baseline
        .map(|(_, values)| {
            values.iter().filter(|value| value.0 == state_idx).map(|value| (value.1, value)).collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let state_label = if kind == "best" { "Best-known parents" } else { "Diverse parents" };
    let interval_label = if window == 0 {
        format!("{state_label} · cumulative through generation {current_generation}")
    } else if baseline.is_empty() {
        format!("{state_label} · start through generation {current_generation}")
    } else {
        format!("{state_label} · exact checkpoint delta {baseline_generation}→{current_generation}")
    };

    let mut config = SearchDrawConfig {
        posterior_caption: format!("{state_label} · posterior at generation {current_generation}"),
        interval_caption: interval_label,
        unavailable: "Exact success and duration counters are unavailable in this older saved state.".to_string(),
        ..Default::default()
    };

    current.iter().filter(|value| value.0 == state_idx).for_each(|value| {
        let HeuristicResult(
            _,
            name_idx,
            _,
            _,
            mean,
            _,
            calls,
            successes,
            duration,
            progress_mean,
            promotion_mean,
            parent_improvements,
        ) = value;
        let Some(name) = names_rev.get(name_idx) else { return };
        let previous = baseline.get(name_idx).copied();
        let calls = calls.saturating_sub(previous.map(|value| value.6).unwrap_or_default());
        let successes = successes.and_then(|successes| {
            let previous = match previous {
                Some(HeuristicResult(_, _, _, _, _, _, _, Some(successes), ..)) => *successes,
                Some(_) => return None,
                None => 0,
            };
            Some(successes.saturating_sub(previous))
        });
        let duration = duration.and_then(|duration| {
            let previous = match previous {
                Some(HeuristicResult(_, _, _, _, _, _, _, _, Some(duration), ..)) => *duration,
                Some(_) => return None,
                None => 0,
            };
            Some(duration.saturating_sub(previous))
        });

        let posterior_label = match (kind, progress_mean, promotion_mean) {
            ("diverse", Some(progress), Some(promotion)) => {
                format!("{name} · {mean:.3} ({progress:.3}×{promotion:.3})")
            }
            _ => format!("{name} · {mean:.3}"),
        };
        config.posterior.push((posterior_label, *mean));
        config.calls.push((format!("{name} · {calls}"), calls as Float));

        if let Some(successes) = successes {
            let rate = if calls == 0 { 0. } else { successes as Float / calls as Float };
            let parent_improvements = parent_improvements.and_then(|parent_improvements| {
                let previous = match previous {
                    Some(HeuristicResult(_, _, _, _, _, _, _, _, _, _, _, Some(parent_improvements))) => {
                        *parent_improvements
                    }
                    Some(_) => return None,
                    None => 0,
                };
                Some(parent_improvements.saturating_sub(previous))
            });
            let label = match (kind, parent_improvements) {
                ("diverse", Some(parent_improvements)) => {
                    format!("{name} · {successes}/{calls}; parent {parent_improvements}/{calls}")
                }
                _ => format!("{name} · {successes}/{calls}"),
            };
            config.success_rates.push((label, rate));
        }
        if let Some(duration) = duration.filter(|_| calls > 0) {
            let mean = duration as Float / calls as Float;
            config.durations.push(((*name).clone(), mean));
        }
    });

    Some(config)
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

    #[test]
    fn partitions_fitness_flow_into_basins() {
        let coordinates = (0..6).map(|x| Coordinate(x, 0)).collect::<Vec<_>>();
        let primary = coordinates.iter().copied().zip([3., 2., 1., 3., 0., 2.]).collect();
        let basins = get_fitness_basins(&[&primary]);

        assert_eq!(basins.sinks, vec![Coordinate(2, 0), Coordinate(4, 0)]);
        assert_eq!(basins.sink_by_coordinate[&Coordinate(0, 0)], Coordinate(2, 0));
        assert_eq!(basins.sink_by_coordinate[&Coordinate(3, 0)], Coordinate(4, 0));
        assert_eq!(basins.depth_by_coordinate[&Coordinate(0, 0)], 2);
    }

    #[test]
    fn turns_equal_fitness_plateau_into_one_acyclic_basin() {
        let primary = (0..3).map(|x| (Coordinate(x, 0), 1.)).collect();
        let basins = get_fitness_basins(&[&primary]);

        assert_eq!(basins.sinks, vec![Coordinate(0, 0)]);
        assert_eq!(basins.next[&Coordinate(2, 0)], Coordinate(1, 0));
        assert_eq!(basins.depth_by_coordinate[&Coordinate(2, 0)], 2);
    }

    #[test]
    fn merges_short_lived_minima_before_persistent_basins() {
        let primary = (0..5).map(|x| Coordinate(x, 0)).zip([0., 3., 2., 3., 1.]).collect();
        let basins = get_persistent_fitness_basins(&[&primary], 2);

        assert_eq!(basins.raw_sinks, vec![Coordinate(0, 0), Coordinate(2, 0), Coordinate(4, 0)]);
        assert_eq!(basins.sinks, vec![Coordinate(0, 0), Coordinate(4, 0)]);
        assert_eq!(basins.sink_by_coordinate[&Coordinate(2, 0)], Coordinate(0, 0));
        assert_eq!(basins.sink_by_coordinate[&Coordinate(4, 0)], Coordinate(4, 0));
    }

    #[test]
    fn keeps_minimum_of_each_disconnected_component() {
        let primary = [(Coordinate(0, 0), 0.), (Coordinate(2, 0), 1.)].into_iter().collect();
        let basins = get_persistent_fitness_basins(&[&primary], 1);

        assert_eq!(basins.sinks, vec![Coordinate(0, 0), Coordinate(2, 0)]);
    }

    #[test]
    fn builds_exact_state_specific_operator_window() {
        let mut state = HyperHeuristicState {
            names: [("first".to_string(), 0), ("second".to_string(), 1)].into_iter().collect(),
            states: [("best".to_string(), 0), ("diverse".to_string(), 1)].into_iter().collect(),
            ..Default::default()
        };
        let result = |state, name, mean, calls, successes, duration| {
            HeuristicResult(state, name, 1., 1., mean, 0.1, calls, Some(successes), Some(duration), None, None, None)
        };
        state.heuristic_states.insert(
            1000,
            vec![result(0, 0, 0.5, 2, 1, 20), result(0, 1, 0.5, 0, 0, 0), result(1, 0, 0.9, 100, 90, 100)],
        );
        state.heuristic_states.insert(
            2000,
            vec![result(0, 0, 0.3, 10, 3, 100), result(0, 1, 0.7, 4, 2, 200), result(1, 0, 0.8, 150, 120, 200)],
        );

        let config = create_search_config(&state, 2200, "best", 1000).unwrap();

        assert_eq!(config.calls, vec![("first · 8".to_string(), 8.), ("second · 4".to_string(), 4.)]);
        assert_eq!(config.posterior, vec![("first · 0.300".to_string(), 0.3), ("second · 0.700".to_string(), 0.7)]);
        assert_eq!(config.success_rates, vec![("first · 2/8".to_string(), 0.25), ("second · 2/4".to_string(), 0.5)]);
        assert_eq!(config.durations, vec![("first".to_string(), 10.), ("second".to_string(), 50.)]);
        assert!(config.interval_caption.ends_with("1000→2000"));
    }

    #[test]
    fn keeps_exact_calls_but_marks_legacy_rates_unavailable() {
        let state = HyperHeuristicState {
            names: [("operator".to_string(), 0)].into_iter().collect(),
            states: [("best".to_string(), 0)].into_iter().collect(),
            heuristic_states: [(10, vec![HeuristicResult(0, 0, 1., 1., 0.5, 0.1, 7, None, None, None, None, None)])]
                .into_iter()
                .collect(),
            ..Default::default()
        };

        let config = create_search_config(&state, 10, "best", 0).unwrap();

        assert_eq!(config.calls, vec![("operator · 7".to_string(), 7.)]);
        assert!(config.success_rates.is_empty());
        assert!(config.durations.is_empty());
    }

    #[test]
    fn shows_hierarchical_diverse_outcomes() {
        let state = HyperHeuristicState {
            names: [("operator".to_string(), 0)].into_iter().collect(),
            states: [("diverse".to_string(), 0)].into_iter().collect(),
            heuristic_states: [(
                10,
                vec![HeuristicResult(0, 0, 4., 6., 0.12, 0.01, 20, Some(2), Some(140), Some(0.4), Some(0.3), Some(8))],
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let config = create_search_config(&state, 10, "diverse", 0).unwrap();

        assert_eq!(config.posterior, vec![("operator · 0.120 (0.400×0.300)".to_string(), 0.12)]);
        assert_eq!(config.success_rates, vec![("operator · 2/20; parent 8/20".to_string(), 0.1)]);
    }
}
