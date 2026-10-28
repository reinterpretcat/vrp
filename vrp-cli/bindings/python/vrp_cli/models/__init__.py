"""Typed models for the json documents the solver reads and writes.

The modules here are generated from the json schemas in the crate's ``bindings/schemas``
directory, which are in turn derived from the solver's rust types. Do not edit them by hand.

One module per document:

============================== ========================================================
:mod:`vrp_cli.models.problem`  a problem definition in `pragmatic` format
:mod:`vrp_cli.models.matrix`   a routing matrix
:mod:`vrp_cli.models.config`   the solver configuration
:mod:`vrp_cli.models.solution` a solution as produced by the solver
:mod:`vrp_cli.models.error`    a single entry of a reported failure
============================== ========================================================

The document roots are re-exported here for convenience. Import anything else from its own module:
the problem and solution documents describe some concepts under different contracts and so define
separate types for them, ``Location`` being the notable example.

Models serialize with the json field names by default, so ``model_dump_json()`` produces a document
the solver accepts without ``by_alias``. Pass ``exclude_none=True`` to omit optional fields rather
than emit nulls.
"""

from . import config, error, matrix, problem, solution
from .config import Config
from .error import FormatError
from .matrix import Matrix
from .problem import Problem
from .solution import Solution

__all__ = [
    "Config",
    "FormatError",
    "Matrix",
    "Problem",
    "Solution",
    "config",
    "error",
    "matrix",
    "problem",
    "solution",
]
