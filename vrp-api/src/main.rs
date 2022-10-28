//! An api to interface with *Vehicle Routing Problem* solver.
use std::sync::Arc;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use actix_web::{ post, web, App, HttpResponse, HttpServer, Responder, middleware, Error, error };
use serde::{Serialize, Deserialize};
use futures::StreamExt;
use vrp_core::prelude::Solver;
use vrp_pragmatic::checker::CheckerContext;
use vrp_pragmatic::format::FormatError;
use vrp_pragmatic::format::problem::{ Matrix, Problem, PragmaticProblem };
use vrp_pragmatic::core::models::{Problem as CoreProblem, Solution as CoreSolution};
use vrp_pragmatic::format::solution::{ deserialize_solution, Solution, PragmaticSolution };
use vrp_cli::extensions::solve::config::{ read_config, create_builder_from_config };

const MAX_SIZE: usize = 262_144;
const MAX_ITERATIONS: usize = 100;
const BASE_CONFIG_PATH: &str = "./vrp-api/config/config.telemetry.json";

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Welcome to VRP-api!")
}

#[derive(Serialize, Deserialize)]
struct SolverRequest {
    uuid: String,
    problem: Problem,
    matrices: Option<Vec<Matrix>>
}

#[derive(Serialize, Deserialize)]
struct SolverResponse {
    solution: Solution
}

#[inline]
fn get_pragmatic_solution(problem: &CoreProblem, solution: &CoreSolution, cost: f64) -> Solution {
    let mut buffer = String::new();
    let writer = unsafe { BufWriter::new(buffer.as_mut_vec()) };

    (solution, cost).write_pragmatic_json(problem, writer).expect("cannot write pragmatic solution");

    deserialize_solution(BufReader::new(buffer.as_bytes())).expect("cannot deserialize solution")
}

#[inline]
fn solve_problem(name: String, problem: Problem, matrices: Option<Vec<Matrix>>) -> Solution {

    let (core_problem, problem, matrices) = if let Some(matrices) = matrices {
            let matrices = matrices;
            ((problem.clone(), matrices.clone()).read_pragmatic(), problem, Some(matrices))
        } else {
            (problem.clone().read_pragmatic(), problem, None)
        };


    let core_problem = Arc::new(core_problem.unwrap_or_else(|errors| {
        panic!("cannot read pragmatic problem: {}", FormatError::format_many(errors.as_slice(), "\t\n"))
    }));

    let reader = BufReader::new(File::open(BASE_CONFIG_PATH).unwrap_or_else(|err| panic!("Cannot open file on path {}, error: {}", BASE_CONFIG_PATH, err)));
    
    // config
    let mut config = read_config(reader).unwrap_or_else(|err| panic!("Cannot read config from path {}, error: {}", BASE_CONFIG_PATH, err));
    if let Some(initial) = config.evolution.as_mut().and_then(|evolution| evolution.initial.as_mut()) {
        initial.alternatives.max_size = 1;
    }
    if let Some(termination) = config.termination.as_mut() {
        termination.max_generations = Some(1);
    }

    let (solution, cost, _metrics) = create_builder_from_config(core_problem.clone(), Default::default(), &config)
                                                .unwrap_or_else( |err | panic!("cannot build from config {}", err))
                                                .with_max_generations(Some(MAX_ITERATIONS))
                                                .build()
                                                .map(|config| Solver::new(core_problem.clone(), config))
                                                .unwrap_or_else( |err | panic!("cannot build from solver {}", err))
                                                .solve()
                                                .unwrap_or_else( |err | panic!("cannot build from problem {}", err));

    let solution = get_pragmatic_solution(&core_problem, &solution, cost);

    if let Err(err) = CheckerContext::new(core_problem, problem, matrices, solution.clone()).and_then(|ctx| ctx.check()) {
            panic!("unfeasible solution in '{}':\n'{}'", name, err.join("\n"));
    };

    return solution.clone();

}


#[post("/api/v1/solve")]
async fn solve_handler(mut payload: web::Payload) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    // body is loaded, now we can deserialize serde-json
    let obj = serde_json::from_slice::<SolverRequest>(&body)?;
    let solution = solve_problem(obj.uuid, obj.problem, obj.matrices);
    Ok(HttpResponse::Ok().json(solution)) // <- send response
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
        .wrap(middleware::Logger::default())
        .service(solve_handler)
        .route("/", web::get().to(hello))
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
}