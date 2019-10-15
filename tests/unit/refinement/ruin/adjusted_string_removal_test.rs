/* Uses predefined values to control algorithm execution.
int distribution values:
1. route index in solution
2*. job index in selected route tour
3*. selected algorithm: 1: sequential algorithm(**)
4*. string removal index(-ies)
double distribution values:
1. string count
2*. string size(-s)
(*) - specific for each route.
(**) - calls more int and double distributions:
    int 5. split start
    dbl 3. alpha param
*/

#[test]
fn can_ruin_solution_with_one_route() {
    // TODO
}
