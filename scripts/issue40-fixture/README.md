# Forge issue 40 disposable verification fixture

## Submit approval

Reply in the current Codex chat with this text. The runner creates its own temporary credentials.

> I approve the Forge issue #40 isolated synthetic verification using scripts/issue40-fixture/manifest.json. Run the two named checks once. You may pull the pinned PostgreSQL image, create the loopback fixtures, apply the listed migrations, write synthetic rows and grants, save redacted evidence, and remove the fixtures afterward.

## Run after approval

From the Forge checkout, run the following command with Python 3 and a local Docker daemon. The Rust toolchain and offline dependencies must already be available. Choose a new receipt directory for this authorized run.

```sh
python3 scripts/issue40-fixture/run.py --approved \
  --receipts .kbd-orchestrator/phases/private-file-metadata-authorization/issue40-run
```

The manifest binds the fixture to `127.0.0.1:55440/forge_issue40_file_metadata_test`, the PostgreSQL 18.7 image used in the identity verification, the three migration contents and the initial enabled Cedar policy. The runner reads the two commands directly from the phase contract and checks their arguments. In order, they select `issue40_owner_save` and `issue40_authorization_controls` from `file_metadata_live_pg`. It requires a successful named test and a count of one for each invocation.

Each case gets a fresh memory-backed PostgreSQL database. A randomly generated password goes to PostgreSQL via stdin and to the test process via its environment. The runner checks the current database, fixture marker and baseline policy set. It then sets `FILE_METADATA_TEST_FIXTURE_MANIFEST` and `FILE_METADATA_TEST_DATABASE_URL` for the test. Within the fixture, a setup connection applies the existing Forge migrations and the gateway switches to the ordinary `authenticated` role for owner operations.

The baseline SHA-256 covers the enabled rows ordered by ID using the C collation. The bytes for each row are its UTF-8 ID, byte 31, and its UTF-8 policy text. Byte 30 separates rows. The fixture starts with the A2UI policy supplied by migration 0019. The record grants belong to the individual synthetic case.

The runner stores the nonsecret manifest, redacted test output, exact arguments and exit status in the receipt directory. It stops after the first unsuccessful case. Cleanup checks the unique ownership label and removes only the container ID this run created. A name or port collision stops provisioning. If the process is forcibly killed, inspect that container’s ID and the `org.flint-forge.issue40.run` label before any manual cleanup. No running service or existing database is a target of this approval.
