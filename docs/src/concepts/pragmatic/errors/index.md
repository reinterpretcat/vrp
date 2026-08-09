# Error Index

This page lists errors produced by the solver.


## E0xxx Error

Errors from E0xxx range are generic.


### E0000

`cannot deserialize problem` is returned when problem definition cannot be deserialized from the input stream.


### E0001

`cannot deserialize matrix` is returned when routing matrix definition cannot be deserialized from the input stream.


### E0002

`cannot create transport costs` is returned when problem cannot be matched within routing matrix data passed.

There are two options to consider, when specifying routing matrix data:

- *time dependent VRP* requires all matrices to have `profile` and `timestamp` properties to be se
- *time agnostic VRP* requires `timestamp` property to be omitted, `profile` property either set or skipped for all matrices


### E0003

`cannot find any solution` is returned when no solution is found. In this case, please submit a bug and share original
problem and routing matrix.


### E0004

`cannot read config` is returned when algorithm configuration cannot be created. To fix it, make sure that config has
a valid json schema and valid parameters.


## E1xxx: Validation errors

Errors from E1xxx range are used by validation engine which checks logical correctness of the rich VRP definition.


### E11xx: Jobs

These errors are related to `plan.jobs` property definition.


#### E1100

`duplicated job ids` error is returned when `plan.jobs` has jobs with the same ids:

```json
{
  "plan": {
    "jobs": [
      {
        "id": "job1",
        /** omitted **/
      },
      {
        /** Error: this id is already used by another job **/
        "id": "job1",
        /** omitted **/
      }
      /** omitted **/
    ]
  }
}
```

Duplicated job ids are not allowed, so you need to remove all duplicates in order to fix the error.


#### E1101

`invalid job task demand` error is returned when job has invalid demand: `pickup`, `delivery`, `replacement` job types should
have demand specified on each job task, `service` type should have no demand specified:

```json
{
  "id": "job1",
  "deliveries": [
    {
      /** omitted **/
      /** Error: delivery task should have demand set**/
      "demand": null
    }
 ],
 "services": [
   {
     /** omitted **/
     /** Error: service task should have no demand specified**/
     "demand": [1]
   }
 ]
}
```

To fix the error, make sure that each job task has proper demand.


#### E1102

`invalid pickup and delivery demand` error code is returned when job has both pickups and deliveries, but the sum of
pickups demand does not match to the sum of deliveries demand:

```json
{
  "id": "job",
  "pickups": [
    {
      "places": [/** omitted **/],
      "demand": [1],
    },
    {
      "places": [/** omitted **/],
      "demand": [1]
    }
  ],
  "deliveries": [
    {
      "places": [/** omitted **/],
      /** Error: should be 2 as the sum of pickups is 2 **/
      "demand": [1]
    }
  ]
}
```


#### E1103

`invalid time windows in jobs` error is returned when there is a job which has invalid time windows, e.g.:

```json
{
  /** Error: end time is one hour earlier than start time**/
  "times": [
    [
      "2020-07-04T12:00:00Z",
      "2020-07-04T11:00:00Z"
    ]
  ]
}
```

Each time window must satisfy the following criteria:

* array of two strings each of these specifies date in RFC3339 format. The first is considered as start,
the second - as end
* start date is earlier than end date
* if multiple time windows are specified, they must not intersect, e.g.:

```json
{
  /** Error: second time window intersects with first one: [13:00, 14:00] **/
  "times": [
    [
      "2020-07-04T10:00:00Z",
      "2020-07-04T14:00:00Z"
    ],
    [
      "2020-07-04T13:00:00Z",
      "2020-07-04T17:00:00Z"
    ]
  ]
}
```


#### E1104

`reserved job id is used` error is returned when there is a job which has reserved job id:

```json
{
  /** Error: 'departure' is reserved job id **/
  "id": "departure"
}
```

To avoid confusion, the following ids are reserved: `departure`, `arrival`, `break`, and `reload`. These
ids are not allowed to be used within `job.id` property.


#### E1105

`empty job` error is returned when there is a job which has no or empty job tasks:

```json
{
  /** Error: at least one job task has to be defined **/
  "id": "job1",
  "pickups": null,
  "deliveries": []
}
```

To fix the error, remove job from the plan or add at least one job task to it.


#### E1106

`job has negative duration` error is returned when there is a job place with negative duration:

```json
{
  "id": "job",
  "pickups": [
    {
      "places": [{
        /** Error: negative duration does not make sense **/
        "duration": -10,
        "location": {/* omitted */}
       }]
       /* omitted */
    }
  ]
}
```

