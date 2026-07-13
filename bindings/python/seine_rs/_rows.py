"""Row-object ingestion sugar.

Lists of @seine_rs.fact instances, plain dicts, or any attribute-bearing
objects (dataclasses, Pydantic models, ...) convert into the certified
dict-of-column-lists path. This layer adds ZERO semantics: it only
reshapes rows into columns in schema order; all type checking and the
loud None/null rejection still happen at the certified boundary.
"""
from __future__ import annotations

from typing import Any, Optional


def _field_order(key: Any, rows: list) -> Optional[list[str]]:
    """Schema field order: the @fact class key wins, else the rows'
    own @fact class, else None (dict-key order of the first row)."""
    if hasattr(key, "__seine_fields__"):
        return list(key.__seine_fields__)
    if rows and hasattr(type(rows[0]), "__seine_fields__"):
        return list(type(rows[0]).__seine_fields__)
    return None


def _extract(row: Any, field: str, idx: int, type_name: str) -> Any:
    if isinstance(row, dict):
        if field not in row:
            raise ValueError(f"{type_name}: row {idx} is missing field {field!r}")
        return row[field]
    try:
        return getattr(row, field)
    except AttributeError:
        raise ValueError(
            f"{type_name}: row {idx} ({type(row).__name__}) has no attribute {field!r}"
        ) from None


def rows_to_columns(key: Any, rows: list) -> dict[str, list]:
    """A list of row objects/dicts -> {field: [values...]} in schema
    order. Raises ValueError on empty input (no schema to infer),
    mixed shapes, or missing fields."""
    type_name = key if isinstance(key, str) else key.__name__
    if not rows:
        raise ValueError(
            f"{type_name}: cannot infer a schema from an empty row list — "
            "pass an Arrow table or a dict of typed column lists instead"
        )
    fields = _field_order(key, rows)
    if fields is None:
        first = rows[0]
        if not isinstance(first, dict):
            raise ValueError(
                f"{type_name}: rows of type {type(first).__name__} carry no schema; "
                "use @seine_rs.fact instances, dicts, or declare with a @fact class key"
            )
        fields = list(first)
    return {f: [_extract(r, f, i, type_name) for i, r in enumerate(rows)] for f in fields}


def is_row_list(data: Any) -> bool:
    """True for a list/tuple of row-shaped things (not a table, not a
    dict of columns)."""
    if not isinstance(data, (list, tuple)):
        return False
    return all(isinstance(r, dict) or hasattr(type(r), "__dict__") or hasattr(r, "__slots__") for r in data) if data else True
