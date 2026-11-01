# Basic job usage

This section demonstrates how to use different job task types. 

You can find more details about job type schema on [job concept section](../../../concepts/pragmatic/problem/jobs.md).


## Basic pickup and delivery usage

In this example, there is one delivery, one pickup, and one pickup and delivery job with one dimensional demand.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/data/pragmatic/simple.basic.solution.json}}
```

</p></details>

</br>

<div id="geojson" hidden>
{{#include ../../../../../examples/data/pragmatic/simple.basic.solution.geojson}}
</div>

<div id="map"></div>

As problem has two job task places with exactly same location, solution contains one stop with two activities.
 
 
## Multiple pickups and deliveries
 
This example contains two multi jobs with slightly different parameters.
 
 <details>
     <summary>Problem</summary><p>
 
 ```json
 {{#include ../../../../../examples/data/pragmatic/basics/multi-job.basic.problem.json}}
 ```
 
 </p></details>
 
 <details>
     <summary>Solution</summary><p>
 
 ```json
 {{#include ../../../../../examples/data/pragmatic/basics/multi-job.basic.solution.json}}
 ```
 
 </p></details>
  
 
 ## Fixed order of tasks

By default, the solver chooses the order of a job's pickups among themselves and of its deliveries among themselves.
In this example, a student travels to school with a companion, who has to be picked up before the student. The
companion's pickup is listed first and `fixedOrder` is set, so the tasks are served exactly in the order they are
listed. Without it, the solver would pick up the student first, as that route is shorter.

 <details>
     <summary>Problem</summary><p>

 ```json
 {{#include ../../../../../examples/data/pragmatic/basics/multi-job.fixed-order.problem.json}}
 ```

 </p></details>

 <details>
     <summary>Solution</summary><p>

 ```json
 {{#include ../../../../../examples/data/pragmatic/basics/multi-job.fixed-order.solution.json}}
 ```

 </p></details>


 ## Mixing job task types
 
You can mix job task types in one job:
 
 <details>
     <summary>Problem</summary><p>
 
 ```json
 {{#include ../../../../../examples/data/pragmatic/basics/multi-job.mixed.problem.json}}
 ```
 
 </p></details>
 
 <details>
     <summary>Solution</summary><p>
 
 ```json
 {{#include ../../../../../examples/data/pragmatic/basics/multi-job.mixed.solution.json}}
 ```
 
 </p></details>
 