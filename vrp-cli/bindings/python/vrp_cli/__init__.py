"""Python bindings for the `vrp-cli` Vehicle Routing Problem solver.

The solver exchanges json documents. Typed models for those documents are provided in
:mod:`vrp_cli.models` and are generated from the solver's own rust types, so they cannot drift from
what it actually accepts.

A minimal example::

    import vrp_cli
    from vrp_cli.models.config import Config, TerminationConfig
    from vrp_cli.models.problem import Fleet, Plan, Problem
    from vrp_cli.models.solution import Solution

    problem = Problem(plan=Plan(jobs=[...]), fleet=Fleet(vehicles=[...], profiles=[...]))
    config = Config(termination=TerminationConfig(maxTime=5))

    solution = Solution.model_validate_json(
        vrp_cli.solve_pragmatic(
            problem=problem.model_dump_json(exclude_none=True),
            matrices=[],
            config=config.model_dump_json(exclude_none=True),
        )
    )

Every function raises :class:`OSError` on failure. The message is a json array of errors, each with
a ``code``, ``cause`` and ``action``, so failures can be inspected programmatically::

    import json
    try:
        vrp_cli.solve_pragmatic(problem=..., matrices=[], config=...)
    except OSError as err:
        codes = [error["code"] for error in json.loads(str(err))]
"""

from . import models
from ._native import (
    convert_to_pragmatic,
    get_routing_locations,
    solve_pragmatic,
    validate_pragmatic,
)

__all__ = [
    "convert_to_pragmatic",
    "get_routing_locations",
    "models",
    "solve_pragmatic",
    "validate_pragmatic",
]
