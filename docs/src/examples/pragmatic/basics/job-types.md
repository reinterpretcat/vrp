# Basic job usage

This section demonstrates how to use different job task types. 

You can find more details about job type schema on [job concept section](../../../concepts/pragmatic/problem/jobs.md).


## Basic pickup and delivery usage

In this example, there is one delivery, one pickup, and one pickup and delivery job with one dimensional demand.

<details>
    <summary>Problem</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.problem.json}}
```

</p></details>

<details>
    <summary>Solution</summary><p>

```json
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.json}}
```

</p></details>

</br>

<div id="geojson" hidden>
{{#include ../../../../../examples/json-pragmatic/data/simple.basic.solution.geojson}}
</div>

<div id="map"></div>

As problem has two job task places with exactly same location, solution contains one stop with two activities.
 
 
## Multiple pickups and deliveries
 
This example contains two multi jobs with slightly different parameters.
 
 <details>
     <summary>Problem</summary><p>
 
 ```json
 {{#include ../../../../../examples/json-pragmatic/data/basics/multi-job.basic.problem.json}}
 ```
 
 </p></details>
 
 <details>
     <summary>Solution</summary><p>
 
 ```json
 {{#include ../../../../../examples/json-pragmatic/data/basics/multi-job.basic.solution.json}}
 ```
 
 </p></details>
  
 
 ## Mixing job task types
 
You can mix job task types in one job:
 
 <details>
     <summary>Problem</summary><p>
 
 ```json
 {{#include ../../../../../examples/json-pragmatic/data/basics/multi-job.mixed.problem.json}}
 ```
 
 </p></details>
 
 <details>
     <summary>Solution</summary><p>
 
 ```json
 {{#include ../../../../../examples/json-pragmatic/data/basics/multi-job.mixed.solution.json}}
 ```
 
 </p></details>
 