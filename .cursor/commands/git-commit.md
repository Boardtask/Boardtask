# Generate Concise Git Commit Message

Generate a short, imperative commit message for the current changes. Output only the message; do not run `git commit` unless asked.

## Steps

1. Review changes: `git status` and `git diff` (or `git diff --cached` if staged)
2. Write a single-line message under 72 characters
3. Use imperative mood: "add", "fix", "update" — not "added", "fixed", "updated"
4. Start with a verb, no period at the end
5. Output only the message text (e.g. `add login and signup buttons to home page`)

## Format

Prefer conventional commits when it helps clarity:

- `type(scope): description` — e.g. `fix(auth): handle expired session`
- Or plain imperative: `add dashboard logout form`

## Rules

- **Concise:** One line, ideally under 50 characters
- **Imperative:** "fix bug" not "fixed bug"
- **Specific:** Describe what changed, not vague labels
- **No period** at the end