# Evaluating performance

This section is mostly intended for developers and researchers who is interested to understand the solver's performance.

## A generate command

A `generate` command is designed to simplify the process of generating realistic problems in `pragmatic` format.
It has the following parameters:

- **type** (required): a format of the problem. So far, only `pragmatic` is supported
- **prototypes** (required): a list of files with problem prototype definition. At the moment, it has to be path to
     problem in pragmatic format. The prototype problem should contain at least three prototype jobs, which are used
     with equal probability to generate other jobs in plan. Other properties like `objectives`, `fleet` are copied as is
- **output** (required): a path where to store generated problem
- **jobs size** (required): amount of jobs to be generated in the plan.
- **area size** (optional): half size of the bounding box's side (in meters). The center is identified from bounding box
    of prototype jobs which is used also when the parameter is omitted.

Using `generate` command, you can quickly generate different VRP variants. Usage example:

        vrp-cli generate pragmatic -p prototype.json -o generated.json -j 100 -a 10000

This command generates a new problem definition with 100 jobs spread uniformly in bounding box with half side 10000 meters.


## A check solution command

A `check` command is intended to prove feasibility of calculated solution. Both, the problem definition and calculated
solution, are required:

        vrp-cli check pragmatic -p problem.json -s solution.json



## Algorithm fine tuning

TODO describe tweaking metaheuristic parameters


## Algorithm performance

TODO describe usage of `jq` command to extract data in tabular format

TODO describe usage of spreadsheet (python script?) to create a plot with fitness values (routes, cost, balancing)

TODO more..