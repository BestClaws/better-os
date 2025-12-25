"""Command-line interface for the generic BLE HTTP proxy."""

from __future__ import annotations

import argparse
import asyncio
import logging
from typing import Optional

from .ble_bridge import BleHttpBridge, BridgeConfig
from .message_store import MessageStore
from .router import HttpRouter


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Generic BLE HTTP proxy for Better OS")
    parser.add_argument(
        "--device-name",
        default="Better HTTP",
        help="BLE advertised name exposed by the firmware",
    )
    parser.add_argument(
        "--service-uuid",
        default="408813df-3469-4f19-9d01-7c87f7f04001",
        help="BLE service UUID to filter during discovery",
    )
    parser.add_argument(
        "--default-upstream",
        default=None,
        help="Optional base URL used when the firmware sends relative paths",
    )
    parser.add_argument(
        "--max-messages",
        type=int,
        default=3,
        help="Number of messages kept in the local message board",
    )
    parser.add_argument(
        "--seed",
        action="append",
        default=None,
        metavar="MESSAGE",
        help="Seed message for the local message board (can be provided multiple times)",
    )
    return parser


async def async_main(args: argparse.Namespace) -> None:
    logging.basicConfig(level=logging.INFO, format="[%(levelname)s] %(message)s")
    bridge_config = BridgeConfig(device_name=args.device_name, service_uuid=args.service_uuid)
    message_store = MessageStore(capacity=args.max_messages)

    if args.seed:
        await message_store.seed(args.seed)

    async with HttpRouter(message_store, default_upstream=args.default_upstream) as router:
        bridge = BleHttpBridge(bridge_config, router.handle)
        await bridge.run()


def main(argv: Optional[list[str]] = None) -> None:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        asyncio.run(async_main(args))
    except KeyboardInterrupt:
        print("Interrupted")
