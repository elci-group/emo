# Implementation baseline

This repository implements the initial local slice of **emo technical
development directive v0.0.1 (EMO-TDD-001, 11 November 2026)** supplied for
this work. The directive remains the governing product contract; the complete
source text was supplied in the development conversation. This file records
the scope admitted into the first code increment.

The initial slice is a local Rust library and CLI, strict JSON request and
decision handling, policy validation, a no-op default, project documentation,
Brandi brief files, and deterministic local acceptance gates. It does not
claim engineering readiness for the full directive, experimental readiness,
or behavioural benefit. See `../implementation-status.md` for current gate
dispositions and missing evidence.

## Development and publication

The project-local Kaptaind configuration requires offline tests before a
commit. Push is enabled only for the explicitly requested `elci-group/emo`
remote. Development commits are made through Kaptaind after Deliver and
`uni --fail-under 80 .` checks; a failed or unavailable gate is reported
rather than represented as passing.
