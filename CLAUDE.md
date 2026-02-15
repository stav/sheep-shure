# SHEEPS Project Guidelines

## Coding Standards
- **Always prefer Tailwind CSS classes over inline styles.** Do not use inline `style` props to work around CSS issues. If Tailwind classes aren't working, investigate the root cause and fix it properly rather than circumventing the framework. For example separating layout from style using inner wrappers.
- Use shadcn/ui component patterns consistently.
- Package manager: bun (not npm or pnpm).
