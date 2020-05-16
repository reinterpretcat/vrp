# Introduction

This project is about solving Vehicle Routing Problem which is common task in transportation planning and logistics.

 ## Vehicle Routing Problem
 From wiki:
 > The vehicle routing problem (VRP) is a combinatorial optimization and integer programming problem
 > which asks "What is the optimal set of routes for a fleet of vehicles to traverse in order to
 > deliver to a given set of customers?". It generalises the well-known travelling salesman problem
 > (TSP).
 >
 > Determining the optimal solution to VRP is NP-hard, so the size of problems that can be solved,
 > optimally, using mathematical programming or combinatorial optimization may be limited.
 > Therefore, commercial solvers tend to use heuristics due to the size and frequency of real
 > world VRPs they need to solve.

<div id="geojson" hidden>
{{#include ../../examples/data/pragmatic/objectives/berlin.balance-max-load.solution.geojson}}
</div>

<div id="map"></div>

 ## Design

Although performance is constantly in focus, a main idea behind projects' design is extensibility:
the project aims to support a very wide range of VRP variations known as Rich VRP. This is achieved
through various extension points: custom constraints, objective functions, acceptance criteria, etc.

By default, it provides a metaheuristic which can be roughly described as *Multi-objective Parthenogenesis based
Evolutionary Algorithm with Ruin and Recreate Mutation Operator*.

More details can be found in [concepts chapter](concepts/index.md).
