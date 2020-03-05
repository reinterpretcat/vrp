# Error Index

## E1xxx: Validation errors

Errors with E1xxx mask are used by validation engine.


### E1000

This error is returned when `plan.jobs` has jobs with the same ids.

```json
{
  "plan": {
    "jobs": [
      {
        "id": "job1",
        /** omitted **/
      },
      {
        /** Invalid: this id is already used by another job **/
        "id": "job1",
        /** omitted **/
      }
      /** omitted **/
    ]
  }
}
```


### E1001

This error code is returned when job has both pickups and deliveries, but the sum of pickups demand does not match to
the sum of deliveries demand.

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
       "demand": [1],
      },
    ],
    "deliveries": [
      {
       "places": [/** omitted **/],
        "demand": [
          /** Invalid: should be 2 as the sum of pickups is 2 **/
          1
        ],
      }
    ]
}
```

### E1002

This error indicates the problem with multiple time window definition on single job: they should not intersect as
such time windows can be merged into one.


