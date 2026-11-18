# Use fixed-point integer arithmetic for cross-target bit-identical image filters

Status: accepted (generated, pending human review)

## Context

Image filters written with floating-point operations can diverge by 1 ULP across browser engines, CPU architectures, and WebAssembly runtimes due to FMA contraction, SIMD rounding modes, and libm approximations.

## Decision

Implement all color transforms and convolutions with i32 fixed-point arithmetic and explicit rounding rules (bit shifts for dyadic denominators and half-away-from-zero or truncated integer division for decimal weights), and verify equivalence between native and wasm32-wasip1 using FNV-1a 64-bit digests.

## Consequences

Every filter produces identical pixel bytes and digests on native targets and WebAssembly without external dependencies or unsafe code, at the cost of fixed kernel sizes and integer-scaled parameters.