To fix the error, make sure that all durations are non negative.


#### E1107

`job has negative demand` error is returned when there is a job with negative demand in any of dimensions:

```json
{
  "id": "job",
  "pickups": [
    {
      "places": [/* omitted */],
      /** Error: negative demand is not allowed **/
      "demand": [10, -1]
    }
  ]
}
```

To fix the error, make sure that all demand values are non negative.


### E12xx: Relations

These errors are related to `plan.relations` property definition.


#### E1200

`relation has job id which does not present in the plan` error is returned when `plan.relations` has relations with
job ids, not present in `plan.jobs`.


#### E1201

`relation has vehicle id which does not present in the fleet` error is returned when `plan.relations` has relations with
vehicle ids, not present in `plan.fleet`.


#### E1202

`relation has empty job id list` error is returned when `plan.relations` has relations with empty `jobs` list or it has
only reserved ids such as `departure`, `arrival`, `break`, `reload`.


#### E1203

`strict or sequence relation has job with multiple places or time windows` error is returned when `plan.relations` has
strict or sequence relation which refers one or many jobs with multiple places and/or time windows.

This is currently not allowed due to matching problem.


#### E1204

`job is assigned to different vehicles in relations` error is returned when `plan.relations` has a job assigned to several
relations with different vehicle ids:

```json
{
  "plan": {
    "relations": [
      {
        "vehicleId": "vehicle_1",
        "jobs": ["job1"],
        /** omitted **/
      },
      {
        /** Error: this job id is already assigned to another vehicle **/
        "vehicleId": "vehicle_2",
        "jobs": ["job1"],
        /** omitted **/
      }
    ]
  }
}
```

To fix this, remove job id from one of relations.


#### E1205

`relation has invalid shift index` error is returned when `plan.relations` has `shiftIndex` value and no corresponding
`shift` is present in list of shifts.


#### E1206

`relation has special job id which is not defined on vehicle shift` error is returned when `plan.relations` has reserved
job id and corresponding property on `fleet.vehicles.shifts` is not defined. Reserved ids are `break`, `reload` and `arrival`.


#### E1207

`some relations have incomplete job definitions` error is returned when `plan.relations` has relation with incomplete
job definitions: e.g. job has two pickups, but in relation its job id is specified only once. To fix the issue, either
remove job ids completely or add missing ones.


### E13xx: Vehicles

These errors are related to `fleet.vehicles` property definition.


#### E1300

`duplicated vehicle type ids` error is returned when `fleet.vehicles` has vehicle types with the same `typeId`:

```json
{
  "fleet": {
    "vehicles": [
      {
        "typeId": "vehicle_1",
        /** omitted **/
      },
      {
        /** Error: this id is already used by another vehicle type **/
        "typeId": "vehicle_1",
        /** omitted **/
      }
      /** omitted **/
    ]
  }
}
```


#### E1301

`duplicated vehicle ids` error is returned when `fleet.vehicles` has vehicle types with the same `vehicleIds`:

```json
{
  "fleet": {
    "vehicles": [
      {
        "typeId": "vehicle_1",
        "vehicleIds": [
          "vehicle_1_a",
          "vehicle_1_b",
          /** Error: vehicle_1_b is used second time **/
          "vehicle_1_b"
        ],
        /** omitted **/
      },
      {
        "typeId": "vehicle_2",
        "vehicleIds": [
          /** Error: vehicle_1_a is used second time **/
          "vehicle_1_a",
          "vehicle_2_b"
        ],
        /** omitted **/
      }
      /** omitted **/
    ]
  }
}
```

Please note that vehicle id should be unique across all vehicle types.


#### E1302

`invalid start or end times in vehicle shift` error is returned when vehicle has start/end shift times violating one of
time windows rules defined for jobs in E1103.


#### E1303

`invalid break time windows in vehicle shift` error is returned when vehicle has invalid time window of a break. List of
break should follow time window rules defined for jobs in E1103. Additionally, break time should be inside vehicle shift
it is specified:

```json
{
  "start": {
    "time": "2019-07-04T08:00:00Z",
    /** omitted **/
  },
  "end": {
    "time": "2019-07-04T15:00:00Z",
    /** omitted **/
  },
  "breaks": [
    {
      /** Error: break is outside of vehicle shift times **/
      "times": [
        [
          "2019-07-04T17:00:00Z",
          "2019-07-04T18:00:00Z"
        ]
      ],
      "duration": 3600.0
    }
  ]
}
```


#### E1304

