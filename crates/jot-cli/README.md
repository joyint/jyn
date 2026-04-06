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
jot add "Buy groceries" --due tomorrow
jot add "Weekly review" --recur weekly
jot ls
jot done 1
```

## Documentation

See the [Jot website](https://joyint.com/en/jot).

## License

MIT
