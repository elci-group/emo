# emo

**emo — evidence-driven prompt modulation for sustained agent work.**

emo is an early local Rust library and CLI baseline. It defaults to an
unchanged-message decision because no transformation policy has passed the
preservation or benefit gates. It does not claim that politeness improves
agent performance.

## Use

Build and test offline with Rust:

```sh
cargo test --locked --offline
```

`emo inspect --root PROJECT --format json` reports local capabilities.
`emo suggest --root PROJECT --format json` and
`emo apply --root PROJECT --format json` read one JSON request from standard
input and emit one JSON decision. They currently return the original message
unchanged with `NO_QUALIFIED_POLICY`; neither command invokes a model or
persists request text. This explicit no-op is a completed decision, not
evidence of benefit. `emo policy validate policies/noop.v1.json --root
PROJECT --format json` validates a bounded policy file inside the named
project root; validation does not qualify or activate it.

Example:

```json
{"schema":"emo.request.v1","request_id":"req-1","session_id":"session-1","message":"Do not deploy."}
```

The CLI accepts at most 64 KiB of request JSON, and the message itself is
limited to 16 KiB of UTF-8 bytes. The decision includes a per-process keyed
message digest. The key is generated from the operating system random source
and is not persisted, so digests cannot be used to correlate messages between
invocations. The message is returned to
the caller in the decision output; callers are responsible for handling it.

## Current limits

This is an engineering scaffold, not the completed directive. It has no
provider transport or live catalogue client, session store, service, tenant isolation, adaptive policy
registry, Deliver or Brandi adapter, sealed evaluator, or demonstrated
behavioural benefit. The local CLI accepts only the minimal request shape
implemented in this release. See [implementation status](docs/implementation-status.md)
for the gate-by-gate scope.

The generated `.brandi/` brief describes presentation preferences only; it
does not validate factual claims. Do not publish a performance claim based on
this repository state.
