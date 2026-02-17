# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

Users on unsupported versions should upgrade to the latest release. Please review the [release notes](https://github.com/EvilBit-Labs/ruley/releases) when upgrading.

## Reporting a Vulnerability

We take the security of ruley seriously. If you believe you have found a security vulnerability, please report it to us as described below.

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, use one of the following channels:

1. [GitHub Private Vulnerability Reporting](https://github.com/EvilBit-Labs/ruley/security/advisories/new) (preferred)
2. Email [support@evilbitlabs.io](mailto:support@evilbitlabs.io) encrypted with our [PGP key](#pgp-key) (verify the full fingerprint below before use)

Please include:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Scope

**In scope:**

- API key or credential leakage through error messages, logs, or generated output
- Command injection via CLI arguments or configuration files
- Path traversal in file input/output handling
- Prompt injection that causes unintended LLM behavior affecting output integrity
- Denial of service via crafted input files or configuration
- Unsafe handling of LLM responses (e.g., writing to unintended paths)

**Out of scope:**

- Vulnerabilities in upstream LLM providers (Anthropic, OpenAI, etc.)
- Issues requiring physical access to the machine running ruley
- Social engineering attacks
- LLM hallucinations or inaccurate generated rules (quality issue, not security)

### What to Expect

**Note**: This is a passion project with volunteer maintainers. Response times are best-effort and may vary based on maintainer availability.

- We will acknowledge receipt of your report within **1 week**
- We will provide an initial assessment within **2 weeks**
- We aim to release a fix within **90 days** of confirmed vulnerabilities
- We will coordinate disclosure through a [GitHub Security Advisory](https://github.com/EvilBit-Labs/ruley/security/advisories)
- We will credit you in the advisory (unless you prefer to remain anonymous)

### Responsible Disclosure

We ask that you:

- Give us reasonable time to respond to issues before any disclosure
- Avoid accessing or modifying other users' data
- Avoid actions that could negatively impact other users

## Security Features

ruley includes several security-focused features:

- **Pure Rust implementation**: `unsafe_code = "deny"` enforced at the package level
- **No network listening**: Outbound-only connections to configured LLM providers
- **Credential isolation**: API keys read from environment variables, never stored in generated output
- **Controlled environment**: CLI tests use `env_clear()` to prevent test pollution
- **Dependency auditing**: Regular `cargo audit` and `cargo deny` checks in CI
- **Supply chain security**: GitHub Actions pinned to commit SHAs, CodeQL and OSSF Scorecard in CI
- **Automated dependency updates**: Via Dependabot

## Safe Harbor

We support safe harbor for security researchers who:

- Make a good faith effort to avoid privacy violations, data destruction, and service disruption
- Only interact with accounts you own or with explicit permission of the account holder
- Report vulnerabilities through the channels described above

We will not pursue legal action against researchers who follow this policy.

## PGP Key

**Fingerprint:** `F839 4B2C F0FE C451 1B11 E721 8F71 D62B F438 2BC0`

```text
-----BEGIN PGP PUBLIC KEY BLOCK-----

mDMEaLJmxhYJKwYBBAHaRw8BAQdAaS3KAoo+AgZGR6G6+m0wT2yulC5d6zV9lf2m
TugBT+O0L3N1cHBvcnRAZXZpbGJpdGxhYnMuaW8gPHN1cHBvcnRAZXZpbGJpdGxh
YnMuaW8+iNcEExYKAH8DCwkHRRQAAAAAABwAIHNhbHRAbm90YXRpb25zLm9wZW5w
Z3Bqcy5vcmexd21FpCDfIrO7bf+T6hH/8drbGLWiuEueWvSTyw4T/QMVCggEFgAC
AQIZAQKbAwIeARYhBPg5Syzw/sRRGxHnIY9x1iv0OCvABQJpiUiCBQkIXQE5AAoJ
EI9x1iv0OCvAm2sA/AqFT6XEULJCimXX9Ve6e63RX7y2B+VoBVHt+PDaPBwkAP4j
39xBoLFI6KZJ/A7SOQBkret+VONwPqyW83xfn+E7Arg4BGiyZsYSCisGAQQBl1UB
BQEBB0ArjU33Uj/x1Kc7ldjVIM9UUCWMTwDWgw8lB/mNESb+GgMBCAeIvgQYFgoA
cAWCaLJmxgkQj3HWK/Q4K8BFFAAAAAAAHAAgc2FsdEBub3RhdGlvbnMub3BlbnBn
cGpzLm9yZ4msIB6mugSL+LkdT93+rSeNePtBY4Aj+O6TRFU9aKiQApsMFiEE+DlL
LPD+xFEbEechj3HWK/Q4K8AAALEXAQDqlsBwMP2XXzXDSnNNLg8yh1/zQcxT1zZ1
Z26lyM7L6QD+Lya5aFe74WE3wTys5ykGuWkHYEgba+AyZNmuPhwMGAc=
=9zSi
-----END PGP PUBLIC KEY BLOCK-----
```

## Contact

For general security questions, open a GitHub Issue. For vulnerability reports, use [Private Vulnerability Reporting](https://github.com/EvilBit-Labs/ruley/security/advisories/new) or email [support@evilbitlabs.io](mailto:support@evilbitlabs.io).

---

Thank you for helping keep ruley and its users secure!
