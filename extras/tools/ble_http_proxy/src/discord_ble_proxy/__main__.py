"""Compatibility module for ``python -m discord_ble_proxy``."""

from __future__ import annotations

from ble_http_proxy.cli import main

if __name__ == "__main__":  # pragma: no cover
    main()
