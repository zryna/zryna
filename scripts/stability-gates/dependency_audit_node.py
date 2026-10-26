"""Fixed Node reader/owner dispatch, not caller-supplied commands or receipt import."""
import os
from pathlib import Path
import subprocess
import tempfile
from dependency_audit_inputs import decode, encode, require, retain, sha


def node_call(node, root, output, mode, name):
    require(mode in ("--locks", "--msal", "--msal-control"), "fixed Node method")
    script = Path(__file__).with_name("dependency-lock-inputs.mjs")
    argv = [str(node), str(script), mode, str(root)]
    if mode == "--msal-control":
        argv.append(str(output))
    allowed = ("PATH", "HOME", "USERPROFILE", "TMP", "TEMP", "TMPDIR", "SystemRoot", "WINDIR",
               "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY", "http_proxy", "https_proxy", "no_proxy",
               "SSL_CERT_FILE", "SSL_CERT_DIR", "NODE_EXTRA_CA_CERTS", "NODE_USE_SYSTEM_CA")
    environment = {key: os.environ[key] for key in allowed if key in os.environ}
    # NODE_OPTIONS/preload/debug/credential variables cannot rewrite the fixed reader/owner.
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        error, code = None, None
        try:
            code = subprocess.run(argv, cwd=root, env=environment, stdin=subprocess.DEVNULL,
                stdout=stdout, stderr=stderr, timeout=45, check=False).returncode
        except subprocess.TimeoutExpired:
            error = "fixed Node bridge exceeded 45-second outer guard"
        stdout.seek(0); raw = stdout.read(1024 * 1024 + 1)
        stderr.seek(0); diagnostic = stderr.read(1024 * 1024 + 1)
    retain(output, name + ".stdout.log", raw)
    retain(output, name + ".stderr.log", diagnostic)
    retain(output, name + ".status.json", encode({"argv": argv, "exitCode": code, "error": error,
        "stdoutBytesRetained": len(raw), "stderrBytesRetained": len(diagnostic),
        "stdoutSha256": sha(raw), "stderrSha256": sha(diagnostic)}))
    require(len(raw) <= 1024 * 1024 and len(diagnostic) <= 1024 * 1024, "bounded bridge result, prefix is invalid")
    require(code == 0 and error is None and not diagnostic, "Node bridge/owner acquisition failure")
    return decode(raw, 1024 * 1024)
