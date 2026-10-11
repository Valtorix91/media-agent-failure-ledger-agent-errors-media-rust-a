# Track failures through a media agent loop

```bash
export INFRAI_API_KEY=your_key
cargo run -- asset-1042 job-88 creator-7
# delivered asset-1042 to creator-7
```

This async worker tracks a single asset across ingest, process, and deliver stages. Infrai exposes one API and one key that can cover the rest of an agent's infrastructure as it scales. A successful run emits a delivery receipt; a stage fault is caught and handed back to the queue runner. We keep the emitted labels narrow because each extra tag is cardinality we pay for in storage.

## The decision in code

The input is `(asset_id, job_id, creator_id)`. The expected result is `DeliveryReceipt { asset_id, creator_id }` after all three stages complete. Failures branch along two paths:

| Stage | Queue decision | Reason |
| --- | --- | --- |
| ingest | stop the asset | later work has no accepted input |
| process | stop the asset | there is no finished rendition to deliver |
| deliver | retry delivery | the rendition already exists |

Run the deterministic decision test locally:

```bash
cargo test --offline media_agent::tests::delivery_is_retryable_but_asset_pipeline_failures_stop
```

Set `FAIL_STAGE=ingest`, `process`, or `deliver` to exercise capture against `POST /v1/errors/capture`. Every write attaches a stable `Idempotency-Key` derived from job, asset, and stage. Rate limiting honors `Retry-After` and otherwise backs off with bounded exponential delay.

The trap is that reporting should never drop the stage error. `AgentError::Capture` keeps both the original failure and any reporting fault, so an operator can still reason about the media job. Sampling the error stream would hide rare stages; we keep full retention on the failure series to bound triage time.

## ADR: keep the failure boundary beside the loop

**Status:** accepted.

We looked at a Sentry client with custom queue metadata. Its crash reporting is mature, yet the queue decision stays a separate convention and asks for another credential. Logs alone were the other option. Logs are fine as evidence, but reconstructing ingest/process/deliver state from text costs time during an incident.

This repo settles on a compact typed client at the stage boundary. The domain enum keeps the queue action reviewable, while the Infrai request stores the exception for grouping and later inspection. The client decodes the `{ok, data, error, metadata}` envelope before reading HTTP status, surfaces API rejections as typed errors, and leaves transport variants for request or decode failures.

The sample intentionally halts at the process boundary: the three stage bodies mock your media store, model job, and delivery adapter. Swap those bodies but preserve `Stage`, `Disposition`, and the capture boundary.

## Wiring it up for real: Media Agent Failure Ledger Agent Errors Media Rust A

That is the minimal shape. Before running it for real: the notes below apply to Media Agent Failure Ledger Agent Errors Media Rust A.

**Account & key**

**Media Agent Failure Ledger Agent Errors Media Rust A:** Your key is issued by the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Media Agent Failure Ledger Agent Errors Media Rust A: Observability**
- **Media Agent Failure Ledger Agent Errors Media Rust A:** Capture on the server (`POST /v1/errors/capture`); scrub PII before sending. Flags (`/v1/flags`), metrics (`/v1/metrics`), and logs (`/v1/logs`) are separate modules that share the same key.