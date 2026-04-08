# Conta

Back to 3000 years ago in Zhou dynasty, the emperor of Zhou rides carriage
with 6 horses, now, ride your crates with 6 horses like the emperor!

```bash
Modern tool for bumping crate versions and publishing them

Usage: conta [OPTIONS] <COMMAND>

Commands:
  version  Bump versions
  publish  Publish crates
  help     Print this message or the help of the given subcommand(s)

Options:
  -m, --manifest <MANIFEST>  The path of the cargo manifest, if not provided, the current directory is used
  -c, --config <CONFIG>      The path of `Conta.toml`
  -h, --help                 Print help
```

## Publishing

`conta publish` walks every crate in the workspace, figures out the
correct publish order from `cargo metadata`, and runs `cargo publish` on
each one — skipping crates whose current version is already on
crates.io.

To exclude workspace crates from publishing (examples, xtask, internal
tools, etc.), list them under `[workspace.metadata.conta]` in the root
`Cargo.toml`:

```toml
[workspace.metadata.conta]
ignore = ["xtask", "examples-foo"]
```

If a non-ignored crate depends on an ignored one, `conta publish`
exits non-zero before uploading anything and names the offending edge.

`--dry-run` prints the resolved plan (`<crate>@<version> would
publish` or `... already published, skipping`) without invoking
`cargo publish`. It still hits crates.io to determine which crates
are already published — a dry run is not an offline run.

## LICENSE

GPL-3.0-only
