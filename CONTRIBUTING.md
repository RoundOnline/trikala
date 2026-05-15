# Contributing to trikala

Welcome. trikala is open-source from day one — first-time contributors
are explicitly invited. This file is short; the philosophy below is
load-bearing.

## Before you open a PR

1. **Read `docs/trikala-axioms-v1.md`.** Every change is audited
   against the 73 contracts there. If your PR conflicts with one,
   read the *Trigger criteria* at the bottom of that file. You
   probably don't need to change an axiom — you need to find a
   path through them.

2. **Pick the smallest scope that works.** "While I'm in there"
   refactors get split into separate PRs.

3. **Write a Conventional Commit message.** Format:
   `<type>(<scope>): <summary>`. Types: `feat`, `fix`, `docs`,
   `chore`, `refactor`, `test`, `perf`. CI lints this.

## Code review SLA

In v0.x: 48 hours during the first month. Best-effort after.
If you've been waiting longer than two weeks, ping the PR.

## Where to start

- **Good first issues** are labelled `good-first-issue` in the
  tracker. They're scoped to land in one sitting.
- **Adding a template** is one of the highest-leverage
  contributions. See `templates/blank/` for the shape; pitch
  the idea in an Issue first so we can coordinate the registry
  entry.
- **Fixing a `code/cause/hint/docs_url` error** that wasn't
  written yet is always welcome — that contract (axiom U10)
  is the most visible thing users touch.

## What we do not accept

- "I prefer this style" changes (axiom trigger criteria, *Auto-reject*)
- Net-new dependencies without a written justification in the PR
- Code that leaks `wgpu::*` / `winit::*` / `egui::*` types across
  a crate boundary (violates F2)
- Changes that break a CI target (auto-revert per F7)

## Code of conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — we adopt Contributor Covenant 2.1.
Violations: open an issue tagged `coc-report` or email
conduct@round.online.

## License

By contributing you agree your contribution is dual-licensed under
MIT and Apache-2.0 to match the rest of the repo.
