# Releasing Inkuno

Releases ship macOS + Linux downloads. There are two paths: the **label gate**
(merge a labeled PR) and **manual preparation** with `scripts/release.sh` —
required for the first release, `0.0.1`. Both end in `release-native.yml`,
which builds the artifacts and publishes only after every download verifies.

> **Naming seam.** New pipeline files use `inkuno` names. Any remaining
> old-name asset strings (bundle names, artifact files, packaging
> directories) are the rename crew's lane and are intentionally untouched
> here. Docs carry the new name except one upstream-origin credit line per
> file (README, ARCHITECTURE, user-guide introduction).

## Label-gated releases

Merge a PR into `main` with exactly one of `rust:patch`, `rust:minor` or
`rust:major`. The gate (`release-gate.yml`) reads the current `crates/inkuno/Cargo.toml`
version, applies the semver bump, commits `crates/inkuno/Cargo.toml` + `Cargo.lock`, tags
`main` as `vX.Y.Z`, creates a **draft** release with generated notes, then
calls `release-native.yml` with the tag. Anything else — no label, two
labels, or an unlabeled merge — is a no-op with a summary line.

- With no previous version tag the gate stops: the first release is manual
  (below). It never creates `0.0.1`.
- If `main` is protected and the push is rejected, the gate pushes a
  `chore: release X.Y.Z` branch and opens a PR instead. Label that PR with the
  same `rust:*` label and merge it; the gate tags the existing bump rather
  than bumping twice. Never push tags by hand.
- Never move a released tag to fix application code. Use a new version.

### Author notes: the `### My take` section

Every release notes draft has three sections:

- **Highlights** — written by the PR author. Add a `### My take` section to
  the PR body describing the change from the user's point of view. The gate
  copies that section verbatim; without it the notes say so.
- **Credits** — author plus commit contributors, collected via `gh`.
- **Changes** — commit subjects since the previous tag.

## Manual releases, including the first 0.0.1

### Prerequisites

Bash, Cargo, Git and GitHub CLI with repository write access. Run from a clean
tracked working tree on `main`, synchronized with `origin/main`, with green
application CI. By default `gh` targets the Inkuno repository
(`kolisachint/inkuno`). Set `INKUNO_REPO` (or edit `REPO` in the script) to
aim elsewhere.
Python 3.11 or newer runs the publication gate on the GitHub runner; the
preparation script itself uses Bash.

Merge the new release scripts and workflows into `main` before making the
first release with this process. `release-native.yml` must be available on the
default branch and on `main`; the tag being built must contain
`scripts/publish-release.py`. Older tags without these files cannot use this
pipeline unchanged.

### Prepare and publish

```bash
# Safe local rehearsal, on any clean branch. Never tags, pushes or contacts GitHub.
# Cargo runs offline: dependencies must already be cached.
DRY_RUN=1 scripts/release.sh X.Y.Z

# First release, then every manual release: from synchronized main.
scripts/release.sh 0.0.1

# Later manual releases (the gate normally handles these).
scripts/release.sh X.Y.Z

# Optional: supply the exact release notes in a local file.
scripts/release.sh X.Y.Z /path/to/notes.md
```

Notes come from the matching changelog section when present, otherwise from
commit subjects since the previous tag. A first release without previous tags
is supported. A supplied notes file takes precedence.

Simulation restores `crates/inkuno/Cargo.toml` and `Cargo.lock` on success, test failure or
interruption. Normal preparation also restores an uncommitted bump if it
fails. Once a version commit has succeeded, it is retained for recovery; the
script does not rewrite committed history.

The local script returns after the workflow has been dispatched, **before
publication finishes**. Follow **Actions → Native release**, or use:

```bash
gh run list --workflow release-native.yml
gh run watch RUN_ID --exit-status
```

The orchestrator validates the numeric `vX.Y.Z` tag, its manifest version and
the draft state. Reusable packaging jobs produce artifacts in that workflow run
without changing a release. The final job runs only after every required job
succeeds, rejects missing, empty or unexpected files, generates `SHA256SUMS`,
uploads the macOS DMG and Linux archive to the draft and checks their exact
names and sizes remotely. Only then does it publish. Remote byte content is
not independently downloaded and rehashed by this gate.

## Recover without a new version

If local preparation stopped after committing or tagging, inspect its error
and use:

```bash
RESUME=1 scripts/release.sh X.Y.Z
```

This requires the existing local tag, a checkout still carrying that version
and a non-divergent `main`. It preserves the tag and existing draft notes,
finishes any missing push/draft step and dispatches the workflow again. A
public release is refused before local mutation. Authentication or API failure
is an error, not evidence that a release is absent.

If preparation finished and only a build or upload failed, fix the environment
or transient issue and **Re-run failed jobs** in the same Actions run. Release jobs upload the compiled binary as a `Cargo.lock`-keyed workflow artifact right after building, so a re-run whose build already succeeded restores that binary and skips recompiling while still re-running test and packaging.
Successfully uploaded workflow artifacts are retained for seven days. Once
they expire, rerun all jobs, or dispatch a fresh complete build:

```bash
gh workflow run release-native.yml --ref main -f tag=vX.Y.Z
```

Runs for the same tag are serialized. Each fresh run downloads only its own
packaging artifacts. A failed upload can be retried against the draft; all
expected assets are replaced and checked before publication. Unexpected stale
release assets must be reviewed and removed explicitly before retrying. The
workflow refuses to overwrite a release that is already public.

Old public releases can still be inspected and downloaded normally; this
workflow intentionally does not rebuild or overwrite them.

## Build packages without publishing

```bash
gh workflow run release-binaries.yml --ref main -f tag=develop
gh workflow run release.yml --ref main
```

Standalone packaging workflows leave their output in Actions artifacts. They
neither attach files to a release nor publish one. `release.yml` is the manual
raw-archive builder for macOS + Linux; it has no publish step and no nightly.
Both packaging workflows run the macOS `cargo test` gate before bundling, so a
red mac test blocks artifacts and, through `release-native.yml`, publication.

## Dropped from the pipeline

- **Windows** (`windows-latest` in CI, the Windows packaging job): not built
  or published. The `packaging/windows` directory remains for the rename crew.
- **Nightly releases**: removed from `release.yml`; pushes to `main` no
  longer create or update any release.
- **AUR / Arch packaging** (`arch-package.yml`, root `PKGBUILD`, `aur/`):
  removed. No Arch package is built or published; checksums are verified from
  the uploaded macOS + Linux downloads instead of `makepkg`.

## Test the release machinery

```bash
bash -n scripts/release.sh
python3 scripts/test-release.py
python3 scripts/test-publish-release.py
```

The first Python suite uses temporary repositories and mocked Cargo/GitHub
tools. The second checks the publication gate, including incomplete builds,
public releases, failed uploads and retry. Neither suite publishes or modifies
the real repository history. **Release checks** runs all three on relevant
pushes and pull requests. Application CI is tiered: fast checks (fmt, check, Clippy,
tests minus the end-to-end session) run on `ubuntu-latest` for every push and
pull request, while the full suite runs on `ubuntu-latest` and `macos-latest`
on `main`, nightly and manual dispatch, with the macOS leg running `cargo test`
before any packaging.

These tests exercise orchestration, not a real macOS/Linux build or a live
GitHub workflow. The first hosted run remains necessary to validate runner
dependencies, permissions and packaging end to end.

The explicit dispatch avoids depending on [draft release
events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows)
or a recursive event generated by `GITHUB_TOKEN`. Publication uses [GitHub
CLI draft editing](https://cli.github.com/manual/gh_release_edit).
