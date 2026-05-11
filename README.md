# claude-shim

A local HTTP proxy for Claude Code that appends a user-supplied override prompt to the request's `system` field before forwarding to the Anthropic API.

## What it does

- Listens on `127.0.0.1:47821`.
- Forwards all requests to `https://api.anthropic.com`, preserving streaming responses.
- On `POST /v1/messages`: reads `~/.config/claude/claude-shim.md` and appends its content to the `system` field. If the file is absent, the request is forwarded unchanged.

## Installation

From a clone:

```sh
cargo install --path .
```

From git directly:

```sh
cargo install --git https://github.com/nakagawa-hayana/claude-shim
```

The binary is installed to `~/.cargo/bin/claude-shim`.

## Configuration

Write the override prompt to `~/.config/claude/claude-shim.md`. The file is read on every `POST /v1/messages` (no restart needed when editing).

How the file content is applied:

- If the request's `system` is a string, the file content is concatenated with `"\n\n"`.
- If the request's `system` is an array of content blocks, a new `{"type": "text", "text": "<file content>"}` block is appended.

Point Claude Code at the proxy by setting `ANTHROPIC_BASE_URL`:

```sh
export ANTHROPIC_BASE_URL=http://127.0.0.1:47821
```

## Auto-start with zsh

Add to `~/.zshrc`:

```sh
export ANTHROPIC_BASE_URL=http://127.0.0.1:47821
if ! pgrep -x claude-shim > /dev/null 2>&1; then
  nohup claude-shim > /tmp/claude-shim.log 2>&1 &!
fi
```

This launches the proxy in the background on shell startup (if it is not already running) and routes Claude Code through it for the rest of the session. Logs go to `/tmp/claude-shim.log`.

## License

GPL-3.0-or-later. See `LICENSE`.
