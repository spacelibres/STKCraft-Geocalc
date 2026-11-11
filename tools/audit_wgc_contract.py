"""Record the published WGC contract and probe its documented examples.

Audit utility only: this does not implement a local WGC server. Uses curl so
the same explicitly supplied proxy works for all requests. Raw evidence is
written after each request; a NASA rejection is never a local compatibility pass.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import time
from html.parser import HTMLParser
from pathlib import Path

BASE = "https://wgc2.jpl.nasa.gov:8443/webgeocalc/api"
ROOT = Path(__file__).resolve().parents[1]


class Contract(HTMLParser):
    def __init__(self):
        super().__init__()
        self.heading = None
        self.heading_parts = []
        self.section = ""
        self.anchor = ""
        self.row = None
        self.cell = None
        self.pre = None
        self.fields = []
        self.examples = {}

    def finish_cell(self):
        if self.cell is not None and self.row is not None:
            self.row.append(" ".join("".join(self.cell).split()))
        self.cell = None

    def finish_row(self):
        self.finish_cell()
        if self.row is not None and len(self.row) >= 2 and self.row[0] not in ("Name", "URL parameters", "Request payload", "Response format", "Example request URL", "Example response", "Example Post Data", "Example Results", "Request method"):
            self.fields.append({"section": self.section, "anchor": self.anchor,
                                "field": self.row[0], "specification": self.row[1],
                                "assessment_scope": "REFERENCE_SPECIFICATION_ONLY"})
        self.row = None

    def handle_starttag(self, tag, attrs):
        if re.fullmatch(r"h[1-6]", tag):
            self.heading = tag
            self.heading_parts = []
            self.anchor = dict(attrs).get("id", "")
        if tag == "tr":
            self.finish_row()
            self.row = []
        if tag in ("td", "th"):
            self.finish_cell()
            self.cell = []
        if tag == "pre":
            self.pre = []

    def handle_data(self, data):
        if self.heading:
            self.heading_parts.append(data)
        if self.cell is not None:
            self.cell.append(data)
        if self.pre is not None:
            self.pre.append(data)

    def handle_endtag(self, tag):
        if tag == self.heading:
            self.section = " ".join("".join(self.heading_parts).split())
            self.heading = None
        if tag in ("td", "th"):
            self.finish_cell()
        if tag == "tr" and self.row is not None:
            self.finish_row()
        if tag == "pre" and self.pre is not None:
            raw = "".join(self.pre).strip()
            try:
                value = json.loads(raw)
                if isinstance(value, dict) and "calculationType" in value:
                    self.examples.setdefault(value["calculationType"], value)
            except ValueError:
                pass
            self.pre = None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--proxy", default="")
    parser.add_argument("--probe", action="store_true")
    args = parser.parse_args()
    out = ROOT / "docs/review-2026-11-11"
    out.mkdir(parents=True, exist_ok=True)
    source = ROOT / "target/wgc/audit-live-api-info.html"
    if not source.exists():
        source = out / "official-api-info.html"
    contract = Contract()
    contract.feed(source.read_text(encoding="utf-8"))
    (out / "contract-fields.json").write_text(json.dumps({
        "source": BASE.replace("/api", "/documents/api-info.html"),
        "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "note": "Reference specification index only. HTTP compatibility is outside scope; functional coverage is assessed in WGC_CONFORMANCE_REVIEW.md and calculation-matrix.csv.",
        "fields": contract.fields}, indent=2, ensure_ascii=False), encoding="utf-8")
    (out / "documented-requests.json").write_text(json.dumps(contract.examples, indent=2), encoding="utf-8")
    print(f"Extracted {len(contract.fields)} table rows; {len(contract.examples)} calculation examples", flush=True)
    if not args.probe:
        return
    evidence = []

    def request(label, method, path, payload=None):
        command = ["curl.exe", "--silent", "--show-error", "--max-time", "25", "--request", method,
                   "--header", "Content-Type: application/json", "--write-out", "\n%{http_code}", BASE + path]
        if args.proxy:
            command += ["--proxy", args.proxy]
        if payload is not None:
            command += ["--data-binary", "@-"]
        started = time.monotonic()
        result = subprocess.run(command, input=None if payload is None else json.dumps(payload),
                                capture_output=True, text=True)
        body, _, status = result.stdout.rpartition("\n")
        try:
            response = json.loads(body)
        except ValueError:
            response = body
        item = {"label": label, "method": method, "path": path, "request": payload,
                "http_status": status, "curl_exit": result.returncode,
                "elapsed_seconds": round(time.monotonic() - started, 3),
                "response": response, "stderr": result.stderr,
                "comparison_scope": "NASA_REFERENCE_ONLY"}
        evidence.append(item)
        (out / "live-probes.json").write_text(json.dumps(evidence, indent=2), encoding="utf-8")
        print(label, status, str(response)[:180], flush=True)
        return response if isinstance(response, dict) else {}

    request("api-root", "GET", "/")
    request("kernel-sets", "GET", "/kernel-sets")
    for name in ("bodies", "frames", "instruments", "surfaces"):
        request(name, "GET", f"/kernel-set/1/{name}")
    request("dsk-surfaces", "POST", "/dsk-surfaces", {"kernels": [{"type": "KERNEL_SET", "id": 1}]})
    request("invalid-empty-payload", "POST", "/calculation/new", {})
    request("unknown-id", "GET", "/calculation/00000000-0000-0000-0000-000000000000")
    for name, example in contract.examples.items():
        # Exact examples, including their kernel sets and nested structure.
        created = request(name, "POST", "/calculation/new", example)
        cid = created.get("calculationId")
        if not cid:
            continue
        phase = created.get("result", {}).get("phase")
        for _ in range(12):
            if phase in ("COMPLETE", "FAILED", "CANCELLED", "EXPIRED"):
                break
            time.sleep(0.5)
            status = request(name + ":status", "GET", f"/calculation/{cid}")
            phase = status.get("result", {}).get("phase")
        if phase in ("COMPLETE", "FAILED"):
            request(name + ":results", "GET", f"/calculation/{cid}/results")
            if name == "STATE_VECTOR":
                request("state:after-results", "GET", f"/calculation/{cid}")
                request("state:repeat-results", "GET", f"/calculation/{cid}/results")
                request("state:cancel-dispatched", "GET", f"/calculation/{cid}/cancel")
        else:
            request(name + ":cancel", "GET", f"/calculation/{cid}/cancel")


if __name__ == "__main__":
    main()
