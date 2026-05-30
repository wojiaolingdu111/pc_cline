---
name: git-archaeology
description: Use git history to diagnose errors caused by version mismatches or architectural rewrites
source: auto-skill
extracted_at: '2026-05-30T14:10:54.394Z'
---

# Git Archaeology — Diagnosing Version Mismatch Errors

When a user reports an error about a missing file, feature, or component that doesn't match the current codebase, use git history to trace when and how the change happened. This is especially common after major rewrites (e.g., Python → Rust).

## Procedure

### Step 1: Search current code for the error keyword

```bash
grep -r "error_keyword" --include="*.{rs,ts,vue,toml,json,yml,yaml}" .
```

If not found → the feature was removed. Proceed to Step 2.

### Step 2: Find when the file/directory was deleted

```bash
git log --oneline --diff-filter=D -- 'path/to/deleted/**'
```

This shows the exact commit that removed it.

### Step 3: Identify which releases are affected

```bash
# Releases that HAVE the change (no longer need the old component)
git tag --contains <removal-commit>

# Releases that DON'T have the change (still use old component)
git tag --no-contains <removal-commit>
```

### Step 4: Inspect old configuration at the boundary

```bash
# Before the removal (old behavior)
git show <removal-commit>^:src-tauri/tauri.conf.json

# After the removal (new behavior)
git show <removal-commit>:src-tauri/tauri.conf.json
```

### Step 5: Compare and explain

- If the user downloaded from old tags → tell them to get the latest release
- If the config diff reveals a bundle/resources change → that's the root cause
- Use `git ls-tree` to verify what files existed at an old commit:
  ```bash
  git ls-tree -r --name-only <commit> -- 'old-dir/'
  ```

## When to apply

- User reports error about a file/component that doesn't exist in current code
- User downloaded a pre-built binary that behaves differently from current source
- Project has undergone major architectural rewrites with multiple release eras
- Need to determine which versions/releases are affected by a change
