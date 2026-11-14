#!/usr/bin/env python3
"""Exercise the real binary with scripted Responses API output; no API key needed.

Run `cargo build`, then `python3 scripts/check_assignment_harness.py`.
Fixtures use seed 1's hand-checked target: user 1 -> job 2; users 75 and 57
must retain jobs 9 and 5. Seed 2 is already satisfied. Expected outcomes
below are specified independently of the harness's ground-truth evaluator.
"""

import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from threading import Thread


def call(name, arguments=None):
    return {"type": "function_call", "name": name,
            "arguments": json.dumps({} if arguments is None else arguments)}


def listing():
    return call("list_assignments")


def update(user=1, job=2):
    return call("update_assignment", {"user_id": user, "job_id": job})


def commit():
    return call("commit")


def fixtures():
    # name, seed, scripted turns, expected persisted measurements
    return [
        ("correct", 1, [listing(), update(), commit()],
         dict(success=True, state_match=True, model_calls=3, tool_calls=3)),
        ("omitted_update", 1, [listing(), commit()],
         dict(success=False, state_match=False)),
        ("wrong_person", 1, [listing(), update(75), commit()],
         dict(success=False, state_match=False)),
        ("wrong_job", 1, [listing(), update(job=3), commit()],
         dict(success=False, state_match=False)),
        ("unintended_change", 1, [listing(), update(), update(75), commit()],
         dict(success=False, state_match=False)),
        ("corrected_mistake", 1, [listing(), update(job=3), update(), commit()],
         dict(success=True, state_match=True, update_calls=2, required_updates=1)),
        ("already_satisfied", 2, [listing(), commit()],
         dict(success=True, required_updates=0, update_calls=0, tool_calls=2)),
        ("unknown_user_recovery", 1, [listing(), update(999), update(), commit()],
         dict(success=True, failed_tool_calls=1, update_calls=2)),
        ("malformed_update_recovery", 1,
         [listing(), call("update_assignment", {"user_id": 1}), update(), commit()],
         dict(success=True, failed_tool_calls=1, update_calls=2)),
        ("malformed_commit_recovery", 1,
         [listing(), update(), call("commit", {"extra": True}), commit()],
         dict(success=True, failed_tool_calls=1, commit_calls=2)),
        ("noop_count", 1, [listing(), update(), update(), commit()],
         dict(success=True, noop_update_calls=1, update_calls=2)),
        ("premature_commit", 1, [commit()],
         dict(success=False, model_committed=True, state_match=False, tool_calls=1)),
        ("matching_without_commit", 1,
         [listing(), update(), {"type": "message", "role": "assistant",
                                "content": [{"type": "output_text", "text": "Done"}]}],
         dict(success=False, state_match=True, model_committed=False,
              termination_reason="missing_commit", model_calls=3, tool_calls=2)),
        ("commit_on_request_20", 1, [listing(), update()] + [listing()] * 17 + [commit()],
         dict(success=True, model_calls=20, tool_calls=20, commit_calls=1)),
        ("update_on_request_20", 1, [listing()] * 19 + [update()],
         dict(success=False, state_match=True, model_committed=False,
              termination_reason="network_limit", model_calls=20, tool_calls=20)),
        ("batch_updates_and_commit", 3,
         [listing(), [update(97, 2), update(75, 9), update(83, 9), commit()]],
         dict(success=True, model_calls=2, tool_calls=5, update_calls=3)),
        ("batch_results_then_commit", 3,
         [listing(), [update(97, 2), update(75, 9), update(83, 9)], commit()],
         dict(success=True, model_calls=3, tool_calls=5)),
        ("batch_error_recovery", 1,
         [listing(), [update(999), update(75, 9)], update(), commit()],
         dict(success=True, failed_tool_calls=1, noop_update_calls=1, tool_calls=5)),
        ("batch_commit_after_error", 1, [listing(), [update(999), commit()]],
         dict(success=False, state_match=False, model_committed=True,
              failed_tool_calls=1, tool_calls=3)),
        ("batch_malformed_commit_continues", 1,
         [listing(), [call("commit", {"extra": True}), update(), commit()]],
         dict(success=True, failed_tool_calls=1, commit_calls=2, model_calls=2)),
        ("calls_after_commit_skipped", 1, [listing(), [update(), commit(), update(75)]],
         dict(success=True, rejected_tool_calls=1, tool_calls=3, update_calls=1)),
        ("second_commit_skipped", 1, [listing(), [update(), commit(), commit()]],
         dict(success=True, rejected_tool_calls=1, commit_calls=1, tool_calls=3)),
        ("batch_commit_on_request_20", 1, [listing()] * 19 + [[update(), commit()]],
         dict(success=True, model_calls=20, tool_calls=21, commit_calls=1)),
        ("tool_count_independent_of_request_count", 1,
         [listing(), [update()] * 25 + [commit()]],
         dict(success=True, model_calls=2, tool_calls=27, noop_update_calls=24)),
        ("api_failure", 1, [500],
         dict(success=False, termination_reason="infrastructure_error",
              model_calls=1, tool_calls=0)),
    ]


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_POST(self):
        server = self.server
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        server.requests.append(request)
        index = len(server.requests) - 1
        if index >= len(server.turns):
            self.send_error(500, "Unexpected extra model request")
            return
        turn = server.turns[index]
        status = turn if isinstance(turn, int) else 200
        if status == 200:
            output = turn if isinstance(turn, list) else [turn]
            output = [dict(item, call_id=f"call_{index}_{i}")
                      if item["type"] == "function_call" else item
                      for i, item in enumerate(output)]
            payload = {"status": "completed", "output": output}
        else:
            payload = {"error": {"message": "Controlled infrastructure failure"}}
        body = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/agent-harness"))
    parser.add_argument("--output", type=Path, help="Save fixture measurements as JSON")
    args = parser.parse_args()
    binary = args.binary.resolve()
    results = []
    with tempfile.TemporaryDirectory(prefix="assignment-harness-check-") as directory:
        with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
            thread = Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                for name, seed, turns, expected in fixtures():
                    server.turns, server.requests = turns, []
                    database = Path(directory) / f"{name}.sqlite3"
                    env = dict(os.environ, OPENAI_API_KEY="offline-fixture-key",
                               OPENAI_MODEL="offline-fixture",
                               OPENAI_API_URL=f"http://127.0.0.1:{server.server_port}/v1/responses",
                               ASSIGNMENTS_DB=str(database), TASK_SEED=str(seed))
                    process = subprocess.run([str(binary)], env=env, capture_output=True,
                                             text=True, timeout=30)
                    with sqlite3.connect(database) as con:
                        rows = con.execute("SELECT result FROM session_results").fetchall()
                        assert len(rows) == 1, (name, process.stdout, process.stderr)
                        result = json.loads(rows[0][0])
                        for key, value in expected.items():
                            assert result[key] == value, (name, key, value, result[key])
                        assert process.returncode == (1 if name == "api_failure" else 0), name
                        assert len(server.requests) == len(turns), name
                        assert result["model_calls"] == len(turns), name
                        calls = con.execute("SELECT name, status, output, error FROM tool_calls ORDER BY id").fetchall()
                        assert all(status not in {"pending", "dispatched"}
                                   for _, status, _, _ in calls), name
                        assert result["commit_calls"] == sum(
                            tool == "commit" and status != "rejected"
                            for tool, status, _, _ in calls), name
                        assert result["tool_calls"] == sum(
                            status != "rejected" for _, status, _, _ in calls), name
                        assert result["rejected_tool_calls"] == sum(
                            status == "rejected" for _, status, _, _ in calls), name
                        assert all(error is not None for _, status, _, error in calls
                                   if status == "rejected"), name
                        if result["model_committed"]:
                            dispatched = [c for c in calls if c[1] != "rejected"]
                            assert dispatched[-1][:2] == ("commit", "succeeded"), name
                            assert json.loads(dispatched[-1][2]) == {"finalised": True}, name
                        # Check actual state without using ground_truth_assignments.
                        actual = dict(con.execute("SELECT user_id, job_id FROM session_assignments"))
                        independent_target = {
                            1: {1: 2, 75: 9, 57: 5},
                            2: {71: 1, 22: 6, 59: 7},
                            3: {75: 9, 83: 9, 97: 2},
                        }[seed]
                        assert result["state_match"] == (actual == independent_target), name
                        failed = con.execute("SELECT COUNT(*) FROM tool_calls WHERE status='failed'").fetchone()[0]
                        assert result["failed_tool_calls"] == failed, name
                        assert con.execute("SELECT status FROM sessions").fetchone()[0] == "finalised", name
                    # Exact first-request keys prevent hidden grading metadata entering context.
                    first_input = server.requests[0]["input"]
                    assert len(first_input) == 2, name
                    assert first_input[0]["role"] == "system", name
                    assert first_input[1]["role"] == "user", name
                    assert set(server.requests[0]) == {"model", "input", "tools", "reasoning", "max_output_tokens", "parallel_tool_calls"}, name
                    assert all(request["parallel_tool_calls"] is True
                               for request in server.requests), name
                    # Every call in a previous batch must have exactly one correlated
                    # result in the next request, including recoverable tool errors.
                    previous_calls = []
                    for index, request in enumerate(server.requests):
                        result_ids = [item["call_id"] for item in request["input"]
                                      if item.get("type") == "function_call_output"]
                        assert result_ids == previous_calls, (name, index, result_ids)
                        turn = turns[index]
                        items = turn if isinstance(turn, list) else [turn]
                        previous_calls += [f"call_{index}_{i}" for i, item in enumerate(items)
                                           if isinstance(item, dict) and item["type"] == "function_call"]
                    results.append(dict(name=name, passed=True, result=result))
                    print(f"PASS {name}", flush=True)
            finally:
                server.shutdown()
                thread.join()
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(results, indent=2) + "\n")
    print(f"{len(results)} controlled fixtures passed")


if __name__ == "__main__":
    main()
