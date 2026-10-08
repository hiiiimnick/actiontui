# actiontui

A terminal UI to browse GitHub Actions: workflows, runs, jobs, steps and their logs, without leaving your shell.

Built with [Ratatui](https://ratatui.rs).

## Usage

Run it inside a git repository. The repo is taken from `remote.origin.url`.

```sh
cargo run --release
cargo run --release -- --profile work   # pick the account explicitly
```

## Configuration

On first start a config file is created at `~/.config/actiontui/config.toml`.
It can hold any number of GitHub accounts (profiles), for several hosts:

```toml
default_profile = "work"        # used when several profiles match

[[profiles]]
name = "personal"
url = "github.com"
pat = "ghp_yourtokenhere"

[[profiles]]
name = "work"
url = "github.example.com"      # GitHub Enterprise Server
owner = "acme"                  # optional: only for repositories of this owner
pat_env = "WORK_GITHUB_TOKEN"   # read the token from this environment variable
```

| Field | Meaning |
| --- | --- |
| `name` | Name of the profile, shown at the bottom right and used by `--profile` |
| `url` | The GitHub host, `github.com` by default. A scheme or a path (e.g. an organization) is ignored, use `owner` to limit a profile to an organization |
| `pat` | A personal access token (`repo` scope for classic tokens, *Actions: read/write* for fine-grained ones) |
| `pat_env` | Name of an environment variable holding the token. Wins over `pat` |
| `owner` | Optional. The profile only applies to repositories of this owner and beats a profile without `owner` |
| `api_url` | Optional. Base URL of the REST API if it cannot be derived from `url` |

The API URL is derived from `url`: `https://api.github.com` for github.com,
`https://api.<host>` for `*.ghe.com`, and `https://<host>/api/v3` for GitHub
Enterprise Server.

**Which profile is used:** `actiontui --profile work` always uses that profile.
Otherwise the host (and owner) of the repository's `origin` remote is matched
against the profiles. If several match equally well, `default_profile` decides.
If only one profile exists, it is used for every repository.

The old format with `url` and `pat` at the top level still works and counts as
one profile named `default`.

### Starting a run

`n` opens a form with the branch and every input the workflow declares under
`on.workflow_dispatch.inputs` (read from the workflow file on that branch):

| Input type | Field | Keys |
| --- | --- | --- |
| `string`, `number` | text | `i` edit, `Esc`/`Enter` done, `D` clear |
| `boolean` | `[x]` | `Space`/`x`/`h`/`l` toggle |
| `choice` | `< value >` | `h`/`l` previous / next |

`j`/`k` move between fields, `Enter` starts the run, `Esc`/`q` cancels. Changing
the branch reloads the inputs for that branch. Fields marked `*` are required.

Starting a run needs `workflow_dispatch` in the workflow's `on:` triggers and a token with *Actions: write*.

## Keys

| Key | Action |
| --- | --- |
| `1`-`5` | Focus workflows / runs / jobs / steps / logs |
| `j` `k` / `↓` `↑` | Move down / up (one line in logs) |
| `J` `K` / `End` `Home` | Jump to last / first |
| `Ctrl`+`j` `k` | Scroll logs by 10 lines |
| `g` / `G` | Jump to top / bottom of the logs |
| `Enter` | Open the selected item (workflow → runs → jobs → steps → logs) |
| `n` | Start a new run of the selected workflow (opens the run form) |
| `R` | Runs: rerun the failed jobs of the selected run. Jobs: rerun the selected job |
| `A` | Runs: rerun all jobs of the selected run |
| `r` | Refresh the current list |
| `q` / `Ctrl`+`c` | Quit |

Every rerun asks for confirmation first: `y` confirms, `n`, `q` or `Esc` cancels.

## License

MIT, see [LICENSE](./LICENSE).
