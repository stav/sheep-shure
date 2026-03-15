# Compass Project Guidelines

## Coding Standards

- **Always prefer Tailwind CSS classes over inline styles.** Do not use inline `style` props to work around CSS issues. If Tailwind classes aren't working, investigate the root cause and fix it properly rather than circumventing the framework. For example separating layout from style using inner wrappers.
- Use shadcn/ui component patterns consistently.
- Package manager: bun (not npm or pnpm).

## Running in Dev

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 bun run tauri dev
```

The `WEBKIT_DISABLE_DMABUF_RENDERER=1` env var is required on Wayland to prevent WebKit rendering issues.

## Tauri Dev Console

- To invoke Tauri commands from the browser dev console, use `window.__TAURI_INTERNALS__.invoke()`:

  ```js
  await window.__TAURI_INTERNALS__.invoke("command_name", { argName: "value" })
  ```

- `window.__TAURI__.core.invoke()` and `window.__TAURI_INTERNALS__.core.invoke()` do **not** work.

## Git Commit Messages

When asked for "commit messages" look in the working tree changes as well as the index (staged files) and give appropriate commit summary messages in the following formats:

1. one word,
2. three words,
3. one sentence,
4. full (long) message.

## React Router v6 — Do Not Use `useSearchParams`

**Never use `useSearchParams` from React Router v6.** It causes infinite re-render loops because `setSearchParams` returns a new function reference whenever `searchParams` changes, which means any `useEffect` that includes `setSearchParams` in its deps will loop forever. Even `window.history.replaceState` is unsafe because React Router monkey-patches it and triggers internal navigation listeners.

Instead, read URL params directly with `new URLSearchParams(window.location.search)` on mount, and manage state internally with `useState`. If URL sync is needed, use `navigate` with `{ replace: true }` behind a debounce or explicit user action — never in a reactive effect.

## Obsidian

The global user Obsidian vault we use is located at ~/Obsidian.

The installed Obsidian mcp server does not work outside of the project directories, so we're just using Claude's built-in tools (Read/Write/Glob/Grep/etc) to interact with the vault.
