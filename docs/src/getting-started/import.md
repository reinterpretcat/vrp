# Defining problem

In general, expressing an arbitrary VRP problem in one simple and universal format is a challenging task. `pragmatic`
format aims to do that and there are [concept](../concepts/pragmatic/index.md) and [example](../examples/pragmatic/index.md)
sections which describe multiple features it supports in great details. However, it might take some time to get a huge
problem with a lot of jobs and vehicles converted into it.

A `csv import` feature might help here.


## CSV import

`vrp-cli` supports importing jobs and vehicles into `pragmatic` json format by the following command:

        vrp-cli import csv -i jobs.csv -i vehicles.csv -o problem.json

As you can see from the command, you need to specify jobs and vehicles in two separate csv files in the exact order.


### Jobs csv

Jobs csv defines a `plan` of the problem and should have the following columns:

* `ID` __(string)__: an id
* `LAT` __(float)__: a latitude
* `LNG` __(float)__: a longitude
* `DEMAND` __(integer)__: a single dimensional demand. Depending on the value, it models different job activities:
    * positive: `pickup`
    * negative: `delivery`
    * zero: `service`
* `DURATION` __(integer)__: job duration in minutes
* `TW_START` __(date in RFC3999)__: earliest time when job can be served
* `TW_END` __(date in RFC3999)__: latest time when job can be served

To model a job with more than one activity (e.g. pickup + delivery), specify same `ID` twice. Example:

```csv
ID,LAT,LNG,DEMAND,DURATION,TW_START,TW_END
job1,52.52599,13.45413,2,5,2020-07-04T08:00:00Z,2020-07-04T12:00:00Z
job2,52.5225,13.4095,1,3,,
job2,52.5165,13.3808,-1,3,,
job3,52.5316,13.3884,3,5,2020-07-04T08:00:00Z,2020-07-04T16:00:00Z
```

job with `job2` id specified twice with positive and negative demand, so it will be considered as pickup and delivery job.


### Vehicles csv

Vehicles csv defines a `fleet` of the problem and should have the following columns:

* `ID` __(string)__: an unique vehicle type id
* `LAT` __(float)__: a depot latitude
* `LNG` __(float)__: a depot longitude
* `CAPACITY` __(unassigned integer)__: a single dimensional vehicle capacity
* `TW_START` __(date in RFC3999)__: earliest time when vehicle can start at depot
* `TW_END` __(date in RFC3999)__: latest time when vehicle should return to depot
* `AMOUNT` __(unassigned integer)__: a vehicle amount of this type
* `PROFILE` __(string)__: a routing profile

This is example of such csv:

```csv
ID,LAT,LNG,CAPACITY,TW_START,TW_END,AMOUNT,PROFILE
vehicle1,52.4664,13.4023,40,2020-07-04T08:00:00Z,2020-07-04T20:00:00Z,10,car
vehicle2,52.4959,13.3539,50,2020-07-04T08:00:00Z,2020-07-04T20:00:00Z,20,truck
```


### Limitations

Please note, to keep csv format simple and easy to use, it's limited to just a few, really basic features known as
_Capacitated Vehicle Routing Problem with Time Windows_ (CVRPTW). However, for a few jobs/vehices, you can modify the
file manually as post-processing step.
