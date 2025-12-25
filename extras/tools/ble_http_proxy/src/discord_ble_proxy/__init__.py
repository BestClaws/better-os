"""Compatibility package forwarding to :mod:`ble_http_proxy`."""

from __future__ import annotations

import ble_http_proxy as _proxy

from ble_http_proxy import *  # type: ignore F401,F403

__all__ = list(getattr(_proxy, "__all__", ()))
