"""Compatibility CLI shim for legacy imports."""

from __future__ import annotations

from ble_http_proxy.cli import main  # noqa: F401

__all__ = ["main"]
