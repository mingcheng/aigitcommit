# AIGitCommit

[![Cargo Build & Test](https://github.com/mingcheng/aigitcommit/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/mingcheng/aigitcommit/actions/workflows/rust.yml)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/11285/badge)](https://www.bestpractices.dev/projects/11285)

![screenshots](./assets/screenshots.png)

`AIGitCommit` is a command-line tool that generates meaningful, semantic commit messages from your staged Git changes using AI. 

It inspects your diffs, summarizes the intent of your changes, and produces clear, concise commit messages that follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification.

<a href="https://www.producthunt.com/products/aigitcommit/reviews?utm_source=badge-product_review&utm_medium=badge&utm_source=badge-aigitcommit" target="_blank"><img src="https://api.producthunt.com/widgets/embed-image/v1/product_review.svg?product_id=1047881&theme=dark" alt="AIGitCommit - A&#0032;simple&#0032;tool&#0032;to&#0032;help&#0032;you&#0032;write&#0032;better&#0032;Git&#0032;commit&#0032;messages&#0046; | Product Hunt" style="width: 250px; height: 54px;" width="250" height="54" /></a>

## References

- https://www.conventionalcommits.org/en/v1.0.0/
- https://nitayneeman.com/blog/understanding-semantic-commit-messages-using-git-and-angular/
- https://ssshooter.com/2020-09-30-commit-message/

## Features

- Generates meaningful, semantic commit messages from staged changes.
- Commit directly to the repository with the `--commit` flag or copy the generated message with `--copy`.
- Output formats: human-readable text, JSON (machine-readable) and table view. JSON output is useful for CI integrations and automation; table view makes it easy to scan multiple suggested lines.
- Easy-to-use command-line interface with sensible defaults and confirm prompts (can be skipped with `--yes`).
- Uses libgit2 via the `git2` crate, avoiding external git commands for improved security and performance.
- Supports multiple OpenAI-compatible models and configurable API base, token, and proxy settings.
- Optional auto sign-off of commits when `GIT_AUTO_SIGNOFF=true`.
- Proxy support: HTTP and SOCKS5 (set via `OPENAI_API_PROXY`).


## How It Works

AIGitCommit inspects your staged Git changes, summarizes the intent of those changes, and generates clear semantic commit messages. It examines diffs and uses an AI model to infer intent and produce concise, useful commit lines.

## Install

AIGitCommit is still in the early stages of development, I suggest you to install it using the git URL using the commands below:

```
cargo install --git https://github.com/mingcheng/aigitcommit.git
```

or, You can install from [crates.io](https://crates.io/crates/aigitcommit)

```
cargo install aigitcommit
```

Those command will auto-download the latest version of the project and install it to your cargo bin directory.

### Docker image

AIGitCommit can run in Docker if you prefer not to install the binary locally. Example (read-only repository):

```bash
docker run \
  --rm \
  -v $PWD:/repo:ro \
  -e OPENAI_API_BASE='<api base>' \
  -e OPENAI_API_TOKEN='<api token>' \
  -e OPENAI_MODEL_NAME='<model name>' \
  -e OPENAI_API_PROXY='<proxy if needed>' \
  ghcr.io/mingcheng/aigitcommit
```

If you want to use `--commit` from inside the container, mount the repo as writable and run interactively:

```bash
docker run \
  --rm \
  -it \
  -v $PWD:/repo:rw \
  -e OPENAI_API_BASE='<api base>' \
  -e OPENAI_API_TOKEN='<api token>' \
  -e OPENAI_MODEL_NAME='<model name>' \
  -e OPENAI_API_PROXY='<proxy if needed>' \
  ghcr.io/mingcheng/aigitcommit --commit
```

Use `--yes` to skip interactive confirmations.

### Git hook

AIGitCommit ships a `hooks/prepare-commit-msg` hook you can copy into a repository's `.git/hooks/prepare-commit-msg` to automatically generate commit messages during `git commit`.

To install globally:

```bash
mkdir -p ~/.git-hooks
# copy the file from this project into ~/.git-hooks/prepare-commit-msg
chmod +x ~/.git-hooks/prepare-commit-msg
git config --global core.hooksPath ~/.git-hooks
```

After installing the hook, `git commit` will run the hook and populate the commit message. Use `--no-verify` to bypass hooks when necessary.

## Configuration

Before using AIGitCommit, export the following environment variables (for example in your shell profile):

- `OPENAI_API_TOKEN`: Your OpenAI-compatible API token.
- `OPENAI_API_BASE`: The API base URL (useful for alternative providers or local proxies).
- `OPENAI_MODEL_NAME`: The model name to query (e.g., a GPT-compatible model).
- `OPENAI_API_PROXY`: Optional. Proxy address for network access (e.g., `http://127.0.0.1:1080` or `socks://127.0.0.1:1086`).
- `GIT_AUTO_SIGNOFF`: Optional. Set to `true` to append a Signed-off-by line to commits.

### Check the configuration

After setting the environment variables, you can check if they are set correctly by running:

```bash
aigitcommit --check-env
```

This will print the current configuration and verify that the required variables are set. 

Then you can run 

```bash
aigitcommit --check-model
```

to check if the specified model is available and can be queried successfully.

You can also run `aigitcommit --help` to see the available options and usage instructions.

## Usage

Run `aigitcommit` in a repository with staged changes. Optionally provide a path to the git directory: `aigitcommit <dir>`.

Common flags:

1. `--commit` commit generated message directly to the repository.
2. `--copy-to-clipboard` copy the generated message to the clipboard.
3. `--json` print the suggestions as JSON for CI or automation.
4. `--yes` skip confirmation prompts and apply the default action.
5. `--signoff` append a Signed-off-by line to the commit message.

See `aigitcommit --help` for the full list of options.


## License

This project is licensed under the MIT License. See the `LICENSE` file for details.
