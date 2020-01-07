# Problem model 

In general a pragmatic problem is split into two required parts:

* `plan` models a work to be performed by vehicles taking into account all related constraints, such as time windows,
  demand, skills, etc.
* `fleet` models available resources defined by vehicle types.


## Modeling jobs

Plan should contain jobs of two types: simple and multi job type. The main difference is that multi job has multiple
pickups and deliveries with variable demand. You can mix both types in any order inside one plan. 

Check [simple job](./simple-jobs.md) and [multi job](./multi-jobs.md) sections for details.

## Modeling vehicles

Vehicles are defined by `fleet.types` property which specifies array of vehicle types, not specific vehicles.
 
More details can be found in [vehicle type section](./vehicle-types.md).


## Relation between jobs and vehicles

An optional `plan.relations` property specifies relations between multiple jobs and single vehicle. It is useful to
lock jobs to a specific vehicle in any or predefined order.
 
Check [relations section](./relations.md) for more details.


## Job and vehicle constraints

There are multiple strict constraints that should be matched on jobs and vehicles.

### Demand and capacity

Each job should have `demand` property which models a _good_ size in abstract integral units:

{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:26:28}}

It is required, but you can set demand to zero in case it is not needed. It can be multidimensional array.

A `capacity` property is a vehicle characteristic which constraints amount of jobs can be served by vehicle of specific
type based on accumulated demand value. Total demand should not exceed capacity value.

### Time windows

Optionally, each job can have one or more time window:

{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:13:22}}

Time windows are strict: if no vehicle can visit a job in given time ranges, then the job is considered as unassigned. 

Vehicle time is limited per each shift and has required start optional end time:

{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json:85:101}}

More details about `shift` property can be found in [vehicle type section](./vehicle-types.md).


### Skills

An optional `skills` property is a set of unique tags which should be matched on job and vehicle to be used. It is useful
to model some specific job requirements to assigned vehicle (e.g. should have fridge or driver should be a handyman). 