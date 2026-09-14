# Contains semi-automatically generated non-complete model of config format.
# Please refer to documentation to define a full model

from __future__ import annotations
from pydantic.dataclasses import dataclass
from typing import Optional


@dataclass
class Telemetry:
    progress: Progress


@dataclass
class Progress:
    enabled: bool
    logBest: int
    logPopulation: int
    dumpPopulation: bool


@dataclass
class Config:
    termination: Termination
    telemetry: Optional[Telemetry] = Telemetry(
        progress=Progress(
            enabled=True,
            logBest=100,
            logPopulation=1000,
            dumpPopulation=False
        )
    )
    environment: Optional[Environment] = None


@dataclass
class Termination:
    maxTime: Optional[int] = None
    maxGenerations: Optional[int] = None


@dataclass
class Logging:
    enabled: bool


@dataclass
class Environment:
    logging: Logging = Logging(enabled=True)
    isExperimental: Optional[bool] = None
