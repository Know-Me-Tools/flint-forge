#!/usr/bin/env python3
"""Run the separately approved issue 40 packet using two fresh local fixtures."""

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import signal
import subprocess
import sys
import time
import uuid

KIT = Path(__file__).resolve().parent
ROOT = KIT.parent.parent
MANIFEST = KIT / "manifest.json"
CONTRACT = ROOT / ".kbd-orchestrator/phases/private-file-metadata-authorization/adopted-contract.json"
CASES = ("issue40_owner_save", "issue40_authorization_controls")
POLICY_COMMIT = "c74c101826b59d8db50d142ea2f339aecf602f69"
OBSERVATIONS = {
    CASES[0]: (
        "FILE-1 owner insert=200 revision=1 update=200 revision=2 independent_full_row_matches=true",
    ),
    CASES[1]: (
        "FILE-2 insert=403 update=403 code=metadata_denied independent_state_matches=true",
        "FILE-5 off_tenant=403 missing_actor=403 wrong_actor=403 independent_state_matches=true",
        "FILE-4 type_registration=403 code=metadata_denied independent_type_set_unchanged=true",
        "FILE-3 http_owner_controls=200 foreign_update=409 independent_state_matches=true",
        "FILE-3 sql_axes=2 foreign_select=0 foreign_update=0 foreign_insert=rls_denied",
    ),
}
MIGRATIONS = ("0005_cedar_policies.sql", "0019_a2ui_cedar_mutation_capability.sql",
              "0021_file_metadata.sql")
LABEL = "org.flint-forge.issue40.run"
BASELINE = """
SELECT encode(sha256(convert_to(string_agg(id || chr(31) || policy_text, chr(30)
    ORDER BY id COLLATE "C"), 'UTF8')), 'hex')
FROM flint_meta.cedar_policies WHERE enabled;
"""


def output(args, **kwargs):
    return subprocess.check_output(args, text=True, **kwargs).strip()


def source_refs():
    baseline = json.loads((CONTRACT.parent / "file-api-source.json").read_text())
    changed = output(["git", "diff", "--name-only", baseline["commit"], POLICY_COMMIT], cwd=ROOT)
    production = {path for path in baseline["paths"] if "/src/" in path or path.endswith("Cargo.toml")}
    for path in sorted(production.union(changed.splitlines())):
        reviewed = subprocess.check_output(["git", "show", f"{POLICY_COMMIT}:{path}"], cwd=ROOT)
        if (ROOT / path).read_bytes() != reviewed:
            raise ValueError(f"Source differs from the reviewed policy revision: {path}")
    return {"file_api_commit": baseline["commit"], "reviewed_policy_commit": POLICY_COMMIT,
            "infra_identity_commit": "06a4bcf611dfae527acaf1aa85d8454a5b454dba",
            "gate_identity_test_commit": "1a4cd4297d0f952b0bfe232824070b6f97ac2a69",
            "fixture_manifest_sha256": hashlib.sha256(MANIFEST.read_bytes()).hexdigest()}


def observations(case, log, manifest):
    # With --exact and RUST_TEST_NOCAPTURE, libtest may print a case prefix
    # immediately before the first evidence marker and "ok" after the last.
    markers = re.compile(r"(?m)(?:^|(?<=\.\.\. ))issue40_(policy_set|evidence) ([^\r\n]*)\r?\n")
    events = markers.findall(log)
    statuses = markers.sub("", log)
    named = re.findall(rf"(?m)^test {re.escape(case)} \.\.\. ok$", statuses)
    totals = re.findall(r"(?m)^test result: ok\. 1 passed; 0 failed; 0 ignored;", statuses)
    sets = [re.fullmatch(r"baseline_sha256=([a-f0-9]{64}) enabled_sha256=([a-f0-9]{64})", value)
            for kind, value in events if kind == "policy_set"]
    policy = sets[0] if len(sets) == 1 else None
    emitted = [value for kind, value in events if kind == "evidence"]
    expected = OBSERVATIONS[case]
    return {"ran_named_case_once": len(named) == len(totals) == 1,
            "baseline_sha256": policy[1] if policy else None,
            "enabled_sha256": policy[2] if policy else None,
            "observations": [item for item in emitted if item in expected],
            "complete_evidence": bool(policy and policy[1] == manifest["baseline_sha256"]
                                      and sorted(emitted) == sorted(expected))}


