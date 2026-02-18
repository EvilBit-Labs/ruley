# Pull Request

## Description

<!-- Provide a clear and concise description of the changes -->

## Type of Change

- [ ] **Bug fix** (non-breaking change which fixes an issue)
- [ ] **New feature** (non-breaking change which adds functionality)
- [ ] **Breaking change** (fix or feature that would cause existing functionality to not work as expected)
- [ ] **Documentation update** (changes to documentation only)
- [ ] **Refactoring** (no functional changes, code improvements)
- [ ] **Performance improvement** (improves performance without changing functionality)
- [ ] **Test addition/update** (adding or updating tests)
- [ ] **Build/CI change** (changes to build system or CI configuration)

## Related Issues

<!-- Link to any related issues using keywords like "Closes #123", "Fixes #456", "Addresses #789" -->

- Closes #
- Fixes #
- Addresses #

## Testing

### Pre-submission Checklist

- [ ] **Code Quality**: Code passes `cargo fmt --check`
- [ ] **Linting**: Zero warnings from `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] **Tests**: All tests pass with `cargo test --all-features`
- [ ] **Error Handling**: Proper error handling with context implemented
- [ ] **Documentation**: New public APIs documented with rustdoc
- [ ] **Dependencies**: Dependencies properly managed (`cargo update` if needed)
- [ ] **Security**: No hardcoded secrets or credentials
- [ ] **Input Validation**: Input validation implemented where needed

### Test Commands Executed

```bash
# Format and lint
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all-features

# Comprehensive validation
just ci-check
```

### Test Results

<!-- Provide test output or summary -->

## Changes Made

### Files Modified

<!-- List the main files that were changed -->

### Key Changes

<!-- Describe the key changes made -->

## Review Checklist

### For Reviewers

- [ ] **Correctness**: Code does what it claims, edge cases handled
- [ ] **Safety**: No unsafe code, proper error handling
- [ ] **Architecture**: Changes align with project architecture patterns
- [ ] **Security**: No security vulnerabilities introduced
- [ ] **Performance**: No performance regressions
- [ ] **Documentation**: Documentation updated if needed
- [ ] **Testing**: Adequate test coverage provided

### For Contributors

- [ ] **Self Review**: Code has been self-reviewed
- [ ] **Commit Messages**: Follow conventional commit format
- [ ] **Branch Naming**: Branch follows naming convention (`feat/`, `fix/`, `docs/`, etc.)
- [ ] **Scope**: Changes are focused and not too broad
- [ ] **Dependencies**: No unnecessary dependencies added
- [ ] **DCO**: Commits are signed off (`git commit -s`)

## Breaking Changes

<!-- If this PR includes breaking changes, document them here -->

### Migration Guide

<!-- Provide migration steps if breaking changes are introduced -->

## Security Considerations

<!-- Document any security implications -->

## Additional Notes

<!-- Any additional information that reviewers should know -->

---

**By submitting this pull request, I confirm that:**

- [ ] I have read and followed the [Contributing Guide](CONTRIBUTING.md)
- [ ] My code follows the project's coding standards
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] I have updated the documentation accordingly
- [ ] My changes generate no new warnings
