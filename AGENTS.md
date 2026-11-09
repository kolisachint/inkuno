# Agent instructions

Contributor rules live in `CONTRIBUTING.md` and `ARCHITECTURE.md`; follow them. This file adds one coordination rule for parallel agent work.

## Parallel work: one worktree per writer

Parallel writer crews sharing one tree collide (stash sweeps, overwrites, E0004 blockers). Every writer crew gets its own worktree.

- **When**: more than one writer works at once. Readers and scouts stay on the main tree; each writer gets a worktree.
- **Coordinator setup** (record the base first):
  ```bash
  git rev-parse HEAD                              # record this base
  git branch --list agent/<label>                 # must print nothing
  git worktree add -b agent/<label> ~/.hoocode/worktrees/<repo>/<label> <base>
  ```
- **Per writer**: file-scoped (edit only its assigned files), commits on its own `agent/<label>` branch, never switches branches, never writes the parent tree, never pushes.
- **Merge + cleanup**: merge onto the base only after verifying (diff + tests). Remove the worktree and branch only after merged:
  ```bash
  git worktree remove ~/.hoocode/worktrees/<repo>/<label>
  git branch -d agent/<label>
  ```
