use crate::helpers::models::domain::create_empty_insertion_context;
use crate::helpers::solver::create_default_refinement_ctx;
use crate::models::examples::create_example_problem;
use crate::solver::selection::{NaiveSelection, Selection};
use crate::solver::{DominancePopulation, Population, RefinementContext};

#[test]
fn can_select_individuals() {
    let problem = create_example_problem();
    let mut population = DominancePopulation::new(problem.clone(), 4);
    population.add_all(vec![
        create_empty_insertion_context(),
        create_empty_insertion_context(),
        create_empty_insertion_context(),
        create_empty_insertion_context(),
    ]);
    let refinement_ctx =
        RefinementContext { population: Box::new(population), ..create_default_refinement_ctx(problem.clone()) };

    let parents = NaiveSelection::new(3).select_parents(&refinement_ctx);

    assert_eq!(parents.len(), 3);
}
