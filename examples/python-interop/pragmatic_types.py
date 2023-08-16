# Contains semi-automatically generated non-complete model of pragmatic format.
# Please refer to documentation to define a full model

from __future__ import annotations
from pydantic.dataclasses import dataclass
from typing import List, Optional
from datetime import datetime


# Routing matrix

@dataclass
class RoutingMatrix:
    profile: str
    durations: List[int]
    distances: List[int]


# Problem

@dataclass
class Problem:
    plan: Plan
    fleet: Fleet
    objectives: Optional[List[List[Objective]]] = None


@dataclass
class Plan:
    jobs: List[Job]
    relations: Optional[List[Relation]] = None


@dataclass
class Job:
    id: str
    pickups: Optional[List[JobTask]] = None
    deliveries: Optional[List[JobTask]] = None


@dataclass
class JobTask:
    places: List[JobPlace]
    demand: List[int]


@dataclass
class JobPlace:
    location: Location
    duration: float
    times: Optional[List[List[datetime]]] = None
    tag: Optional[str] = None


@dataclass
class VehicleReload:
    location: Location
    duration: float


@dataclass
class Location:
    lat: float
    lng: float


@dataclass
class Relation:
    type: str
    jobs: List[str]
    vehicleId: str


@dataclass
class Fleet:
    vehicles: List[VehicleType]
    profiles: List[RoutingProfile]


@dataclass
class VehicleType:
    typeId: str
    vehicleIds: List[str]
    profile: VehicleProfile
    costs: VehicleCosts
    shifts: List[VehicleShift]
    capacity: List[int]


@dataclass
class VehicleProfile:
    matrix: str


@dataclass
class VehicleCosts:
    fixed: float
    distance: float
    time: float


@dataclass
class VehicleShift:
    start: VehicleShiftStart
    end: VehicleShiftEnd
    breaks: Optional[List[VehicleBreak]] = None
    reloads: Optional[List[VehicleReload]] = None


@dataclass
class VehicleShiftStart:
    earliest: datetime
    location: Location
    latest: Optional[datetime] = None


@dataclass
class VehicleShiftEnd:
    latest: datetime
    location: Location
    earliest: Optional[datetime] = None


@dataclass
class VehicleBreak:
    time: List[datetime]
    places: List[JobPlace]


@dataclass
class RoutingProfile:
    name: str


@dataclass
class Objective:
    type: str
    options: Optional[ObjectiveOptions] = None


@dataclass
class ObjectiveOptions:
    threshold: float


Problem.__pydantic_model__.update_forward_refs()

Plan.__pydantic_model__.update_forward_refs()
Job.__pydantic_model__.update_forward_refs()
JobTask.__pydantic_model__.update_forward_refs()
JobPlace.__pydantic_model__.update_forward_refs()

Fleet.__pydantic_model__.update_forward_refs()
VehicleReload.__pydantic_model__.update_forward_refs()
VehicleType.__pydantic_model__.update_forward_refs()
VehicleShift.__pydantic_model__.update_forward_refs()
VehicleShiftStart.__pydantic_model__.update_forward_refs()
VehicleShiftEnd.__pydantic_model__.update_forward_refs()
VehicleBreak.__pydantic_model__.update_forward_refs()

Objective.__pydantic_model__.update_forward_refs()


# Solution

@dataclass
class Solution:
    statistic: Statistic
    tours: List[Tour]


@dataclass
class Statistic:
    cost: float
    distance: int
    duration: int
    times: Times


@dataclass
class Times:
    driving: int
    serving: int
    waiting: int
    commuting: int
    parking: int


@dataclass
class Tour:
    vehicleId: str
    typeId: str
    shiftIndex: int
    stops: List[Stop]
    statistic: Statistic


@dataclass
class Stop:
    location: Location
    time: Schedule
    distance: int
    load: List[int]
    activities: List[Activity]


@dataclass
class Schedule:
    arrival: datetime
    departure: datetime


@dataclass
class Activity:
    jobId: str
    type: str
    location: Optional[Location] = None
    time: Optional[Time] = None
    jobTag: Optional[str] = None


@dataclass
class Time:
    start: datetime
    end: datetime


Solution.__pydantic_model__.update_forward_refs()
Statistic.__pydantic_model__.update_forward_refs()
Tour.__pydantic_model__.update_forward_refs()
Stop.__pydantic_model__.update_forward_refs()
Activity.__pydantic_model__.update_forward_refs()
