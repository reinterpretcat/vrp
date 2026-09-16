"""Shared behavior for generated pydantic document models."""

from typing import Any

from pydantic import BaseModel, ConfigDict


class Model(BaseModel):
    """Rejects unknown fields and serializes Python-safe names with their JSON aliases."""

    model_config = ConfigDict(extra="forbid", populate_by_name=True)

    def model_dump(self, *, by_alias: bool | None = True, **kwargs: Any) -> dict[str, Any]:
        """Serializes with JSON field names unless explicitly requested otherwise."""
        return super().model_dump(by_alias=by_alias, **kwargs)

    def model_dump_json(self, *, by_alias: bool | None = True, **kwargs: Any) -> str:
        """Serializes with JSON field names unless explicitly requested otherwise."""
        return super().model_dump_json(by_alias=by_alias, **kwargs)
