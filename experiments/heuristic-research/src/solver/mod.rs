use rosomaxa::example::*;
use rosomaxa::get_default_population;

mod objectives;
pub use self::objectives::*;

mod proxies;
pub use self::proxies::*;

/// Runs the solver to minimize objective function with given name.
pub fn run_solver(objective_name: &str, selection_size: usize) {
    let _solver = Solver::default()
        .with_objective_fun(get_objective_function_by_name(objective_name))
        .with_context_factory(Box::new(move |objective, environment| {
            let inner =
                get_default_population::<VectorContext, _, _>(objective.clone(), environment.clone(), selection_size);
            let population = Box::new(ProxyPopulation::new(inner));
            VectorContext::new(objective, population, environment)
        }))
        .solve();
}
