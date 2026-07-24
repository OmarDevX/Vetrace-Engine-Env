#!/usr/bin/env python3
"""Development server for Vetrace Web with correct WASM MIME and no stale cache."""

from __future__ import annotations

import argparse
import functools
import http.server
import mimetypes
from pathlib import Path

mimetypes.add_type("application/wasm", ".wasm")
mimetypes.add_type("text/javascript", ".js")
mimetypes.add_type("application/json", ".json")


class VetraceHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self) -> None:
        self.send_header("Cache-Control", "no-store, no-cache, must-revalidate, max-age=0")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")
        self.send_header("X-Content-Type-Options", "nosniff")
        super().end_headers()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--port", type=int, default=8080)
    parser.add_argument("--bind", default="127.0.0.1")
    args = parser.parse_args()

    handler = functools.partial(VetraceHandler, directory=str(args.directory.resolve()))
    server = http.server.ThreadingHTTPServer((args.bind, args.port), handler)
    print(f"Serving Vetrace website at http://{args.bind}:{args.port}/website/")
    print(f"Examples: http://{args.bind}:{args.port}/website/examples/")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
