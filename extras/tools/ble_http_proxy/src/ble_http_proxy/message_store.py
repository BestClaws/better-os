"""Local message board storage for the BLE HTTP proxy."""

from __future__ import annotations

import asyncio
from collections import deque
from typing import Iterable, List

_MAX_MESSAGE_LENGTH = 64
_PLACEHOLDER = "..."


def _sanitize_ascii(text: str) -> str:
    filtered = [ch for ch in text if 32 <= ord(ch) < 127]
    sanitized = "".join(filtered).strip()
    if not sanitized:
        return _PLACEHOLDER
    return sanitized[:_MAX_MESSAGE_LENGTH]


class MessageStore:
    """Thread-safe ring buffer of ASCII messages."""

    def __init__(self, capacity: int = 3) -> None:
        if capacity <= 0:
            raise ValueError("capacity must be positive")
        self._messages = deque([_PLACEHOLDER] * capacity, maxlen=capacity)
        self._lock = asyncio.Lock()

    @property
    def capacity(self) -> int:
        return self._messages.maxlen or 0

    async def seed(self, messages: Iterable[str]) -> None:
        async with self._lock:
            self._messages.clear()
            for message in messages:
                self._messages.append(_sanitize_ascii(message))
            while len(self._messages) < self.capacity:
                self._messages.append(_PLACEHOLDER)

    async def append(self, message: str) -> None:
        async with self._lock:
            self._messages.append(_sanitize_ascii(message))

    async def snapshot(self) -> List[str]:
        async with self._lock:
            return list(self._messages)

    async def render_snapshot(self) -> bytes:
        messages = await self.snapshot()
        joined = "\n".join(messages)
        return joined.encode("ascii", "ignore")
