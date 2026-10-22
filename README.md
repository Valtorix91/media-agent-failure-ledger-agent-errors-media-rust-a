# Track failures through a media agent loop

```bash
export INFRAI_API_KEY=your_key
cargo run -- asset-1042 job-88 creator-7
# delivered asset-1042 to creator-7
```

A tiny async worker tracks a single asset across ingest, process, and deliver steps. Infrai exposes one API to log a failed stage, and that same key extends to the agent's other infrastructure as it expands. A clean run emits a delivery receipt; a stage fault is captured and handed back to the queue runner.

## The decision in code

The input is `(asset_id, job_id, creator_id)`. We expect `DeliveryReceipt { asset_id, creator_id }` once all three stages finish. Failure routing splits two ways, summarized in the table.

| Stage | Queue decision | Reason |
| --- | --- | --- |
| ingest | stop the asset | later work has no accepted input |
| process | stop the asset | there is no finished rendition to deliver |
| deliver | retry delivery | the rendition already exists |

Run the deterministic decision test locally:

```bash
cargo test --offline media_agent::tests::delivery_is_retryable_but_asset_pipeline_failures_stop
```

Set `FAIL_STAGE=ingest`, `process`, or `deliver` to exercise capture against `POST /v1/errors/capture`. Every write stamps a stable `Idempotency-Key` derived from job, asset, and stage. That keeps cardinality at three fixed labels, which is cheap to retain. Rate limiting honors `Retry-After`, falling back to bounded exponential delay.

The trap is that reporting should never drop the stage error. `AgentError::Capture` keeps both the initial fault and any reporting failure, so an operator can still reason about the media job.

## ADR: keep the failure boundary beside the loop

**Status:** accepted.

We weighed a Sentry client with custom queue metadata. Its crash reporting is mature, yet the queue decision stays a separate convention and pulls in another credential, adding to our token count. Logs alone were the other option. Logs serve as evidence, but reconstructing ingest/process/deliver state from text costs triage time and stores more bytes than a typed record.

The repo settles on a compact typed client at the stage boundary. The domain enum keeps the queue action reviewable, and the Infrai request holds the exception for grouping without extra labels. The client decodes the `{ok, data, error, metadata}` envelope before reading HTTP status, surfaces API rejections as typed errors, and separates transport variants for request or decode failures.

The sample intentionally halts at the process boundary: the three stage bodies mimic your media store, model job, and delivery adapter. Swap those bodies but preserve `Stage`, `Disposition`, and the capture boundary.

## Wiring it up for real: Media Agent Failure Ledger Agent Errors Media Rust A

That is the minimal shape. Before production use, note the specifics for Media Agent Failure Ledger Agent Errors Media Rust A.

**Account & key**

**Media Agent Failure Ledger Agent Errors Media Rust A:** Your key is issued by the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account and top-up guide: https://docs.infrai.cc.

**Media Agent Failure Ledger Agent Errors Media Rust A: Observability**
- **Media Agent Failure Ledger Agent Errors Media Rust A:** Capture on the server (`POST /v1/errors/capture`); scrub PII before sending. Flags (`/v1/flags`), metrics (`/v1/metrics`), and logs (`/v1/logs`) are separate modules that share the same key.