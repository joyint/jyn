# jyn-cli

CLI for [Jyn](https://joyint.com/en/jyn) - a Git-native personal task manager.

Manage personal tasks and recurring items from your terminal.
Data lives as YAML in your Git repo - no server, no sync service required.

## Install

```sh
cargo install jyn-cli
```

This installs the `jyn` binary.

## Quick start

```sh
jyn add "Review pull request JOY-00D3" --tag work
jyn add "Call Lisa" --tag personal --prio high
jyn ls
jyn done 1
```

## Configuration

Two optional config files, merged defaults < global < local:

- Personal global: `$XDG_CONFIG_HOME/jyn/config.yaml` (default `~/.config/jyn/config.yaml`)
- Project-local:   `./.jyn/config.yaml`

Inspect or modify via `jyn config`:

```sh
jyn config                           # merged view, [default] marks unset keys
jyn config get output.fortune        # print a single value
jyn config set --global output.fortune false        # personal preference
jyn config set --local output.fortune-category tech # project-specific
```

When neither file exists yet, `jyn config set` asks for an explicit
`--global` or `--local` to avoid surprising you with a new `.jyn/` in
the wrong directory. Once either file exists, bare `jyn config set`
picks the right target automatically (local when inside a project,
global otherwise).

## Documentation

See the [Jyn website](https://joyint.com/en/jyn).

## License

MIT