`invalid reload time windows in vehicle shift` error is returned when vehicle has invalid time window of a reload. Reload
list should follow time window rules defined for jobs in E1003 except multiple reloads can have time window intersections.
Additionally, reload time should be inside vehicle shift it is specified:

```json
{
  "start": {
    "time": "2019-07-04T08:00:00Z",
    /** omitted **/
  },
  "end": {
    "time": "2019-07-04T15:00:00Z",
    /** omitted **/
  },
  "reloads": [
    {
      /** Error: reload is outside of vehicle shift times **/
      "times": [
        [
          "2019-07-04T17:00:00Z",
          "2019-07-04T18:00:00Z"
        ]
      ],
      "location": { /** omitted **/ },
      "duration": 3600.0
    }
  ]
}
```

#### E1306

`time and duration costs are zeros` is returned when both time and duration costs are zeros in vehicle type definition:

```json
{
  "typeId": "vehicle",
  "vehicleIds": [
    "vehicle_1"
  ],
  "profile": {
    "matrix": "car"
  },
  "costs": {
    "fixed": 20.0,
    /** Error: distance and time are zero **/
    "distance": 0,
    "time": 0
  },
  /** omitted **/
}
```

You can fix the error by defining a small value (e.g. 0.0000001) for duration or time costs.

#### E1308

`invalid vehicle reload resource` is returned when:

- `fleet.resources` has vehicle reloads with the same `id`
- required vehicle reload is used with resource id, which is not specified in `fleet.resources`


### E15xx: Routing profiles

These errors are related to routing locations and `fleet.profiles` property definitions.


#### E1500

`duplicate profile names` error is returned when `fleet.profiles` has more than one profile with the same name:

```json
{
  "profiles": [
    {
      "name": "vehicle_profile",
      "type": "car"
    },
    {
      "name": "vehicle_profile",
      "type": "truck"
    }
  ]
}
```

To fix the issue, remove all duplicates.


#### E1501

`empty profile collection` error is returned when `fleet.profiles` is empty:

```json
{
  "profiles": []
}
```

#### E1502

`mixing different location types` error is returned when problem contains locations in different formats. In order to
fix the issue, change the problem definition to use one specific location type: index reference or geocoordinate.


#### E1503

`location indices requires routing matrix to be specified` is returned when location indices are used, but no
routing matrix provided.


#### E1504

`amount of locations does not match matrix dimension` is returned when:

* location indices are used and max index is greater than matrix size
* amount of total locations is higher than matrix size

Check locations in problem definition and matrix size.


#### E1505

`unknown matrix profile name in vehicle or vicinity clustering profile` is returned when vehicle has in `fleet.vehicles.profile.matrix`
or `plan.clustering.profile` value which is not specified in `fleet.profiles` collection. To fix issue, either change
value to one specified or add a corresponding profile in profiles collection.


### E16xx: Objectives

These errors are related to `objectives` property definition.


#### E1600

`an empty objective specified` error is returned when objective property is present in the problem, but no single
objective is set, e.g.:

```json
{
  "objectives": []
}
```

`objectives` property is optional, just remove it to fix the problem and use default objectives.


#### E1601

`duplicate objective specified` error is returned when objective of specific type specified more than once:

```json
{
  "objectives": [
    {
      "type": "minimize-unassigned"
    },
    {
      "type": "minimize-unassigned"
    },
    {
      "type": "minimize-cost"
    }
  ]
}
```

To fix this issue, just remove one, e.g. `minimize-unassigned`.


#### E1602

`missing one of cost objectives` error is returned when no cost objective specified:

```json
{
  "objectives": [
    {
      "type": "minimize-unassigned"
    }
  ]
}
```

To solve it, specify one of the cost objectives: `minimize-cost`, `minimize-distance` or `minimize-duration`.


#### E1603

`redundant value objective` error is returned when objectives definition is overridden with `maximize-value`, but
there is no jobs with non-zero value specified. To fix the issue, specify at least one non-zero valued job or simply
delete 'maximize-value' objective.


#### E1604

`redundant tour order objective` error is returned when objectives definition is overridden with `tour-order`, but
there is no jobs with non-zero order specified. To fix the issue, specify at least one job with non-zero order or simply
delete 'tour-order' objective.


#### E1605

`value or order of a job should be greater than zero` error is returned when job's order or value is less than 1. To
fix the issue, make sure that value or order of all jobs are greater than zero.


#### E1606

`multiple cost objectives specified` error is returned when more than one cost objective is specified. To fix the issue,
keep only one cost objective in the list of objectives.


#### E1607

`missing value objective` error is returned when plan has jobs with value set, but user defined objective doesn't
include the `maximize-value` objective.
