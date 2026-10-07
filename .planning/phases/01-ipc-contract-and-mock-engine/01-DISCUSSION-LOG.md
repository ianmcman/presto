# Phase 1: IPC Contract and Mock Engine - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md. This log preserves the alternatives considered.

**Date:** 2026-10-07
**Phase:** 1-IPC Contract and Mock Engine
**Areas discussed:** Transport, Message vocabulary, Fault injection, Seam notes and layout

---

## Transport

| Question | Options | Selected |
|----------|---------|----------|
| Connection | Unix socket / Child stdio / Stdio now, socket later | Unix socket |
| Lifecycle | presto spawns engine / Engine runs independently | presto spawns engine |
| Version mismatch | Exact major match / Major.minor with capabilities | Major.minor with capabilities |
| Engine logs | Capture to log file / Inherit to terminal / You decide | Capture to log file |

**Notes:** Chose the non-recommended versioning option.

## Message vocabulary

| Question | Options | Selected |
|----------|---------|----------|
| Commands | Typed per action / Generic Invoke | Typed per action |
| Queue wire format | Full snapshot + revision / Deltas + revision | Full snapshot + revision |
| Proxy scope | Any API path / Typed endpoints only | Any API path |
| Errors | Closed error enum / HTTP status + message | Closed error enum |

## Fault injection

| Question | Options | Selected |
|----------|---------|----------|
| Trigger | Control message + CLI flag / CLI flags only / Scenario file | Control message + CLI flag |
| Fidelity | Fake catalog + real clock / Static replies | Fake catalog + real clock |
| Fault shapes | Hang = stop replying, keep socket / You decide | Hang = stop replying, keep socket |
| Mock binary | Separate bin / In-process task | Separate presto-engine-mock bin |

## Seam notes and layout

| Question | Options | Selected |
|----------|---------|----------|
| Protocol doc | Hand-written + schema test / Generated from types | Hand-written + schema test |
| Token test | Walk schema for banned names / Snapshot only | Walk schema for banned names |
| Workspace | Root workspace / Flat single crate | Workspace at repo root |
| Seam depth | Seams + fetchability check / Full port inventory | Seams + fetchability check |

## Claude's Discretion

Heartbeat interval, timeout values, event set details, fault parameter syntax.

## Deferred Ideas

None.