def local_docker():
    endpoint = os.environ.get("DOCKER_HOST")
    if not endpoint or os.environ.get("DOCKER_CONTEXT"):
        endpoint = output(["docker", "context", "inspect", "--format",
                           "{{.Endpoints.docker.Host}}", output(["docker", "context", "show"])])
    if not endpoint.startswith("unix:///") or not Path(endpoint[7:]).is_socket():
        raise ValueError("This fixture requires a local Docker Unix socket.")
    return ["docker", "--host", endpoint]


def inputs():
    manifest = json.loads(MANIFEST.read_text())
    if (manifest["host"] != "127.0.0.1" or manifest["port"] != 55440
            or manifest["database"] != "forge_issue40_file_metadata_test"
            or manifest["fixture_id"] != "forge-40-rls-v1"
            or manifest["container_name"] != "flint-forge-issue40-postgres"
            or not re.fullmatch(r"postgres:18\.7-bookworm@sha256:[0-9a-f]{64}", manifest["image"])):
        raise ValueError("Fixture manifest differs from the approved local target.")
    migrations = []
    for name in MIGRATIONS:
        data = (ROOT / "migrations" / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != manifest["migrations"][name]:
            raise ValueError(f"Migration differs from the fixture manifest: {name}")
        migrations.append(data.decode())
    commands = json.loads(CONTRACT.read_text())["boundaryPacket"]["commands"]
    expected = ["env", "FLINT_AUTHZ_MODE=rls", "FLINT_FILE_METADATA_ENABLED=true",
                "FILE_METADATA_TEST_ALLOW_WRITES=isolated-disposable", "cargo", "test",
                "--offline", "-p", "fdb-gateway", "--test", "file_metadata_live_pg"]
    if [c["argv"] for c in commands] != [expected + [case, "--", "--ignored", "--exact"] for case in CASES]:
        raise ValueError("The phase verification commands have changed. Review the packet.")
    return manifest, migrations, commands


@contextmanager
def database(docker, manifest, migrations):
    """Own only this invocation's container and its memory-backed PGDATA."""
    password = secrets.token_hex(32)
    run_id = str(uuid.uuid4())
    db = manifest["database"]
    # Send the random password via stdin into the container's memory-backed /dev/shm.
    # This keeps it out of Docker's recorded environment and the host filesystem.
    startup = ('while [ ! -f /dev/shm/issue40-password ]; do sleep 0.1; done; '
               'exec docker-entrypoint.sh postgres -c forge.fixture_id=forge-40-rls-v1')
    container = output(docker + ["create", "--pull", "never", "--name", manifest["container_name"], "--label",
                                 f"{LABEL}={run_id}", "--publish", "127.0.0.1:55440:5432",
                                 "--log-driver", "none", "--tmpfs", "/var/lib/postgresql",
                                 "--env", f"POSTGRES_DB={db}", "--env",
                                 "POSTGRES_PASSWORD_FILE=/dev/shm/issue40-password",
                                 "--entrypoint", "sh", manifest["image"], "-ec", startup])
    try:
        subprocess.run(docker + ["start", container], stdout=subprocess.DEVNULL, check=True)
        subprocess.run(docker + ["exec", "-i", container, "sh", "-ec",
                                 "umask 077; cat > /dev/shm/issue40-tmp; "
                                 "mv /dev/shm/issue40-tmp /dev/shm/issue40-password"],
                       input=password + "\n", text=True, check=True)
        for _ in range(60):
            rc = subprocess.run(docker + ["exec", container, "pg_isready", "-q",
                                          "-h", "127.0.0.1", "-d", db, "-U", "postgres"]).returncode
            if rc == 0:
                break
            time.sleep(1)
        else:
            raise RuntimeError("PostgreSQL did not become ready on its TCP listener.")
        subprocess.run(docker + ["exec", container, "rm", "/dev/shm/issue40-password"], check=True)
        psql = docker + ["exec", "-i", "-u", "postgres", container, "psql", "-XAtq",
                         "-v", "ON_ERROR_STOP=1", "-d", db]
        identity = output(psql, input="SELECT current_database(), current_setting('forge.fixture_id');\n")
        if identity != db + "|" + manifest["fixture_id"]:
            raise RuntimeError("Connected fixture identity differs from the approved manifest.")
        setup = ["BEGIN;", "CREATE ROLE authenticated NOLOGIN NOBYPASSRLS;",
                 "CREATE ROLE service_role NOLOGIN;", "CREATE SCHEMA flint_meta;", *migrations, "COMMIT;"]
        subprocess.run(psql, text=True, input="\n".join(setup), check=True)
        if output(psql, input=BASELINE) != manifest["baseline_sha256"]:
            raise RuntimeError("Enabled Cedar policies differ from the approved baseline.")
        yield f"postgresql://postgres:{password}@127.0.0.1:55440/{db}", password
    finally:
        owner = output(docker + ["inspect", "--format",
                                 '{{ index .Config.Labels "' + LABEL + '" }}', container])
        if owner != run_id:
            raise RuntimeError("Container ownership changed. Cleanup requires operator inspection.")
        subprocess.run(docker + ["rm", "-fv", container], stdout=subprocess.DEVNULL, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--approved", action="store_true", required=True,
                        help="Acknowledges explicit operator authorization for the isolated fixture and writes.")
    parser.add_argument("--receipts", required=True, type=Path, help="A new directory for redacted run receipts.")
    args = parser.parse_args()
    manifest, migrations, commands = inputs()
    refs = source_refs()
    docker = local_docker()
    args.receipts.mkdir(parents=True, exist_ok=False, mode=0o700)
    (args.receipts / "manifest.json").write_bytes(MANIFEST.read_bytes())
    subprocess.run(docker + ["pull", manifest["image"]], check=True)
    for case, command in zip(CASES, commands):
        print(f"Provisioning the disposable fixture for {case}.", flush=True)
        with database(docker, manifest, migrations) as (url, password):
            env = {k: v for k, v in os.environ.items()
                   if not k.startswith(("PG", "FLINT_", "FILE_METADATA_TEST_"))
                   and k not in {"DATABASE_URL", "RUST_LOG"}}
            env["FILE_METADATA_TEST_DATABASE_URL"] = url
            env["FILE_METADATA_TEST_FIXTURE_MANIFEST"] = str(MANIFEST)
            env["RUST_TEST_NOCAPTURE"] = "1"
            result = subprocess.run(command["argv"], env=env, cwd=ROOT, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, text=True, errors="replace")
            log = result.stdout.replace(url, "[fixture connection]").replace(password, "[redacted]")
            log = re.sub(r'\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+', '[redacted JWT]', log)
            log = re.sub(r'(?im)^(\s*(?:left|right):) [^\n]*(?:Object|Array|\{|\[)[^\n]*',
                         r'\1 [redacted assertion payload]', log)
            log = re.sub(r'(?i)\b[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}\b', '[fixture id]', log)
            (args.receipts / f"{case}.log").write_text(log)
            receipt = {"case": case, "argv": command["argv"], "exit_code": result.returncode,
                       "fixture_id": manifest["fixture_id"], "source": refs,
                       **observations(case, log, manifest)}
            (args.receipts / f"{case}.json").write_text(json.dumps(receipt, indent=2) + "\n")
            if result.returncode or not receipt["ran_named_case_once"] or not receipt["complete_evidence"]:
                raise RuntimeError(f"{case} failed or lacks required evidence; inspect its redacted receipt.")
    print(f"Both named cases executed. Receipts: {args.receipts}")


if __name__ == "__main__":
    def interrupt(_signum, _frame):
        raise KeyboardInterrupt
    signal.signal(signal.SIGTERM, interrupt)
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError, KeyboardInterrupt) as exc:
        sys.exit(str(exc) or "Fixture run interrupted.")
