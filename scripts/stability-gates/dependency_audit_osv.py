"""Exact OSV snapshot parsing and bounded ordinary proxy/TLS capture, no waiver API."""
import json
import re
import time
import urllib.error
import urllib.request
from dependency_audit_inputs import AuditError, decode, encode, require, retain, sha
from dependency_audit_msal import backport_record
import dependency_audit_block_buffer as block_buffer

ENDPOINT = "https://api.osv.dev/v1/"
ADVISORY = "GHSA-xcx6-jx8p-x3h5"
ADVISORY_SHA = "d4793c5901cfdb7e546430dfc396feafe0eff80aaf9f465458f9a1a07cff3236"


def parse_batch(raw, batch):
    document = decode(raw)
    require(isinstance(document, dict) and set(document) == {"results"}, "closed querybatch response")
    require(isinstance(document["results"], list) and len(document["results"]) == len(batch), "complete query results")
    result = []
    for query, row in zip(batch, document["results"]):
        require(isinstance(row, dict) and set(row) <= {"vulns", "next_page_token"}, "query result fields")
        if "next_page_token" in row:
            require(isinstance(row["next_page_token"], str) and row["next_page_token"] == "",
                    "only absent or empty-string pagination is complete")
        vulns = row.get("vulns", [])
        require(isinstance(vulns, list) and len(vulns) <= 256, "bounded advisory matches")
        seen = set()
        for item in vulns:
            require(isinstance(item, dict) and set(item) == {"id", "modified"}, "query advisory identity")
            require(isinstance(item["id"], str) and re.fullmatch(r"[A-Za-z0-9_-]{1,128}", item["id"]), "advisory ID")
            require(isinstance(item["modified"], str) and 0 < len(item["modified"]) <= 64, "modified timestamp")
            require(item["id"] not in seen, "duplicate matched advisory")
            seen.add(item["id"])
        result.append({"query": query, "advisories": vulns})
    return result


def parse_advisory(raw, identifier):
    value = decode(raw, 4 * 1024 * 1024)
    require(isinstance(value, dict) and value.get("id") == identifier, "fetched advisory identity")
    require(value.get("withdrawn") is None, "withdrawn advisory requires review; never silently ignored")
    require(isinstance(value.get("affected"), list) and value["affected"], "advisory affected inventory")
    require(isinstance(value.get("modified"), str), "advisory modified field")
    return value


def bind_modified(results, details):
    for row in results:
        for match in row["advisories"]:
            detail = details[match["id"]]
            # One retained provider pair, not equal instants, truncation or a waiver.
            retained_pair = (match["id"] == ADVISORY and
                match["modified"] == "2026-10-23T22:30:05.010413Z" and
                detail["modified"] == "2026-10-23T22:30:05.010413241Z" and
                sha(encode(detail)) == ADVISORY_SHA)
            original_buffer_pair = (row["query"] == block_buffer.QUERY and match["id"] == block_buffer.ADVISORY and
                match["modified"] == "2026-08-19T19:30:06.686866Z" and
                detail["modified"] == "2026-08-19T19:30:06.686866977Z" and
                sha(encode(detail)) == block_buffer.DETAIL_SHA)
            require(match["modified"] == detail["modified"] or retained_pair or original_buffer_pair,
                    "querybatch/full-advisory modified binding")


def classify(results, details, backport, original_buffer=None):
    reviewed_backport = backport_record(backport) if backport is not None else None
    reviewed_buffer = block_buffer.not_affected_record(original_buffer) if original_buffer is not None else None
    visible = [{"query": row["query"], "advisory": advisory["id"]}
               for row in results for advisory in row["advisories"]]
    msal = [row for row in results if row["query"] == {
        "package": {"ecosystem": "npm", "name": "@azure/msal-node"}, "version": "6.0.1"}]
    if msal:
        require(len(msal) == 1 and any(x["id"] == ADVISORY for x in msal[0]["advisories"]),
                "known raw MSAL6 advisory must remain visible")
    unresolved, remediated, not_affected = [], [], []
    for row in visible:
        identifier = row["advisory"]
        detail = details[identifier]
        exact = row["query"] == {"package": {"ecosystem": "npm", "name": "@azure/msal-node"}, "version": "6.0.1"}
        if exact and identifier == ADVISORY and sha(encode(detail)) == ADVISORY_SHA and reviewed_backport is not None:
            remediated.append({**row, "disposition": "exact-reviewed-upstream-backport-and-current-owner-execution",
                               "backport": reviewed_backport})
        elif row["query"] == block_buffer.QUERY and identifier == block_buffer.ADVISORY and \
                sha(encode(detail)) == block_buffer.DETAIL_SHA and reviewed_buffer is not None:
            not_affected.append({**row, "disposition": "exact-reviewed-original-source-not-affected",
                                 "originalSourceOwner": reviewed_buffer})
        else:
            unresolved.append(row)
    return {"observedDependencyGate": "pass" if not unresolved else "fail",
            "rawAdvisoryMatches": visible, "sourceBoundRemediations": remediated,
            "sourceBoundNotAffected": not_affected, "unresolved": unresolved}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


class Capture:
    def __init__(self, output):
        self.output = output
        self.started = time.monotonic()
        self.requests = 0
        # Default ProxyHandler and HTTPSHandler preserve normal proxy and CA trust.
        self.opener = urllib.request.build_opener(NoRedirect())

    def request(self, path, name, body=None, maximum=16 * 1024 * 1024):
        self.requests += 1
        require(self.requests <= 512 and time.monotonic() - self.started < 600, "finite capture request budget")
        require(path == "querybatch" or re.fullmatch(r"vulns/[A-Za-z0-9_-]{1,128}", path), "fixed OSV endpoint")
        if body is not None:
            retain(self.output, name + ".request.json", body)
        request = urllib.request.Request(ENDPOINT + path, data=body,
            headers={"Content-Type": "application/json", "User-Agent": "zryna-dependency-advisory-capture/1"},
            method="POST" if body is not None else "GET")
        raw, status, date, error = bytearray(), None, None, None
        begin = time.monotonic()
        try:
            try:
                response = self.opener.open(request, timeout=45)
            except urllib.error.HTTPError as failure:
                response = failure
            with response:
                status, date = response.status, response.headers.get("Date")
                while len(raw) <= maximum:
                    require(time.monotonic() - begin < 45 and time.monotonic() - self.started < 600,
                            "observed HTTP elapsed budget")
                    chunk = response.read1(min(65536, maximum + 1 - len(raw)))
                    raw.extend(chunk)
                    require(time.monotonic() - begin < 45 and time.monotonic() - self.started < 600,
                            "observed HTTP elapsed budget after read including EOF")
                    if not chunk:
                        break
        except Exception as failure:  # Retain partial raw response and exact acquisition failure first.
            error = str(failure)[:4096]
        retain(self.output, name + ".response.json", bytes(raw))
        retain(self.output, name + ".http.json", encode({"status": status, "httpDate": date,
            "error": error, "elapsedSeconds": time.monotonic() - begin, "bytes": len(raw),
            "socketTimeoutSeconds": 45, "observedRequestDeadlineSeconds": 45,
            "url": ENDPOINT + path}))
        require(error is None and status == 200 and len(raw) <= maximum, "HTTP/status/size capture failure")
        return bytes(raw)
