# Violations

Some of the constraints, specified by the problem, are considered as soft and can be violated under certain circumstances.
Violations are listed in `violations` collection and divided in to specific groups.


## Vehicle Break violation

A vehicle break is considered as soft constraint and can be violated if the solver is not able to assign it. When it is
violated, the following object is returned:

```json
{
  "type": "break",
  "vehicleId": "my_vehicle_id",
  "shiftIndex": 0,
  "reason": "cannot be visited within time window"
}
```