# Verification — p16-c004

## Gate
A monitored private disclosure channel exists and is documented.

## Evidence to record on completion
- [x] SECURITY.md at repo root, names a real monitored channel — GitHub
      private vulnerability reporting (decided via AskUserQuestion, 2026-07-16)
- [x] GitHub private vulnerability reporting enabled —
      `gh api --method PUT repos/Know-Me-Tools/flint-forge/private-vulnerability-reporting`
      confirmed `{"enabled":true}`
- [x] Supported-versions table, consistent with SemVer claim in CHANGELOG.md —
      1.0.x supported, pre-1.0 not; matches the single-release history
- [x] SUPPORT.md, CONTRIBUTING.md present
- [x] No response-time commitment the maintainers cannot meet — stated 5
      business days acknowledgement, biweekly status updates, no fixed
      resolution SLA (explicitly justified in the doc), 90-day default
      coordinated-disclosure window

## Verification command output
```
test -f SECURITY.md && test -f SUPPORT.md && test -f CONTRIBUTING.md && echo OK
# OK
gh api repos/Know-Me-Tools/flint-forge/private-vulnerability-reporting
# {"enabled":true}
```

## Decisions recorded (2026-07-16, via AskUserQuestion)
- Disclosure channel: GitHub Security Advisories / private vulnerability
  reporting (not a dedicated email address)
- Repo setting change (enabling private vulnerability reporting) was
  confirmed with the user before being applied via `gh api`, since it's a
  shared-state change visible to the whole team, not a local file edit

## Status
COMPLETE — 5/5 tasks.
