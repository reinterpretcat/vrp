# Problem model 

In general a pragmatic problem is split into two required and one optional parts:

* `plan` (required) models a work to be performed by vehicles taking into account all related constraints, such as time windows,
  demand, skills, etc.
* `fleet` (required) models available resources defined by vehicle types.
* `objectives` (optional) defines objective functions as goal of whole optimization.


## Modeling jobs

A work which has to be done is model by list of jobs defined in `plan`.

Check next [job](./jobs.md) section for detailed explanation.

## Modeling vehicles

Vehicles are defined by `fleet.vehicles` property which specifies array of vehicle types, not specific vehicles.
 
More details can be found in [vehicle type section](vehicles.md).


## Relation between jobs and vehicles

An optional `plan.relations` property specifies relations between multiple jobs and single vehicle. It is useful to
lock jobs to a specific vehicle in any or predefined order.
 
Check [relations section](./relations.md) for more details.


## Job and vehicle constraints

There are multiple strict constraints that should be matched on jobs and vehicles.

### Demand and capacity

Each job should have `demand` property which models a _good_ size in abstract integral units:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:27:29}}
```

It is required, but you can set demand to zero in case it is not needed. It can be multidimensional array.

A `capacity` property is a vehicle characteristic which constraints amount of jobs can be served by vehicle of specific
type based on accumulated demand value. Total demand should not exceed capacity value.

### Time windows

Optionally, each job can have one or more time window:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:15:24}}
```

Time windows are strict: if no vehicle can visit a job in given time ranges, then the job is considered as unassigned. 

Vehicle time is limited per each shift and has required start optional end time:

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json:109:124}}
```

More details about `shift` property can be found in [vehicle type section](vehicles.md).


### Skills

An optional `skills` property is a set of unique tags which should be matched on job and vehicle to be used. It is useful
to model some specific job requirements to assigned vehicle (e.g. should have fridge or driver should be a handyman).
See [skills example](../../../examples/pragmatic/basics/skills.md).

### Priority

An optional `priority` property allows you to force some jobs being served before other. Priority is represented as integer in range [1, MAX_INT]
where the lower value means higher priority. By default value is set to 1.