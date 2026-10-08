# actiontui

A terminal UI to browse GitHub Actions: workflows, runs, jobs, steps and their logs, without leaving your shell.

Built with [Ratatui](https://ratatui.rs).

## Usage

Run it inside a git repository. The repo is taken from `remote.origin.url`.

```sh
cargo run --release
```

## Configuration

On first start a config file is created at `~/.config/actiontui/config.toml`:

```toml
url = "github.com"
pat = "ghp_yourtokenhere"
```

- `pat`: a personal access token with access to Actions (`repo` scope for classic tokens, *Actions: read* for fine-grained ones).
- `url`: the GitHub host. Change it for GitHub Enterprise.

## Keys

| Key | Action |
| --- | --- |
| `1`-`5` | Focus workflows / runs / jobs / steps / logs |
| `j` `k` / `↓` `↑` | Move down / up (one line in logs) |
| `J` `K` / `End` `Home` | Jump to last / first |
| `Ctrl`+`j` `k` | Scroll logs by 10 lines |
| `g` / `G` | Jump to top / bottom of the logs |
| `Enter` | Open the selected item (workflow → runs → jobs → steps → logs) |
| `r` | Refresh the current list |
| `q` / `Ctrl`+`c` | Quit |

## License

MIT, see [LICENSE](./LICENSE).
