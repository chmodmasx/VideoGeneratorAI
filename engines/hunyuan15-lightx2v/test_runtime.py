from __future__ import annotations

import contextlib
import io
import json
import unittest

import runtime


class RuntimeProtocolTests(unittest.TestCase):
    def setUp(self) -> None:
        self.original_protocol_stdout = runtime.PROTOCOL_STDOUT
        self.protocol = io.StringIO()
        runtime.PROTOCOL_STDOUT = self.protocol

    def tearDown(self) -> None:
        runtime.PROTOCOL_STDOUT = self.original_protocol_stdout

    def test_emit_uses_reserved_protocol_stream_during_stdout_redirect(self) -> None:
        upstream_stdout = io.StringIO()

        with contextlib.redirect_stdout(upstream_stdout):
            print("upstream noise")
            runtime.emit({"type": "ack", "request_id": 7})

        self.assertEqual(upstream_stdout.getvalue(), "upstream noise\n")
        self.assertEqual(
            json.loads(self.protocol.getvalue()),
            {"type": "ack", "request_id": 7},
        )


if __name__ == "__main__":
    unittest.main()
