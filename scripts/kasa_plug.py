#!/usr/bin/env python3
"""Local Kasa plug control for razochar6e cycle (KLAP / modern firmware).

Requires: pip install python-kasa
Auth (modern plugs): KASA_USERNAME + KASA_PASSWORD, or --username/--password.

Usage:
  kasa_plug.py discover
  kasa_plug.py state --host 192.168.1.88
  kasa_plug.py on    --host 192.168.1.88
  kasa_plug.py off   --host 192.168.1.88
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import sys


def _creds(args: argparse.Namespace):
    from kasa import Credentials

    user = args.username or os.environ.get("KASA_USERNAME")
    password = args.password or os.environ.get("KASA_PASSWORD")
    if user and password:
        return Credentials(user, password)
    return None


async def _device(host: str, args: argparse.Namespace):
    from kasa import Discover

    return await Discover.discover_single(
        host,
        credentials=_creds(args),
        timeout=args.timeout,
    )


async def cmd_discover(args: argparse.Namespace) -> int:
    from kasa import Discover

    devices = await Discover.discover(
        credentials=_creds(args),
        timeout=args.timeout,
        discovery_timeout=args.timeout,
    )
    rows = []
    for host, dev in devices.items():
        try:
            await dev.update()
            rows.append(
                {
                    "host": host,
                    "alias": getattr(dev, "alias", None),
                    "model": getattr(dev, "model", None),
                    "is_on": bool(getattr(dev, "is_on", False)),
                }
            )
        except Exception as e:  # noqa: BLE001 — report per-device
            rows.append({"host": host, "error": str(e)})
    print(json.dumps({"devices": rows}, indent=2))
    return 0 if rows else 2


async def cmd_state(args: argparse.Namespace) -> int:
    dev = await _device(args.host, args)
    await dev.update()
    print(
        json.dumps(
            {
                "host": args.host,
                "alias": getattr(dev, "alias", None),
                "model": getattr(dev, "model", None),
                "is_on": bool(dev.is_on),
            }
        )
    )
    return 0


async def cmd_on(args: argparse.Namespace) -> int:
    dev = await _device(args.host, args)
    await dev.turn_on()
    await dev.update()
    print(json.dumps({"host": args.host, "is_on": bool(dev.is_on)}))
    return 0


async def cmd_off(args: argparse.Namespace) -> int:
    dev = await _device(args.host, args)
    await dev.turn_off()
    await dev.update()
    print(json.dumps({"host": args.host, "is_on": bool(dev.is_on)}))
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--username", default=None)
    p.add_argument("--password", default=None)
    p.add_argument("--timeout", type=float, default=8.0)
    sub = p.add_subparsers(dest="cmd", required=True)

    d = sub.add_parser("discover")
    d.set_defaults(func=cmd_discover)

    for name, fn in (("state", cmd_state), ("on", cmd_on), ("off", cmd_off)):
        s = sub.add_parser(name)
        s.add_argument("--host", required=True)
        s.set_defaults(func=fn)

    args = p.parse_args()
    try:
        return asyncio.run(args.func(args))
    except Exception as e:  # noqa: BLE001
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
