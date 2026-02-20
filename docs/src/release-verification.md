# Release Verification

[TOC]

This chapter explains how to verify the authenticity and integrity of ruley release artifacts.

## GitHub Attestations

All release artifacts are signed via [Sigstore](https://www.sigstore.dev/) using GitHub Attestations. This provides cryptographic proof that binaries were built by the official GitHub Actions workflow and have not been tampered with.

### Verifying with `gh`

```bash
gh attestation verify <artifact> --repo EvilBit-Labs/ruley
```

Replace `<artifact>` with the path to the downloaded binary or archive.

### What This Verifies

- The artifact was built by the `EvilBit-Labs/ruley` repository's GitHub Actions
- The build environment matches the expected workflow
- The artifact has not been modified since it was built

## SHA256 Checksums

Each release includes SHA256 checksums for all platform binaries. These are attached to the GitHub release alongside the binaries.

### Verifying Checksums

{{#tabs }} {{#tab name="macOS / Linux" }}

```bash
# Download the checksum file
curl -fsSLO https://github.com/EvilBit-Labs/ruley/releases/latest/download/sha256sums.txt

# Verify a specific artifact
sha256sum -c sha256sums.txt --ignore-missing
```

{{#endtab }} {{#tab name="Windows" }}

```powershell
# Compute the hash of the downloaded archive
Get-FileHash ruley-x86_64-pc-windows-msvc.zip -Algorithm SHA256
```

{{#endtab }} {{#endtabs }}

## crates.io Verification

When installing via `cargo install ruley`, Cargo verifies the package integrity automatically using the crates.io checksum. No additional steps are needed.

## Verifying a Cargo Install

To verify the installed version:

```bash
ruley --version
```

Compare the output with the expected version from the [releases page](https://github.com/EvilBit-Labs/ruley/releases).

## Supply Chain Security

ruley takes several measures to secure the build and release pipeline:

| Measure                 | Description                                                     |
| ----------------------- | --------------------------------------------------------------- |
| **Pinned Actions**      | All GitHub Actions are pinned to full commit SHAs               |
| **Sigstore signing**    | Artifacts signed via GitHub Attestations                        |
| **cargo-audit**         | Checks for known vulnerabilities in dependencies                |
| **cargo-deny**          | Checks license compliance and duplicate dependencies            |
| **CodeQL**              | Static analysis for security vulnerabilities                    |
| **OSSF Scorecard**      | Automated security posture monitoring                           |
| **Dependabot**          | Automated dependency update PRs                                 |
| **Reproducible builds** | Pinned Rust toolchain via `rust-toolchain.toml` and `mise.toml` |
| **Committed lock file** | `Cargo.lock` is committed for deterministic builds              |
