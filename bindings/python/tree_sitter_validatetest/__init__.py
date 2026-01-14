"""GStreamer ValidateTest grammar for tree-sitter."""

from importlib.resources import files as _files

from ._binding import language


def _get_query(name, filename):
    query_path = _files("tree_sitter_validatetest") / "queries" / filename
    return query_path.read_text()


HIGHLIGHTS_QUERY = _get_query("highlights", "highlights.scm")
INJECTIONS_QUERY = _get_query("injections", "injections.scm")

__all__ = [
    "HIGHLIGHTS_QUERY",
    "INJECTIONS_QUERY",
    "language",
]
