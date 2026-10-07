# Phase 2: Engine Feasibility Spike (GATE) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md. This log preserves the alternatives considered.

**Date:** 2026-10-07
**Phase:** 02-engine-feasibility-spike-gate
**Areas discussed:** Spike scope and candidates, Spike code vs real protocol, Account/ToS/test setup, Go/no-go criteria and report

---

## Spike scope and candidates

| Question | Options | Selected |
|----------|---------|----------|
| Engine candidates to run | ECS first, Chrome if it fails; ECS and Chrome both; ECS only | ECS first, Chrome if it fails |
| ECS version | Newest supported tag; pin v44.1.0 | Newest supported tag |
| Hidden window | Wayland and X11; Wayland only | Wayland and X11 |
| Web player fragility | Record version, no mitigation; add smoke script | Record version, no mitigation |

## Spike code vs real protocol

| Question | Options | Selected |
|----------|---------|----------|
| Engine to Rust protocol | Real presto-ipc; ad-hoc JSON | Real presto-ipc |
| Rust driver | Spike binary; REPL; both | Spike binary |
| Code after gate | Seed for Phase 3; throwaway | Seed for Phase 3 |
| Bridge script | Standalone file; inlined | Standalone file |

## Account, ToS and test setup

| Question | Options | Selected |
|----------|---------|----------|
| Account | Own subscriber account; separate test account | Own subscriber account |
| ToS reading | Gate task before sign-in; in report | Gate task before sign-in |
| Sign-in | By hand in visible window; reuse system browser profile | By hand in visible window |
| Profile dir | Dedicated XDG state dir, 0700; temp dir per run | Dedicated XDG state dir, 0700 |

## Go/no-go criteria and report

| Question | Options | Selected |
|----------|---------|----------|
| Pass bar | All five criteria; playback and sign-in only | All five criteria |
| RSS threshold | Measure only; set ceiling now | Measure only |
| ECS failure | Try Chrome via CDP; stop and report | Try Chrome via CDP |
| Report form | Markdown plus raw logs; markdown only | Markdown plus raw logs |

---

## Claude's Discretion

Spike crate layout, measurement method, test track choice, report structure beyond SPIKE-06.

## Deferred Ideas

Smoke script for web player breakage; REPL driver.
