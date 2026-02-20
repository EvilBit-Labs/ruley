# Security Hardening

## Description

Analyze diff for security posture, apply safe internal hardening edits, produce report.

Analyze ONLY changed files (diff scope) for security posture and apply clearly safe hardening improvements while preserving all public APIs.

## FOCUS CATEGORIES

01. Input Validation & Parsing (config, CLI, paths) - reject invalid early, no silent defaults
02. Data Handling (no secrets logged, path canonicalization, no sensitive data in error messages)
03. API Key & Credential Safety (no hardcoded keys, environment variable handling, masking in logs)
04. Error Handling & Logging Hygiene (no sensitive leakage, structured context, no println! for operational info)
05. Dependency & Surface Minimization (avoid unnecessary crates/features, dead code removal)
06. Defense-in-Depth Opportunities (bounds checking, resource ceilings, input size limits)
07. Security Regression Risks (stubs flagged, TODOs categorized, unimplemented sections clearly documented)
08. Supply Chain & Build Hygiene (forbid unsafe, clippy -D warnings, deny unknown features)
09. Concurrency Safety (ordering, timeouts, backpressure)
10. Network Safety (TLS validation, URL handling, redirect following)

## Steps

1 Diff list -> 2 Security analysis per category -> 3 Classify findings (`safe-edit` / `deferred` / `requires-approval`) -> 4 Apply only mechanical non-breaking hardening edits (logging normalization, path canonicalize + bound checks, converting println!/eprintln! to tracing, adding missing error context) -> 5 Run `just lint` & `just test` -> 6 Revert any failing hunk -> 7 Report (summary, applied, deferred, approval-needed, risk notes, roadmap) -> 8 Output unified diff (no commit).

If zero safe edits: state "No safe security edits applied" and still emit full report.

## SAFE HARDENING EDIT EXAMPLES

- Replace `println!/eprintln!` with `tracing::{info,warn,error}!`
- Add `tracing::instrument` to sensitive boundaries without changing signatures
- Inline guard clauses for obvious panics or unchecked unwraps (if internal)
- Canonicalize paths then verify prefix containment
- Remove dead code exposing potential attack surface
- Strengthen error messages (no raw API keys or system paths if sensitive)
- Add length / size / iteration bounds for unbounded growth structures
- Replace stringly-typed mode flags with private enums
- Ensure all public API doc comments mention security considerations where relevant

## AUTO-EDIT CONSTRAINTS (STRICT)

Scope: diff-only | Gates: `just lint` + tests must pass | No commits | No public signature/visibility changes | Validate after edits

## CRITICAL REQUIREMENTS

- Preserve functional behavior while reducing risk
- No new dependencies unless strictly necessary for safety
- Avoid speculative rewrites - minimal surface change
- Avoid perf regressions; if added checks are non-trivial mark as deferred
- Do not mask existing errors - surface with context instead

## REPO RULES (REINFORCED)

Zero warnings | No unsafe | Precise typing | Async I/O | Trait-based providers | thiserror+anyhow | CLI-first | Memory efficiency | rustdoc for public APIs

## EXECUTION CHECKLIST

1 Diff scan 2 Analyze security 3 Classify 4 Apply safe hardening edits 5 Gates pass 6 Report 7 Output diff | On blocker: report with remediation.

## QUICK SECURITY MATRIX

Category -> Sample Safe Edit:

- Input Validation -> Add numeric range check before use
- Data Handling -> Canonicalize + ensure path within data root
- API Key Safety -> Mask key in error/log output
- Logging -> Replace raw error chain with redacted display
- Resource Bounds -> Add comment + bound to vector growth pattern
- Stub Sections -> Mark with `SECURITY_TODO:` prefix for tracking

Ambiguous? Defer and document.

## Completion Checklist

- [ ] Code conforms to ruley project rules and standards (AGENTS.md)
- [ ] Tests pass (`just test`)
- [ ] Linting is clean (`just lint`)
- [ ] Full CI validation passes (`just ci-check`)
- [ ] A short summary of what was done is reported
