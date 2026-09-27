#!/usr/bin/env python3
"""Local Kasa plug control for razochar6e cycle (KLAP / modern firmware).

Requires: pip install python-kasa
Auth (modern plugs): KASA_USERNAME + KASA_PASSWORD, or --username/--password.

EP10 (and similar IOT.SMARTPLUGSWITCH + KLAP lv2 / new_klap) devices need
IotProtocol + KlapTransportV2. Stock Discover picks IotProtocol + KlapTransport
(v1 hashes), which rejects the correct cloud password. This helper forces the
working combo after discovery.

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


async def _connect(host: str, args: argparse.Namespace):
    """Connect with the transport that actually authenticates on this firmware."""
    from kasa import (
        Credentials,
        Device,
        DeviceConfig,
        DeviceConnectionParameters,
        DeviceEncryptionType,
        DeviceFamily,
    )
    from kasa.iot.iotplug import IotPlug
    from kasa.protocols.iotprotocol import IotProtocol
    from kasa.transports.klaptransport import KlapTransport, KlapTransportV2

    creds = _creds(args)
    if creds is None:
        raise RuntimeError(
            "KASA_USERNAME and KASA_PASSWORD (or --username/--password) are required "
            "for KLAP plugs"
        )

    timeout = args.timeout
    # EP10 IOT+KLAP lv2 needs IotProtocol + KlapTransportV2 (stock Discover uses
    # KlapTransport v1 hashes and rejects the correct cloud password).
    attempts = [
        (DeviceFamily.IotSmartPlugSwitch, KlapTransportV2),
        (DeviceFamily.IotSmartPlugSwitch, KlapTransport),
        (DeviceFamily.SmartKasaPlug, KlapTransportV2),
    ]
    last_err: Exception | None = None

    for family, transport_cls in attempts:
        try:
            conn = DeviceConnectionParameters(
                family,
                DeviceEncryptionType.Klap,
                login_version=2,
            )
            cfg = DeviceConfig(
                host,
                credentials=creds,
                connection_type=conn,
                timeout=timeout,
            )
            if family is DeviceFamily.IotSmartPlugSwitch:
                transport = transport_cls(config=cfg)
                protocol = IotProtocol(transport=transport)
                dev = IotPlug(host, config=cfg, protocol=protocol)
            else:
                dev = await Device.connect(config=cfg)
            await dev.update()
            return dev
        except Exception as e:  # noqa: BLE001
            last_err = e
            continue

    raise RuntimeError(f"could not authenticate to {host}: {last_err}")


async def cmd_discover(args: argparse.Namespace) -> int:
    from kasa import Discover

    devices = await Discover.discover(
        credentials=_creds(args),
        timeout=args.timeout,
        discovery_timeout=args.timeout,
    )
    rows = []
    for host, _dev in devices.items():
        try:
            live = await _connect(host, args)
            rows.append(
                {
                    "host": host,
                    "alias": getattr(live, "alias", None),
                    "model": getattr(live, "model", None),
                    "is_on": bool(getattr(live, "is_on", False)),
                }
            )
            try:
                await live.disconnect()
            except Exception:
                pass
        except Exception as e:  # noqa: BLE001 — report per-device
            rows.append({"host": host, "error": str(e)})
    print(json.dumps({"devices": rows}, indent=2))
    return 0 if rows else 2


async def cmd_state(args: argparse.Namespace) -> int:
    dev = await _connect(args.host, args)
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
    dev = await _connect(args.host, args)
    await dev.turn_on()
    await dev.update()
    print(json.dumps({"host": args.host, "is_on": bool(dev.is_on)}))
    return 0


async def cmd_off(args: argparse.Namespace) -> int:
    dev = await _connect(args.host, args)
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
        s.add_argument("--username", default=None)
        s.add_argument("--password", default=None)
        s.set_defaults(func=fn)

    args = p.parse_args()
    try:
        return asyncio.run(args.func(args))
    except Exception as e:  # noqa: BLE001
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
