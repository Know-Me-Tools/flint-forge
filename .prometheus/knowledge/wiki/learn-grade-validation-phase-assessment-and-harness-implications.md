---
type: Reference
id: learn-grade-validation-phase-assessment-and-harness-implications
title: learn-grade Validation Phase Assessment and Harness Implications
tags:
- learn-grade
- grader-validation
- feynman-learning
- evaluation-dataset
- agent-harness
- phase-tracking
sources:
- stdin
- manual:phase-learn-grader-validation
timestamp: 2026-07-16T19:35:26.615795+00:00
created_at: 2026-07-16T19:35:26.615795+00:00
updated_at: 2026-07-16T19:35:26.615795+00:00
revision: 0
---

## Phase Context

- Phase: `phase-learn-grader-validation`
- Project: unspecified
- KBD root: `/Users/gqadonis/Projects/prometheus/prometheus-skill-pack`
- Captured: `2026-07-16T19:35:02Z`
- Source marker: `manual:phase-learn-grader-validation`
- Status: `assessment_complete`
- Progress: `0/0 changes`, `0/6 goals`
- Last action: `assessment.md` written with 2 major findings and 4 open questions
- Next action: `/kbd-plan phase-learn-grader-validation`
- Commit pushed: `dca9da2`

## Background

`phase-learn-feynman` v1.4.0 closed on `2026-06-28` and shipped `learn-grade`, a sycophancy-corrected external grader intended to close each Feynman learning loop. Confidence in the grader was assessed at only 60–70%.

The phase reflection identified this as the highest-severity open risk in the learn domain:

> A grader that misses misconceptions is worse than no grader — it provides false assurance.

No empirical validation dataset currently exists. The grader has not been tested against explanations with known, expert-labeled gaps. This phase is intended to create that dataset and measure actual precision/recall.

## Phase Goals

### G-01: Grader Evaluation Dataset

Assemble at least 20 Feynman explanations across at least 3 subject domains, such as:

- One STEM topic
- One humanities topic
- One technical/programming topic

Each explanation must include expert-authored ground-truth annotations:

- Misconceptions present
- Misconceptions absent
- Gold-standard score

Target storage path:

```text
skills/learn/learn-grade/references/eval-dataset/
```

Dataset format must be machine-readable, using JSON or YAML per explanation.

### G-02: Run `learn-grade` Against Dataset

Create a script or harness that:

- Feeds each explanation through the actual `learn-grade` skill/protocol path, not a mock.
- Captures the grader score and misconception list.
- Diffs grader output against ground truth.

## Assessment Findings

### Missing Prior Integration Test Artifacts

The 4 integration-test changes marked `DONE` in `phase-learn-feynman` have no artifacts anywhere in the repository.

Implication: any G-05 regression or integration test work must build a new test harness from scratch rather than extending an existing one.

### `learn-grade` Is Not a Callable Script

`learn-grade` is a prose-executed 9-step agent protocol, not a directly callable script. The only real code artifact is `write-grade.sh`, which performs the final write step.

Implications for G-02:

- “Running the grader” means invoking an agent per evaluation item.
- The harness cannot simply call a local executable for each fixture.
- Cost, batching, and parallelization are material design concerns.
- Recommended direction: use `Agent`/`Workflow` fan-out instead of sequential turns.

## Rubric and Metrics

Rubric interpretation was confirmed:

- `misconceptions_absent` is binary, so precision/recall applies.
- The other 3 rubric dimensions are continuous values in `[0, 1]`, so correlation metrics apply.

This matches the assumptions already present in `goals.md`.

## Open Planning Questions

1. **Expert ground-truth authorship**
   - Human review is required.
   - Ground truth should not be treated as fully automatable.

2. **G-05 regression test strategy**
   - Options:
     - Snapshot comparison: CI-safe and deterministic.
     - Live-agent re-grade: expensive and potentially non-deterministic.
   - Recommendation: snapshot comparison.

3. **Domain selection**
   - Investigate whether `change-learn-016`'s meta-grounding corpus can be reused.

4. **G-02 cost and batching**
   - Recommended approach: `Agent`/`Workflow` fan-out rather than sequential agent turns.

# Citations

1. stdin
2. manual:phase-learn-grader-validation