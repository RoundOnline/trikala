# Hosting policy

Numbers and quotas that back the deploy contracts in
[`trikala-axioms-v1.md`](trikala-axioms-v1.md). Per axiom D2, these
specific numbers can change **without** opening a v2 axiom proposal —
edit this file directly, ship a `chore(hosting): ...` commit.

## Free tier (as of v0.1.0-alpha.1)

| Resource | Limit | Scope |
|---|---|---|
| Games per claimed user | 5 | active deployments |
| Storage per user | 100 MB | sum of all artifacts |
| Monthly bandwidth | 5 GB | egress to players |
| Anonymous URL TTL | 7 days | from last deploy |
| Anonymous URLs per IP / 24h | 10 | abuse guard (axiom D10) |
| Deploy rate per identity | 30 / hour | abuse guard |

Anonymous URLs do **not** count against the user's per-claimed-user
quota — they're rate-limited separately by IP (axiom D10).

## Beyond free tier

Paid tier scoping is **out of scope until v1.0**. When introduced
the principles below will guide it:

- Removable "Made with trikala" footer
- Higher storage / bandwidth quotas
- Custom domain mapping (CNAME)
- Priority support SLA

## What's covered by axiom (won't change without v2)

- Free tier **exists** and is non-commercial-use (D2)
- Hard caps, not time-limited trial (D2)
- Anonymous deploys are first-class (D5)
- Anonymous URLs auto-expire (D6) — *the existence of expiry*, not 7 days specifically
- No source code stored server-side (D7)

## What can change in this file freely

- The exact number of games, MB, GB, hours, days
- Specific rate-limit thresholds
- Tier names / pricing structure (if monetized)

## Change log

- *2026-05-15* — initial draft. Numbers reflect early-stage operating
  cost estimates; revise when usage data exists.
