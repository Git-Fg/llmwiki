## Summary

<!-- One-paragraph description of the change -->

## Type of change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that changes existing behavior)
- [ ] Documentation update
- [ ] CI/build/dependency update

## Checklist

- [ ] `cargo fmt` clean
- [ ] `cargo clippy --all-targets -- -D warnings` clean
- [ ] `cargo test` passes (all tests, including new ones)
- [ ] `cargo build --release` succeeds
- [ ] `bash tests/skill_smoke.sh` passes (if CLI surface changed)
- [ ] CHANGELOG.md updated under "Unreleased" or the next version
- [ ] Docs updated (README, AGENTS.md, or skill content) if user-facing

## Related issues

<!-- Link related issues: Fixes #123, Relates to #456 -->

## Testing

<!-- How did you verify the change? Manual steps, new tests, edge cases covered -->

## Breaking changes

<!-- If breaking: migration steps, deprecation timeline, affected callers -->
