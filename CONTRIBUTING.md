# Contributing

Thanks for your interest in improving mars-xlog.

## Development
- Format Rust code with `cargo fmt` and address warnings from `cargo clippy`.
- Run tests with `cargo test` where applicable.
- If you touch native build logic, mention the toolchains you used in the PR.

## Updating the Mars subtree
This repo vendors Tencent Mars via git subtree at `third_party/mars`.
Use the following to update it:

```bash
git subtree pull --prefix third_party/mars https://github.com/Tencent/mars.git master --squash
```

## Reporting issues
Please include reproduction steps, platform/toolchain details, and logs.
