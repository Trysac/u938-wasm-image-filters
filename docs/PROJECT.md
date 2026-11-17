# Project record

## Objective

Integer-only RGBA image filters that give bit-identical results natively and in WebAssembly, with Gaussian blur, Sobel edges, sepia, contrast and a filter pipeline

## Scope

- In scope: the public API documented in [README.md](../README.md), documented and tested.
- Out of scope: anything that needs dependencies or I/O without an accepted ADR.

## Timeline

| Date | Event |
|---|---|
| 2026 | Repository created from the factory utility template. |

## Effort metrics

Measured values only; a value that has not been measured stays "pending measurement".

| Metric | Value |
|---|---|
| Tokens (input / output) | 1494205 / 26646 |
| Hours (human) | pending measurement |
| Cost (USD) | 0.7280 |

## Factory run u938 (measured from API usage)

| Date (UTC) | Phase | Model | Batch | Input | Output | Cache write | Cache read | Cost (USD) |
|---|---|---|---|---|---|---|---|---|
| 2026-11-17 | generation | google/gemini-4-argon | yes | 6425 | 18827 | 0 | 0 | 0.4022 |
| 2026-11-17 | fix-1 | google/gemini-3.8-flash | no | 116316 | 4985 | 0 | 618996 | 0.1524 |
| 2026-11-17 | fix-2 | google/gemini-3.8-flash | no | 37048 | 502 | 0 | 139702 | 0.0401 |
| 2026-11-17 | audit | google/gemini-3.8-flash | no | 119614 | 846 | 0 | 433645 | 0.1254 |
| 2026-11-17 | docs | google/gemini-3.1-flash-lite | no | 22459 | 1486 | 0 | 0 | 0.0078 |

Total: $0.7280. Prices: vertex line, as measured by its adapter. Providers serve the current version of each model id, so the date tells which version answered; versions, launch stages and price sources per line are in the factory's model register (docs/MODELS.md).
