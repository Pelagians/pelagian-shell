#!/usr/bin/env python3
"""Keep the pinned Selkies WebSocket video path active during runtime smoke.

An HTTPS GET does not start capture. Without a viewer, the pinned outer
compositor can withhold frame callbacks from Labwc and its GTK clients.
Use the same SETTINGS and frame acknowledgements as the Selkies web client,
and require decoded JPEG data before declaring the stream ready.
"""

import asyncio
import io
import json
import os
from pathlib import Path
import ssl
import sys

from PIL import Image
from websockets.asyncio.client import connect


async def main():
    width, height = map(int, sys.argv[1:3])
    state_dir = Path("/tmp/pelagian-stream-smoke")
    state_dir.mkdir(mode=0o700, exist_ok=True)
    (state_dir / "pid").write_text(str(os.getpid()))
    ready = state_dir / "ready"
    ready.unlink(missing_ok=True)
    # The ephemeral test container serves a self-signed certificate on loopback.
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    tls.check_hostname = False
    tls.verify_mode = ssl.CERT_NONE
    async with connect(
        "wss://127.0.0.1:3001/websockets",
        ssl=tls,
        open_timeout=10,
        max_size=16 * 1024 * 1024,
    ) as client:
        await client.send("SETTINGS," + json.dumps({
            "displayId": "primary",
            "is_manual_resolution_mode": True,
            "manual_width": width,
            "manual_height": height,
            "encoder": "jpeg",
            "framerate": 30,
            "use_cpu": True,
        }))
        rows = set()
        async for message in client:
            if isinstance(message, str):
                if message.startswith("KILL"):
                    raise RuntimeError(message)
                continue
            # JPEG packet: type, flags, frame ID (u16 BE), stripe Y (u16 BE), JPEG.
            if len(message) <= 6 or message[0] != 0x03:
                continue
            frame_id = int.from_bytes(message[2:4], "big")
            y = int.from_bytes(message[4:6], "big")
            with Image.open(io.BytesIO(message[6:])) as stripe:
                stripe.load()
                assert stripe.format == "JPEG"
                assert stripe.width == width and 0 < stripe.height <= height
                assert y + stripe.height <= height
                rows.update(range(y, y + stripe.height))
            await client.send(f"CLIENT_FRAME_ACK {frame_id}")
            if len(rows) == height and not ready.exists():
                ready.write_text(f"decoded {width}x{height}\n")
                print(f"selkies smoke: decoded {width}x{height}", flush=True)
    raise RuntimeError("Selkies closed the smoke stream")


if __name__ == "__main__":
    asyncio.run(main())
