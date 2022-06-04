use super::*;

#[test]
fn can_solve_scientific_problem() {
    let problem = r#"NAME : SMALL
COMMENT : Test problem
TYPE : CVRP
DIMENSION : 5
EDGE_WEIGHT_TYPE : EUC_2D
CAPACITY : 100
NODE_COORD_SECTION
 1 82 76
 2 96 44
 3 50 5
 4 49 8
 5 13 7
DEMAND_SECTION
1 0
2 19
3 21
4 6
5 19
DEPOT_SECTION
 1
 -1
EOF
"#
    .to_string();
    let logger = Environment::default().logger;

    solve_vrp("tsplib", problem, "rosomaxa", 8, 2000, logger);
}
