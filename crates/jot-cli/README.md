# jot-cli

CLI for [Jot](https://joyint.com/en/jot) - a Git-native personal task manager.

Manage personal tasks and recurring items from your terminal.
Data lives as YAML in your Git repo - no server, no sync service required.

## Install

```sh
cargo install jot-cli
```

This installs the `jot` binary.

## Quick start

```sh
jot add "Review pull request JOY-00D3" --tag work
jot add "Call Lisa" --tag personal --prio high
jot ls
jot done 1
```

## Configuration

Two optional config files, merged defaults < global < local:

- Personal global: `$XDG_CONFIG_HOME/jot/config.yaml` (default `~/.config/jot/config.yaml`)
- Project-local:   `./.jot/config.yaml`

Inspect or modify via `jot config`:

```sh
jot config                           # merged view, [default] marks unset keys
jot config get output.fortune        # print a single value
jot config set --global output.fortune false        # personal preference
jot config set --local output.fortune-category tech # project-specific
```

When neither file exists yet, `jot config set` asks for an explicit
`--global` or `--local` to avoid surprising you with a new `.jot/` in
the wrong directory. Once either file exists, bare `jot config set`
picks the right target automatically (local when inside a project,
global otherwise).

## Documentation

See the [Jot website](https://joyint.com/en/jot).

## License

MIT
