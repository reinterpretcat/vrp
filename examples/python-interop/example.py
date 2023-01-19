import vrp_cli
import pragmatic_types as prg
import config_types as cfg
import json
from pydantic.json import pydantic_encoder

# if you want to use approximation, you can skip this definition and pass empty list later
# also there is a get_locations method to get list of locations in expected order.
# you can use this list to fetch routing matrix externally
matrix = prg.Matrix(
    profile='normal_car',
    travelTimes=[0, 609, 981, 906, 813, 0, 371, 590, 1055, 514, 0, 439, 948, 511, 463, 0],
    distances=[0, 3840, 5994, 5333, 4696, 0, 2154, 3226, 5763, 2674, 0, 2145, 5112, 2470, 2152, 0]
)


# specify termination criteria: max running time in seconds or max amount of refinement generations
config = cfg.Config(
    termination=cfg.Termination(
        maxTime=5,
        maxGenerations=1000
    )
)

# specify test problem
problem = prg.Problem(
    plan=prg.Plan(
        jobs=[
            prg.Job(
                id='delivery_job1',
                deliveries=[
                    prg.JobTask(
                        places=[
                            prg.JobPlace(
                                location=prg.Location(lat=52.52599, lng=13.45413),
                                duration=300,
                                times=[['2019-07-04T09:00:00Z', '2019-07-04T18:00:00Z']]
                            ),
                        ],
                        demand=[1]
                    )
                ]
            ),
            prg.Job(
                id='pickup_job2',
                pickups=[
                    prg.JobTask(
                        places=[
                            prg.JobPlace(
                                location=prg.Location(lat=52.5225, lng=13.4095),
                                duration=240,
                                times=[['2019-07-04T10:00:00Z', '2019-07-04T16:00:00Z']]
                            )
                        ],
                        demand=[1]
                    )
                ]
            ),
            prg.Job(
                id="pickup_delivery_job3",
                pickups=[
                    prg.JobTask(
                        places=[
                            prg.JobPlace(
                                location=prg.Location(lat=52.5225, lng=13.4095),
                                duration=300,
                                tag="p1"
                            )
                        ],
                        demand=[1]
                    )
                ],
                deliveries=[
                    prg.JobTask(
                        places=[
                            prg.JobPlace(
                                location=prg.Location(lat=52.5165, lng=13.3808),
                                duration=300,
                                tag="d1"
                            ),
                        ],
                        demand=[1]
                    )
                ]
            )
        ]
    ),
    fleet=prg.Fleet(
        vehicles=[
            prg.VehicleType(
                typeId='vehicle',
                vehicleIds=['vehicle_1'],
                profile=prg.VehicleProfile(matrix='normal_car'),
                costs=prg.VehicleCosts(fixed=22, distance=0.0002, time=0.005),
                shifts=[
                    prg.VehicleShift(
                        start=prg.VehicleShiftStart(
                            earliest="2019-07-04T09:00:00Z",
                            location=prg.Location(lat=52.5316, lng=13.3884),
                        ),
                        end=prg.VehicleShiftEnd(
                            latest="2019-07-04T18:00:00Z",
                            location=prg.Location(lat=52.5316, lng=13.3884),
                        )
                    )
                ],
                capacity=[10]
            )
        ],
        profiles=[prg.RoutingProfile(name='normal_car')]
    )

)

# run solver and deserialize result into solution model
solution = prg.Solution(**json.loads(vrp_cli.solve_pragmatic(
    problem=json.dumps(problem, default=pydantic_encoder),
    matrices=[json.dumps(matrix, default=pydantic_encoder)],
    config=json.dumps(config, default=pydantic_encoder),
)))

print(solution)
