# Security

[TOC]

This chapter covers ruley's security model, vulnerability reporting, and security features.

## Reporting Vulnerabilities

**Do not report security vulnerabilities through public GitHub issues.**

Use one of the following channels:

1. [GitHub Private Vulnerability Reporting](https://github.com/EvilBit-Labs/ruley/security/advisories/new) (preferred)
2. Email [support@evilbitlabs.io](mailto:support@evilbitlabs.io) encrypted with the project's [PGP key](https://github.com/EvilBit-Labs/ruley/blob/main/SECURITY.md#pgp-key)

Please include:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

See [SECURITY.md](https://github.com/EvilBit-Labs/ruley/blob/main/SECURITY.md) for full policy details including scope, response times, safe harbor provisions, and the PGP key.

## Security Features

### Code Safety

- **`unsafe_code = "deny"`** enforced at the package level
- Pure Rust implementation with no C dependencies in core logic
- Zero `unwrap()` and `panic!()` in production code (enforced via clippy lints)

### Credential Handling

- API keys are read from environment variables at runtime
- Keys are never stored in generated output files
- Keys are never logged or included in error messages
- No credential persistence between runs

### Network Security

- **No network listening**: ruley makes outbound-only connections
- Connections are made only to configured LLM provider APIs
- HTTPS is used for all API calls
- No telemetry or analytics

### Supply Chain Security

- GitHub Actions pinned to full commit SHAs
- `cargo audit` runs in CI to check for known vulnerabilities
- `cargo deny` checks license compliance and duplicate dependencies
- CodeQL analysis on every PR
- OSSF Scorecard monitoring
- Automated dependency updates via Dependabot

## Scope

### In Scope

- API key or credential leakage through error messages, logs, or generated output
- Command injection via CLI arguments or configuration files
- Path traversal in file input/output handling
- Prompt injection affecting output integrity
- Denial of service via crafted input files or configuration
- Unsafe handling of LLM responses (e.g., writing to unintended paths)

### Out of Scope

- Vulnerabilities in upstream LLM providers (Anthropic, OpenAI, etc.)
- Issues requiring physical access to the machine
- Social engineering attacks
- LLM hallucinations or inaccurate generated rules (quality issue, not security)

## Response Timeline

This is a volunteer-maintained project. Response times are best-effort:

- **Acknowledgment**: Within 1 week
- **Initial assessment**: Within 2 weeks
- **Fix release**: Within 90 days of confirmed vulnerabilities
- **Disclosure**: Coordinated through GitHub Security Advisories
