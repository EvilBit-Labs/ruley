# Releasing ruley

This document describes the process for creating a new release of ruley.

## Overview

Releases are automated via [cargo-dist](https://opensource.axo.dev/cargo-dist/) and GitHub Actions. Pushing a version tag (e.g., `v1.0.0`) triggers the release workflow, which builds binaries for all supported platforms, generates a changelog, creates a GitHub release, publishes to crates.io, and updates the Homebrew tap. Configuration lives in `dist-workspace.toml`.

## Pre-Release Checklist

Before creating a release, verify the following:

- [ ] All tests pass locally: `just ci-check`
- [ ] Zero clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Documentation is up to date (README.md, CHANGELOG.md)
- [ ] Review open issues and PRs for release blockers
- [ ] Release build succeeds: `cargo build --release`
- [ ] Binary works correctly: `./target/release/ruley --help`
- [ ] Dry-run crates.io publish: `cargo publish --dry-run --all-features`

## Version Bump Process

1. Update the version in `Cargo.toml`:

   ```toml
   version = "X.Y.Z"
   ```

2. Run `cargo update` to update `Cargo.lock` with the new version.

3. Generate the changelog:

   ```bash
   just changelog
   ```

4. Review and edit `CHANGELOG.md` for the new version entry.

5. Commit all changes:

   ```bash
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "chore(release): prepare for vX.Y.Z"
   ```

## Tag Creation and Push

1. Create an annotated tag:

   ```bash
   git tag -a vX.Y.Z -m "Release vX.Y.Z"
   ```

2. Push the tag to trigger the release workflow:

   ```bash
   git push origin vX.Y.Z
   ```

   GitHub Actions will automatically:

   - Validate the tag version matches `Cargo.toml`
   - Build binaries for all 5 platform targets
   - Generate SHA256 checksums
   - Create a GitHub release with changelog and binaries
   - Publish to crates.io (non-prereleases only)

## Release Verification

After the workflow completes:

1. **GitHub Actions**: Monitor the workflow run at `.github/workflows/release.yml`
2. **Binaries**: Verify all 5 platform binaries are attached to the GitHub release
3. **Checksums**: Confirm SHA256 checksums are present
4. **crates.io**: Verify the package is published at `https://crates.io/crates/ruley`
5. **Install test**: Run `cargo install ruley` to verify end-to-end

### Platform Targets

| Platform              | Target                      | Archive   |
| --------------------- | --------------------------- | --------- |
| Linux x86_64          | `x86_64-unknown-linux-gnu`  | `.tar.gz` |
| Linux x86_64 (static) | `x86_64-unknown-linux-musl` | `.tar.gz` |
| Linux ARM64           | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| macOS ARM64           | `aarch64-apple-darwin`      | `.tar.gz` |
| Windows x86_64        | `x86_64-pc-windows-msvc`    | `.zip`    |

## Post-Release Tasks

- [ ] Announce the release on GitHub Discussions (if applicable)
- [ ] Update project status in README.md (if milestone changed)
- [ ] Close the release milestone (if applicable)
- [ ] Create the next milestone for upcoming work

## Rollback Procedure

If a release needs to be rolled back:

1. **Delete the tag locally**:

   ```bash
   git tag -d vX.Y.Z
   ```

2. **Delete the tag remotely**:

   ```bash
   git push origin :refs/tags/vX.Y.Z
   ```

3. **Delete the GitHub release** via the web interface (Releases page).

4. **Yank from crates.io** (if published):

   ```bash
   cargo yank --version X.Y.Z
   ```

   Note: Yanking prevents new installs but does not remove the package. Existing `Cargo.lock` files that reference this version will still work.

## Troubleshooting

### CI Failures

- Check the workflow logs in GitHub Actions
- Ensure all tests pass locally with `just ci-check`
- Verify the tag version matches `Cargo.toml` exactly

### Build Failures

- Cross-compilation issues: Check that the target toolchain is installed in CI
- Dependency issues: Run `cargo update` and retry

### Publish Failures

- Verify the `CARGO_REGISTRY_TOKEN` secret is set in GitHub repository settings
  - Obtain a token from <https://crates.io/me>
  - Add to: Settings > Secrets and variables > Actions > New repository secret
  - Name: `CARGO_REGISTRY_TOKEN`
- Ensure the version has not already been published to crates.io
- Run `cargo publish --dry-run --all-features` locally to check for issues

### Changelog Issues

- Review the git-cliff configuration in `cliff.toml`
- Ensure commit messages follow conventional commit format
- Run `just changelog` locally to preview the output

## Prerelease Versions

For release candidates or beta releases:

1. Use a prerelease tag: `v1.0.0-rc.1`, `v1.0.0-beta.1`
2. The release workflow will mark these as prereleases on GitHub
3. Prerelease versions are **not** published to crates.io automatically
