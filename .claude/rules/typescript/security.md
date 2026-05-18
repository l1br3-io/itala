---
paths:
  - "apps/**/*.ts"
  - "apps/**/*.tsx"
  - "apps/**/*.js"
  - "apps/**/*.jsx"
---
# TypeScript Security

> Applies to the Tauri webview shell.

## Secrets

Secrets do not exist in the webview. API keys for cloud services live in Rust, loaded from the OS keychain via `keyring-rs`. The webview never sees them.

```ts
// WRONG — secrets in JS
const apiKey = 'sk-…'

// WRONG — fetching keys from Rust to JS
const apiKey = await invoke<string>('get_openai_key')

// CORRECT — ask Rust to perform the action; the key never crosses the IPC boundary
const reply = await invoke<CompletionReply>('ai_complete', { prompt })
```

The Rust side enforces this — `itala-ai` does not expose any command that returns a key.

## Tauri capabilities

`src-tauri/capabilities/main.json` declares which commands the main webview can call. Default-deny:

```json
{
  "identifier": "main",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "fs:allow-read-text-file"
  ]
}
```

Add to the allowlist consciously. Reviews flag any new permission addition.

## Input validation

The webview validates inputs with `zod` before crossing IPC boundaries:

```ts
import { z } from 'zod'

const ImportRequest = z.object({
  source: z.string().url(),
  format: z.enum(['markdown', 'html']),
})

const validated = ImportRequest.parse(rawInput)
await invoke('import_note', validated)
```

Don't trust URL params, drag-and-drop payloads, or clipboard content. Validate at the boundary.

## XSS / injection

- The webview renders trusted Markdown via the kernel's render output. Plugin-provided HTML goes through `ammonia` on the Rust side before reaching JS — but the webview still treats it as untrusted in shadow DOM or sandboxed iframes.
- Never set `innerHTML` on dynamic content without sanitization. Solid's JSX escapes by default; preserve that — don't reach for `innerHTML` to "speed things up."
- Plugin-provided HTML widgets render in a sandboxed iframe with a strict CSP, isolated from the host webview's session.

## CSP

The Tauri config sets a strict Content Security Policy:

```json
"app": {
  "security": {
    "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: https://ipc.localhost"
  }
}
```

`unsafe-inline` for styles only (Tailwind generates a stylesheet but runtime style switches use it). No remote scripts. No `eval`.

## Console.log

Production code has no `console.log`. The pre-commit hook flags any. Use the `tauri-plugin-log` bridge:

```ts
import { info, warn, error } from '@tauri-apps/plugin-log'

info('vault opened', { vaultId })
```

Logs flow through `tracing` on the Rust side and respect the user's log-level setting.

## Agent support

- Use **security-reviewer** for comprehensive security audits.

## Dependency hygiene

- Lockfile committed (`pnpm-lock.yaml`).
- Run `pnpm audit` in CI; fail on high-severity advisories.
- Avoid packages with native dependencies in the webview build — they break Tauri's mobile bundles.
- Review before adopting any package that ships postinstall scripts.
