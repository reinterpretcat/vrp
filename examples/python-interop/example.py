"""Solves a small Vehicle Routing Problem using the `vrp-cli` package.

Run it with:

    pip install vrp-cli
    python examples/python-interop/example.py

The models used below ship with the package as `vrp_cli.models`. They are generated from the
solver's own rust types, so they always match the format it accepts.
"""

import json

import vrp_cli
from vrp_cli.models.config import Config, TerminationConfig
from vrp_cli.models.matrix import Matrix
from vrp_cli.models.problem import (
    Fleet,
    Job,
    JobPlace,
    JobTask,
    LocationCoordinate,
    MatrixProfile,
    Plan,
    Problem,
    ShiftEnd,
    ShiftStart,
    VehicleCosts,
    VehicleProfile,
    VehicleShift,
    VehicleType,
)
from vrp_cli.models.solution import Solution

# Routing information between every pair of locations, in the order returned by
# `vrp_cli.get_routing_locations`. Omit it entirely to let the solver approximate distances, or
# fetch it from a real routing service.
matrix = Matrix(
    profile="normal_car",
    travelTimes=[0, 609, 981, 906, 813, 0, 371, 590, 1055, 514, 0, 439, 948, 511, 463, 0],
    distances=[0, 3840, 5994, 5333, 4696, 0, 2154, 3226, 5763, 2674, 0, 2145, 5112, 2470, 2152, 0],
)

# When to stop searching: whichever limit is reached first.
config = Config(termination=TerminationConfig(maxTime=5, maxGenerations=1000))

problem = Problem(
    plan=Plan(
        jobs=[
            # a job which only delivers goods, within a time window
            Job(
                id="delivery_job1",
                deliveries=[
                    JobTask(
                        places=[
                            JobPlace(
                                location=LocationCoordinate(lat=52.52599, lng=13.45413),
                                duration=300,
                                times=[["2019-07-04T09:00:00Z", "2019-07-04T18:00:00Z"]],
                            )
                        ],
                        demand=[1],
                    )
                ],
            ),
            # a job which only picks goods up
            Job(
                id="pickup_job2",
                pickups=[
                    JobTask(
                        places=[
                            JobPlace(
                                location=LocationCoordinate(lat=52.5225, lng=13.4095),
                                duration=240,
                                times=[["2019-07-04T10:00:00Z", "2019-07-04T16:00:00Z"]],
                            )
                        ],
                        demand=[1],
                    )
                ],
            ),
            # a job which moves goods from one place to another: the pickup is always served
            # before the delivery, and both are assigned to the same vehicle or neither is
            Job(
                id="pickup_delivery_job3",
                pickups=[
                    JobTask(
                        places=[
                            JobPlace(
                                location=LocationCoordinate(lat=52.5225, lng=13.4095),
                                duration=300,
                                tag="p1",
                            )
                        ],
                        demand=[1],
                    )
                ],
                deliveries=[
                    JobTask(
                        places=[
                            JobPlace(
                                location=LocationCoordinate(lat=52.5165, lng=13.3808),
                                duration=300,
                                tag="d1",
                            )
                        ],
                        demand=[1],
                    )
                ],
            ),
        ]
    ),
    fleet=Fleet(
        vehicles=[
            VehicleType(
                typeId="vehicle",
                vehicleIds=["vehicle_1"],
                profile=VehicleProfile(matrix="normal_car"),
                costs=VehicleCosts(fixed=22, distance=0.0002, time=0.005),
                shifts=[
                    VehicleShift(
                        start=ShiftStart(
                            earliest="2019-07-04T09:00:00Z",
                            location=LocationCoordinate(lat=52.5316, lng=13.3884),
                        ),
                        end=ShiftEnd(
                            latest="2019-07-04T18:00:00Z",
                            location=LocationCoordinate(lat=52.5316, lng=13.3884),
                        ),
                    )
                ],
                capacity=[10],
            )
        ],
        profiles=[MatrixProfile(name="normal_car")],
    ),
)

# `exclude_none=True` leaves optional fields out instead of sending explicit nulls
problem_json = problem.model_dump_json(exclude_none=True)
matrix_json = matrix.model_dump_json(exclude_none=True)

# Any of the calls below raises `OSError` whose message is a json array of errors, each carrying a
# `code`, a `cause` and a suggested `action`.
try:
    solution = Solution.model_validate_json(
        vrp_cli.solve_pragmatic(
            problem=problem_json,
            matrices=[matrix_json],
            config=config.model_dump_json(exclude_none=True),
        )
    )
except OSError as err:
    for error in json.loads(str(err)):
        print(f"{error['code']}: {error['cause']}\n  {error['action']}")
        raise SystemExit(1) from err

print(f"cost: {solution.statistic.cost:.2f}")
print(f"distance: {solution.statistic.distance} m, duration: {solution.statistic.duration} s")
print(f"tours: {len(solution.tours)}, unassigned: {len(solution.unassigned or [])}")

for tour in solution.tours:
    print(f"\nvehicle {tour.vehicleId} visits {len(tour.stops)} stops:")
    for stop in tour.stops:
        served = ", ".join(activity.jobId for activity in stop.activities)
        print(f"  {stop.time.arrival} -> {served}")
