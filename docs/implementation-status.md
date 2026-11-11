# Implementation status — 0.0.1 baseline

The technical development directive supplied on 11 November 2026 is the
proposed product contract. This repository begins implementation; this status
file does not replace or claim to reproduce the full directive.

## Implemented in this baseline

- Local Rust library and JSON CLI with `inspect`, `suggest`, `apply`, and
  `policy validate` operations.
- Strict request and policy parsing, a 64 KiB request envelope bound, a 16 KiB
  message bound, and explicit no-op decisions when no qualified policy exists.
- No-op output preserves the message bytes, reports zero edits, and does not
  call an external provider or store request text.
- An ephemeral keyed digest avoids publishing a reusable raw SHA-256 digest
  for a low-entropy message.
- Candidate-policy validation cannot activate a policy and accepts only a
  no-op edit in this baseline.
- Offline free-route admission checks require exact allowlisted IDs, fresh
  catalogue expiry, explicit zero pricing across required categories, denied
  data collection, compatible terms, and bounded retention for every fallback.
- The admission layer makes no provider requests; live catalogue retrieval,
  quota reservations, usage reconciliation, and transport fault handling remain.
- Unit and CLI tests, a strict Deliver spec, and Brandi-generated project
  brief files.

## Gate disposition

| Directive gate | Current disposition | Remaining evidence |
|---|---|---|
| DG-0 scope | Partial | Accountable owners, data classification, external revisions, and capability inventory are not assigned. |
| DG-1 contracts | Partial | Only local request/decision/policy contracts are implemented; outcome, work, claim, capability, and compatibility contracts remain. |
| DG-2 preservation | Partial | Tested no-op preservation only; segmentation, protected-span fixtures, and reviewed transformations remain. |
| DG-3 broker | Partial | Offline free-route admission is implemented; live catalogue retrieval, exact OpenRouter controls, quota reservations, transport retries, and usage reconciliation remain. |
| DG-4 integrations | Blocked | Deliver is used as a local gate; no adapter conformance against a pinned Deliver or Brandi executable. |
| DG-5 host integrity | Blocked | No sealed grader or experimental host. |
| DG-6 security/reliability | Blocked | No tenant/session store, migrations, backup/restore, or 24-hour soak. |
| DG-7 benefit | Inconclusive | No experiment was run and no benefit claim is permitted. |
| DG-8 Brandi | Partial | Generated brief is present; complete surface inventory, claim validation, and accessibility evidence remain. |
| DG-9 release | Blocked | No signed manifest, rollback rehearsal, or release owner. |
| DG-10 operation | Blocked | No deployed canary or revocation exercise. |

No blocker above is converted to a pass by this baseline. `uni` is a separate
engineering snapshot and cannot satisfy project acceptance gates.
