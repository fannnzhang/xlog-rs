# Contributing

Thanks for your interest in improving mars-xlog.

## Development
- Format Rust code with `cargo fmt` and address warnings from `cargo clippy`.
- Run tests with `cargo test` where applicable.
- If you touch native build logic, mention the toolchains you used in the PR.

## Updating the Mars submodule
This repo uses Tencent Mars as a git submodule at `third_party/mars`.
Use the following to update it:

```bash
git -C third_party/mars fetch
git -C third_party/mars checkout <tag-or-commit>
git add third_party/mars
```

## Reporting issues
Please include reproduction steps, platform/toolchain details, and logs.
