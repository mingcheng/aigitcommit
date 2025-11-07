# AIGitCommit

[![Cargo Build & Test](https://github.com/mingcheng/aigitcommit/actions/workflows/cargo.yml/badge.svg?branch=main)](https://github.com/mingcheng/aigitcommit/actions)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/11285/badge)](https://www.bestpractices.dev/projects/11285)
[![Crates.io](https://img.shields.io/crates/v/aigitcommit.svg)](https://crates.io/crates/aigitcommit)

![screenshots](./assets/screenshots.png)

`AIGitCommit` is a command-line tool that generates meaningful, semantic commit messages from your staged Git changes using AI.

It inspects your diffs, summarizes the intent of your changes, and produces clear, concise commit messages that follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification.

<a href="https://www.producthunt.com/products/aigitcommit/reviews?utm_source=badge-product_review&utm_medium=badge&utm_source=badge-aigitcommit" target="_blank"><img src="https://api.producthunt.com/widgets/embed-image/v1/product_review.svg?product_id=1047881&theme=dark" alt="AIGitCommit - A&#0032;simple&#0032;tool&#0032;to&#0032;help&#0032;you&#0032;write&#0032;better&#0032;Git&#0032;commit&#0032;messages&#0046; | Product Hunt" style="width: 250px; height: 54px;" width="250" height="54" /></a>

## References

- [Conventional Commits Specification](https://www.conventionalcommits.org/en/v1.0.0/)
- [Understanding Semantic Commit Messages](https://nitayneeman.com/blog/understanding-semantic-commit-messages-using-git-and-angular/)
- [Commit Message Best Practices](https://ssshooter.com/2020-09-30-commit-message/)

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests on [GitHub](https://github.com/mingcheng/aigitcommit).

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Features

- **AI-Powered Commit Messages**: Automatically generates meaningful, semantic commit messages from staged Git changes
- **Conventional Commits**: Follows the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification for consistent, structured messages
- **Multiple Output Formats**:
  - Human-readable table view (default)
  - JSON format for CI/CD integration and automation
  - Plain text output
- **Flexible Workflow**:
  - Direct commit with `--commit` flag
  - Copy to clipboard with `--copy-to-clipboard`
  - Git hook integration for automatic message generation
- **Interactive & Non-Interactive**: Confirmation prompts by default, skip with `--yes` for scripting
- **Security & Performance**: Uses libgit2 via the `git2` crate, avoiding external git command execution
- **Multi-Provider Support**: Compatible with OpenAI and other OpenAI-compatible APIs (Azure OpenAI, local models, etc.)
- **Flexible Configuration**:
  - Environment variables for API settings
  - Git config for repository-specific or global settings
  - Configurable API base URL, token, proxy, and timeouts
- **Sign-off Support**: Auto sign-off via `AIGITCOMMIT_SIGNOFF` environment variable or `git config aigitcommit.signoff`
- **Proxy Support**: HTTP and SOCKS5 proxies via `OPENAI_API_PROXY`


## How It Works

AIGitCommit streamlines your commit workflow by:

1. **Analyzing Changes**: Inspects staged changes using `git diff --cached`
2. **Understanding Context**: Examines recent commit history for stylistic consistency
3. **AI Generation**: Sends diffs to an OpenAI-compatible model with carefully crafted prompts
4. **Structured Output**: Generates commit messages following Conventional Commits specification
5. **User Review**: Presents the message for review and optional editing

The tool uses libgit2 for secure, efficient Git operations without spawning external processes. It automatically filters out common noise files (lock files, generated code) to focus on meaningful changes.

## Installation

### From crates.io (Recommended)

```bash
cargo install aigitcommit
```

### From Source

For the latest development version:

```bash
cargo install --git https://github.com/mingcheng/aigitcommit.git
```

Both commands will download, compile, and install the binary to your Cargo bin directory (typically `~/.cargo/bin`). Ensure this directory is in your `PATH`.

### Docker Image

Run AIGitCommit in Docker without installing the binary locally.

**Read-only mode** (generate message only):

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

**Interactive mode** (with `--commit` flag):

```bash
docker run \
  --rm \
  -it \
  -v $PWD:/repo:rw \
  -e OPENAI_API_BASE='<api base>' \
  -e OPENAI_API_TOKEN='<api token>' \
  -e OPENAI_MODEL_NAME='<model name>' \
  -e OPENAI_API_PROXY='<proxy if needed>' \
  ghcr.io/mingcheng/aigitcommit --commit --yes
```

Note: Use `--yes` to skip interactive confirmations in non-TTY environments.

### Git Hook

AIGitCommit includes a `prepare-commit-msg` hook that automatically generates commit messages during your workflow. The hook triggers when you run `git commit` or `git commit -m ""`, generates a message from staged changes, and opens your editor for review.

**Prerequisites**

- `aigitcommit` must be installed and available in your `PATH`
- Configure required environment variables before committing (see [Configuration](#configuration))

**Per-Repository Installation**

Install the hook for a single repository:

```bash
cp hooks/prepare-commit-msg .git/hooks/prepare-commit-msg
chmod +x .git/hooks/prepare-commit-msg
```

After installation, the hook runs automatically when you execute `git commit`. You can review and edit the generated message before finalizing the commit.

**Disable for a single commit**: Use `git commit --no-verify` to bypass the hook.

**Global Installation**

Set up the hook for all new and existing repositories using Git templates:

```bash
# Create template directory structure
mkdir -p ~/.git-template/hooks
cp hooks/prepare-commit-msg ~/.git-template/hooks/prepare-commit-msg
chmod +x ~/.git-template/hooks/prepare-commit-msg

# Configure Git to use this template for new repositories
git config --global init.templateDir ~/.git-template

# Apply to existing repositories
# Option 1: Copy manually
cp ~/.git-template/hooks/prepare-commit-msg <repo>/.git/hooks/

# Option 2: Re-initialize (safe, preserves existing data)
cd <repo> && git init
```

**Important**: Setting `core.hooksPath` globally overrides all repository hooks. The template approach is more flexible and recommended.

**Hook Behavior**

The hook only runs when:
- You execute `git commit` (interactive mode) with no pre-written message
- You execute `git commit -m ""` (explicit empty message)

The hook skips execution for:
- Commits with pre-written messages (`git commit -m "message"`)
- Merge commits, rebase, cherry-pick, or other automated commits
- When the commit message file already contains non-comment content

**Troubleshooting**

- **"No staged changes detected"**: Run `git add` to stage your changes before committing
- **"aigitcommit is not installed"**: Ensure the binary is in your `PATH` or install it first
- **Missing configuration error**: Export required environment variables (`OPENAI_API_TOKEN`, etc.) in your shell
- **Hook output too verbose**: Redirect stderr in your Git configuration: `git config core.hookStderr false`

## Configuration

### Environment Variables

Configure AIGitCommit by setting these environment variables (in your shell profile, `.bashrc`, `.zshrc`, etc.):

**Required:**
- `OPENAI_API_TOKEN`: Your OpenAI-compatible API authentication token
- `OPENAI_API_BASE`: API endpoint URL (e.g., `https://api.openai.com/v1` or your provider's URL)
- `OPENAI_MODEL_NAME`: Model identifier (e.g., `gpt-4`, `gpt-3.5-turbo`, or provider-specific models)

**Optional:**
- `OPENAI_API_PROXY`: HTTP/SOCKS5 proxy URL (e.g., `http://127.0.0.1:1080`, `socks5://127.0.0.1:1086`)
- `OPENAI_API_TIMEOUT`: Request timeout in seconds (default: 30)
- `OPENAI_API_MAX_TOKENS`: Maximum tokens in response (default: model-specific)
- `AIGITCOMMIT_SIGNOFF`: Enable auto sign-off (`true`, `1`, `yes`, `on`)

**Example configuration:**

```bash
# ~/.bashrc or ~/.zshrc
export OPENAI_API_TOKEN="sk-..."
export OPENAI_API_BASE="https://api.openai.com/v1"
export OPENAI_MODEL_NAME="gpt-4"
export OPENAI_API_PROXY="http://127.0.0.1:1080"  # Optional
export AIGITCOMMIT_SIGNOFF="true"                # Optional
```

### Git Configuration

You can also enable sign-off via Git configuration (takes precedence over environment variables):

```bash
# Repository-specific
git config aigitcommit.signoff true

# Global (all repositories)
git config --global aigitcommit.signoff true
```

### Verify Configuration

Check your environment setup:

```bash
# Verify all environment variables
aigitcommit --check-env

# Test API connectivity and model availability
aigitcommit --check-model

# Show all available options
aigitcommit --help
```

## Usage

### Basic Usage

Run AIGitCommit in a Git repository with staged changes:

```bash
# In the current repository
aigitcommit

# Specify a different repository path
aigitcommit /path/to/repo
```

The tool will:
1. Analyze your staged changes (`git diff --cached`)
2. Generate a Conventional Commit message using AI
3. Display the result in table format (default)

### Command-Line Options

**Output Formats:**
- Default: Table view (easy to read)
- `--json`: JSON output (for CI/automation)
- `--no-table`: Plain text output

**Actions:**
- `--commit`: Automatically commit with the generated message
- `--copy-to-clipboard`: Copy the message to clipboard
- `--yes`: Skip confirmation prompts (useful for scripting)
- `--signoff`: Append `Signed-off-by` line to the commit

**Diagnostics:**
- `--check-env`: Verify environment variable configuration
- `--check-model`: Test API connectivity and model availability
- `--help`: Show all available options

### Examples

**Generate and review message:**
```bash
aigitcommit
```

**Auto-commit without confirmation:**
```bash
aigitcommit --commit --yes
```

**Copy message to clipboard:**
```bash
aigitcommit --copy-to-clipboard
```

**JSON output for CI pipelines:**
```bash
aigitcommit --json | jq '.title'
```

**Commit with sign-off:**
```bash
aigitcommit --commit --signoff
```

### Workflow Integration

**Typical workflow:**
```bash
# Stage your changes
git add .

# Generate and review commit message
aigitcommit

# Or commit directly
aigitcommit --commit

# Or use the Git hook (if installed)
git commit  # Hook generates message automatically
```

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.
