# `trikala` — Axioms v1

> Load-bearing design contracts. ห้ามเปลี่ยนโดยไม่ผ่าน v2.
> ทุก feature/PR ต้อง audit กับ axioms ก่อน merge.
> หาก code ขัด axiom → code ผิด ไม่ใช่ axiom

## รหัส

6 หมวด: **T**enets, **U**X/CLI, **F**oundation, **D**eploy, **C**ommunity, **I**ntegration

ขัด axiom 1 ข้อ = block. แก้ได้ก็ต่อเมื่อเสนอ v2 ทั้งชุด (ไม่ patch ทีละข้อ)

---

## T — Tenets (identity) · 8 ข้อ

**T1.** *What trikala IS* (product identity) — **thin foundation + opinionated CLI + hosting**. ไม่ใช่ engine (ไม่มี ECS / scene graph mandate), ไม่ใช่แค่ template (มี CLI + release lifecycle + working deploy)

**T2.** สามเฟส (อดีต/ปัจจุบัน/อนาคต = new/dev/deploy) เป็น load-bearing UX ทุก surface ต้องสื่อ (CLI help, error codes, docs, template structure)

**T3.** Beginners-first ทุก decision ที่ trade-off "ง่ายสำหรับมือใหม่" vs "ทรงพลังสำหรับ pro" → ฝั่งมือใหม่ชนะเสมอใน v0.x และ v1.0. Pro audience ที่ต้องลึกกว่านี้ = Bevy serves them

**T4.** ห้ามบังคับ paradigm — ไม่มี ECS mandate, ไม่มี scene graph mandate, ไม่มี state mgmt opinion. User เลือกเอง

**T5.** Multi-discipline native — ทุก template + ทุก doc ต้องชี้ทาง Code / Visual / Sound persona ชัด

**T6.** Production-credible — patterns ใน trikala ต้องมาจาก 3chess production ที่ ship จริง (cite-able commit/file ใน 3chess), ไม่ใช่ tutorial-quality code ที่เขียนใหม่เพื่อ docs

**T7.** ชื่อ + 3-phase narrative คือ moat ไม่ใช่ ornament — ห้าม dilute (ห้ามเปลี่ยนชื่อ ห้ามทิ้ง 3-phase metaphor ห้ามลบ Sanskrit etymology)

**T8.** *How trikala IS BUILT* (implementation philosophy) — **glue layer ที่ assemble ecosystem** (cargo-generate, cargo-mobile2, egui, kira, notify, dexterous_developer) ผ่าน opinionated UX + branded surface + hosting. Value-add อยู่ที่ assembly + workflow + brand, ไม่ใช่การเขียน infrastructure ใหม่

---

## U — UX / CLI · 16 ข้อ

**U1.** `trikala new <name>` ทำงานได้โดยไม่ต้องมี flag — ใช้ default template, zero-prompt

**U2.** `trikala dev` boot < 5 วินาที บน mid-tier laptop + hot reload working ทันที

**U3.** `trikala deploy` (ไม่มี arg) ส่ง round.online เป็น default — target อื่น opt-in ผ่าน sub-command

**U4.** *Error UX principle* — ทุก error ต้องบอก next step ที่ user action ได้, ห้าม dump raw stack trace เป็น primary output (เก็บ trace ใต้ `--verbose`). Format ที่บังคับใช้ดู U10

**U5.** `trikala --help` คือ doc สมบูรณ์ — ทำงานได้โดยไม่ต้องเปิด website

**U6.** 3-phase narrative ปรากฏใน `trikala --help` output (อดีต→new, ปัจจุบัน→dev, อนาคต→deploy)

**U7.** ไม่มี interactive prompt ระหว่าง deploy — auth token ผ่าน env หรือ config file ที่ตั้งล่วงหน้า (CI/CD friendly)

**U8.** ทุก command มี `--dry-run` ที่ทำงานได้จริง — แสดงสิ่งที่จะเกิดโดยไม่ทำ

**U9.** CLI ตอบกลับใน 1 บรรทัดเมื่อ success (เงียบ ๆ ตาม Unix philosophy). verbose mode สำหรับ debug

**U10.** *Error structural format* — ทุก error ต้องมี `code` (machine-readable เช่น `ATI-001`) + `cause` (1-line human) + `hint` (next step) + `docs_url` (deep link). Implements U4

**U11.** *Install bootstrap* — primary คือ `cargo install trikala` (`git clone + cargo build` ระหว่าง alpha ก่อน publish). Updates ผ่าน `trikala upgrade` (CLI self-update, ดาวน์โหลด binary จาก GitHub Releases). Pre-built binary helpers (install.sh / install.ps1) เป็น secondary polish — เพิ่มเมื่อมี evidence ว่า non-Rust audience ใช้

**U12.** Telemetry **opt-in** เสมอ — banner ครั้งเดียวตอน first run, `TRIKALA_TELEMETRY=0` ปิดได้, **ห้าม block CLI** ถ้า user ปฏิเสธ. Data scope ดู U15

**U13.** Version pinning — `trikala.toml` ระบุ `[trikala] version = "0.x.y"`, `trikala use <ver>` switch ได้ (rustup-style) เพื่อ pre-empt breaking-change pain

**U14.** Stable flag names — deprecation warning ≥ 2 versions ก่อนลบ; ห้าม rename flag ใน minor version

**U15.** *Telemetry data scope* (เมื่อ user opt-in):
- เก็บได้: CLI version, OS family (linux/macos/windows/wasm), anonymous session ID, command name, exit code, error code (U10)
- **ห้าม**เก็บ: source code, file paths, project name, asset content, IP address (in long-term storage), environment variables
- Schema documented ใน `docs/telemetry-schema.md` — เปลี่ยน schema = breaking change ต้องผ่าน v2

**U16.** CLI output (error / `--help` / prompts) เป็น **English เท่านั้น** ใน v0.x และ v1.0. i18n รอ trigger gate (10x UX evidence ว่า user majority ไม่อ่าน EN). Docs/marketing content ภาษาไหนก็ได้

---

## F — Foundation · 31 ข้อ

**F1.** `trikala-core` คือ foundation crate **เดียว** — ลด surface ให้น้อยที่สุด. Templates ไม่ผ่าน trikala wrapper crates สำหรับ render/audio/text/ui (ใช้ wgpu/kira/cosmic-text/egui ตรง ๆ ตาม F30)

**F2.** Public API ของ `trikala-core` ใช้ type ในตัวเอง — ไม่ leak `wgpu::*`, `winit::*`, `serde_json::Value` ฯลฯ ผ่าน signature ของ public function (consumers ของ core ไม่ควรต้อง depend on transitive crates)

**F3.** Shaders hot-reload ใน dev, embedded เป็น bytes ใน release build — บังคับทุก WGSL ใน project

**F4.** Assets ทำตาม slot convention: `art/` `music/` `sfx/` `fonts/` `levels/` — auto-discover ไม่ต้อง register

**F5.** ไม่มี global state ทั้งระบบ — state ทั้งหมดอยู่ใน user's `App` struct ที่ trikala รับเป็น generic

**F6.** MSRV = Rust stable - 2 releases. ห้ามใช้ nightly features

**F7.** CI เขียวบน Windows / macOS / Linux / Web ทุก commit บน `main` ถ้า target ไหนแดง → revert ทันที (ไม่ค้าง branch)

**F8.** Text rendering รองรับ Thai shaping out-of-the-box — bar คือ "ถ้าแสดงไทยไม่ได้ก็ไม่ใช่ trikala" (port จาก 3chess production)

**F9.** ไม่มี runtime panic ใน foundation crate — ทุก path คืน `Result` หรือ `Option` ที่ user handle

**F10.** Hot reload เปลี่ยน asset/shader ต้อง preserve game state — ไม่ reset progress ของ user (umbrella; format details in F11/F20)

**F11.** *Dev loop workflow* — ครบ 4 ขั้น: hot reload → debug panel → save defaults → preserved state. ทุก template + foundation crate ต้องรองรับ workflow นี้ออกของกล่อง

**F12.** `tuning.toml` ที่ root ของ project คือ source of truth สำหรับ runtime constants — `Save Defaults` ใน debug panel เขียนกลับไฟล์นี้ commit ลง git ได้

**F13.** *Dev state preservation contract* — state snapshot สำหรับ `cargo rebuild` preservation ต้อง: (1) อยู่ใน build directory ที่ gitignored, (2) ไม่ใช่ source of truth, (3) reset ได้โดยไม่ทำลาย project. Implementation path อยู่ใน architecture spec (เปลี่ยนได้ผ่าน implementation PR, ไม่ใช่ axiom v2)

**F14.** Build variants ผ่าน `trikala.toml [variants.*]` — default variants: `release` / `dev` / `demo` / `capture` / `tools`. User เพิ่ม custom variant ได้

**F15.** Demo gating ต้อง **compile-time** ไม่ใช่ runtime toggle (ป้องกัน memory-edit unlock)

**F16.** Capture variant ต้อง deterministic — same seed = same frames (สำหรับ marketing assets ที่ reproducible)

**F17.** ทุก variant produce binary ที่ตั้งชื่อชัด: `{name}-{variant}{.ext}` เช่น `mygame-demo.exe`, `mygame-capture.exe`

**F18.** Lazy-loaded WASM modules ตาม convention `<crate>.wasm` — `main.wasm` ต้อง < 1MB (load แรกเร็ว)

**F19.** Asset bundling per variant ผ่าน `assets_include` / `assets_exclude` ใน `trikala.toml`

**F20.** *Hot-reload state type contract* — game state ที่ผ่าน reload ต้อง `Serialize + DeserializeOwned + Send` (dexterous_developer pattern). Compile-time check ผ่าน trikala-render builder

**F21.** API churn defense — `wgpu` / `winit` / `egui` versions pin เป็น `= "x.y.z"` ใน workspace deps; trikala public traits stable ข้าม underlying version bump (lesson: comfy archived Nov 2025 จาก ecosystem churn)

**F22.** No blocking calls บน WASM target — `std::thread::sleep`, blocking file I/O, blocking network → compile error หรือ panic ทันที (lesson: macroquad's async requirement)

**F23.** Editor / debug-panel code **ห้าม** compile เข้า release binary — strict `#[cfg(feature = "dev")]` + CI ตรวจ feature combinations (Fyrox 3-crate pattern)

**F24.** Templates = named registry, hash-pinned, expandable — `trikala new -t <name>` resolve จาก registry; ไม่ hard-code 7 templates; community เพิ่ม template ได้ผ่าน PR

**F25.** *Performance budget* — blank template cold-start < 100 ms (WASM main thread to first frame), frame time stable @ 60 fps สำหรับ idle scene บน mid-tier laptop. CI bench guard (regression fails build)

**F26.** *Asset size budget* — blank template < 5 MB gzipped (binary + assets). ทุก template ใน registry ระบุ size budget ใน metadata; CI fails if > 110% ของ declared budget

**F27.** *Build reproducibility* — same git SHA + same Rust toolchain version + same trikala foundation version → byte-identical output (verifiable ผ่าน sha256). ป้องกัน supply chain attack + ทำให้ user verify deploys ได้

**F28.** *60-second iteration loop* — `time(edit src/main.rs → trikala deploy → fetch URL)` < 60 วินาที บน mid-tier laptop + reasonable network. Measured ผ่าน CI bench ทุก commit บน `main`. Regression > 10% = block. **First-build time** (cold cargo) ไม่อยู่ใน promise นี้ — เป็น machine-dependent

**F29.** *Template line-count cap* — ทุก template ใน `templates/` ต้อง ≤ 300 บรรทัด total .rs files (ไม่รวม comments หนาแน่น + Cargo.toml + assets). CI guard นับ + fails > 110% ของ cap. Templates คือ teaching artifacts — readable in one sitting

**F30.** *No trikala-* imports in templates* — templates depend on `wgpu`, `winit`, `kira`, `egui`, `glam`, `bytemuck` ตรง ๆ ผ่าน Cargo.toml ของตัวเอง. นำเข้าได้แค่ `trikala-core` (สำหรับ Phase / Error format compatibility). User fork template + ลบ trikala folder = template ยัง compile ได้

**F31.** *AI-agent friendly templates* — `templates/<name>/src/main.rs` ต้อง fit ใน **8000 tokens** (≈ 1 context window ของ smaller LLMs). ห้ามมี macro DSL ที่ AI ต้อง expand เอง, hidden global state, หรือ trait magic ที่ behavior ไม่อยู่ในไฟล์. Comment ในระดับที่ Cline/Aider/Cursor/Claude Code grok แล้วแก้ได้ตรงจุด — *property ที่ทำให้ template เป็น AI-native by design ไม่ใช่ feature เพิ่ม*

**F32.** *AI agent instructions are versioned in the repo* — `AGENT.md` ที่ root ของ trikala repo (และทุก template) คือ **canonical source** ของ instruction set สำหรับ AI coding agents. Vendor-specific format (`.claude/skills/trikala/SKILL.md`, `.cursorrules`, `.aider.conf.yml`, etc.) คือ **mirrors** ของ `AGENT.md` (symlink หรือ generation). เพิ่ม vendor format ใหม่ = add mirror, ห้าม re-author content. ทุก mirror สม่ำเสมอผ่าน CI check

---

## D — Deploy / Hosting (round.online) · 13 ข้อ

**D1.** Default ของ `trikala deploy` คือ `round.online/play/<user-or-anon>/<game>` — path-based ไม่ใช่ subdomain

**D2.** *Free tier shape* — มี hard cap ที่นับได้ (จำนวน games + storage size + bandwidth quota) — ไม่ใช่ time-limited trial. ตัวเลขจริงอยู่ใน `docs/hosting-policy.md` ที่ปรับตาม economic constraints ได้ **โดยไม่ผ่าน v2 axiom process**

**D3.** Footer "Made with trikala" auto-inject ในทุก game ที่ host บน round.online — ปลดได้บน paid tier ในอนาคต

**D4.** Trikala host เฉพาะ WASM bundle + assets — **ไม่มี server-side execution** ทุกอย่างรันใน user's browser (security + cost)

**D5.** *Anonymous-first deploy* — `trikala deploy` first call ทำงานได้โดยไม่ต้อง login → ephemeral URL ทันที. **Permanent** URL ต้อง `trikala claim` ผ่าน GitHub OAuth. Unfair advantage vs Vercel/Expo/Fly account-gate

**D6.** *URL lifecycle* — claimed URLs ถาวรจนกว่าจะลบเอง; anonymous URLs auto-expire 7 วัน (banner เตือนใน-game ก่อนหมดอายุ)

**D7.** ห้ามเก็บ source code ของ user บน server — เก็บเฉพาะ build artifact (.wasm + assets)

**D8.** Quota เกิน → deploy fail พร้อม actionable message + ลิงก์ upgrade — ไม่ silent truncate

**D9.** `round.online/play/*` ไม่กระทบ `round.online/3chess` operationally — ดาวน์คนละ blast radius

**D10.** Rate-limiting per IP (anonymous) + per GitHub identity (claimed) — กัน abuse โดยไม่บล็อก anonymous-first UX

**D11.** TOS แสดงตอน first `trikala deploy` (anonymous หรือ claimed ก็ตาม) — user ต้อง consent ครั้งเดียว เก็บใน config

**D12.** Cost / quota preview — ถ้า deploy ครั้งถัดไปจะเกิน quota หรือเข้า paid tier, แสดง preview ก่อน execute (lesson: Fly.io billing surprises)

**D13.** *No login wall on non-persistent commands* — `trikala build`, `dev`, `doctor`, `new` ทำงานบน CI/automation ได้ **โดยไม่ต้อง interactive auth**. Login บังคับเฉพาะ action ที่ persist (claim, custom domain, upgrade tier)

---

## C — Community · 9 ข้อ

**C1.** License = dual MIT / Apache-2.0 — Rust ecosystem standard

**C2.** PR review SLA — **v0.x**: CI green + ≥ 1 reviewer approval ก่อน merge. **v1.0+**: เพิ่มเป็น ≥ 2 reviewers สำหรับ PR ที่มี label `breaking` (CI lint ตรวจ label vs Conventional commit type)

**C3.** Maintainer triage SLA — **Month 1 หลัง launch**: 48 ชั่วโมง (issues + PRs). **หลังจากนั้น**: best-effort, ไม่มี SLA promise (community/employer ของ maintainers ไม่บังคับให้ตอบ)

**C4.** Conventional commits บังคับผ่าน CI lint (`feat:` / `fix:` / `docs:` / `chore:` / `refactor:` / `test:` / `perf:` / `ci:` / `build:` / `revert:`)

**C5.** ห้ามใส่ corporate sponsor logos ใน repo จนถึง v1.0 — เลี่ยง Bevy-style sponsor sprawl ในช่วง MVP

**C6.** AI prompt cards format มาตรฐานเดียวบังคับ — ถ้า PR เขียน format อื่น → block. รายละเอียดใน `docs/ai-prompt-cards.md`

**C7.** ทุก template ใน `templates/` ต้อง compile + run บน CI ทุก target — template เสีย = block release

**C8.** Code of conduct = Contributor Covenant v2.1 (no custom version)

**C9.** Discussions เปิด แต่ official support channel = GitHub Issues — ห้ามตอบ support บน Twitter/Discord เป็น primary

---

## I — Integration กับ 3chess · 5 ข้อ

**I1.** ห้ามมี proprietary code จาก 3chess ใน trikala — clean license boundary (3chess = Proprietary, trikala = MIT/Apache)

**I2.** บทความบน 3chess.online cross-link ไป trikala ทุกบท + vice versa

**I3.** Brand attribution `trikala by Round Online` แต่ trikala repo operationally แยก (PR/issue/release lifecycle ของตัวเอง, ไม่ผูกกับ 3chess release schedule)

**I4.** ถ้า 3chess production จะใช้ foundation crate ของ trikala ใช้ได้ — **ทางเดียว** (prod ใช้ trikala ไม่ใช่ reverse)

**I5.** Trikala templates ไม่ shipping 3chess art/sound assets — license ของ 3chess strict กว่า MIT

---

## Total: **83 axioms**

| Category | จำนวน |
|---|---|
| T (Tenets) | 8 |
| U (UX/CLI) | 16 |
| F (Foundation) | 32 |
| D (Deploy/Hosting) | 13 |
| C (Community) | 9 |
| I (Integration) | 5 |

---

## สิ่งที่ **ไม่ใช่** axiom (จงใจปล่อยให้ยืดหยุ่น)

- ภาษาของ codebase comment (อังกฤษเป็นหลัก แต่ไทยใส่ได้ใน docs)
- รายการ deploy targets ที่รองรับ (เพิ่ม/ลดได้ตาม demand)
- จำนวน templates ใน v0.1 (7 ตอนนี้ แต่ปรับได้ก่อน v1)
- เลือก audio crate (kira default ตาม T8 แต่ swap ได้)
- รูปแบบ AI prompt cards content (format บังคับ แต่เนื้อหา flexible)
- Sponsorship / monetization model หลัง v1.0
- Roadmap ของ paid tier features
- Anonymous URL TTL ที่แน่นอน (7 วันคือ default ใน policy doc แต่ปรับได้)
- **ตัวเลข free tier** (จำนวน games, storage, bandwidth) — อยู่ใน `docs/hosting-policy.md`
- **Path ของ dev state snapshot** — อยู่ใน architecture spec

---

## เปลี่ยน axiom ยังไง

### Trigger criteria (เกณฑ์ที่อนุญาตให้เริ่มเสนอ v2)

การเสนอ v2 ต้องตอบ ≥ 1 ข้อต่อไปนี้ได้ **พร้อมหลักฐานวัดได้**:

**(a) High-performance discovery** — มีเทคนิคที่ให้ผล ≥ **2x** ในมิติที่วัดได้:
- frame time / render speed
- memory footprint
- startup / load time
- binary size
- build time
- network latency

ต้องแนบ benchmark ที่ reproducible (ไม่ใช่ anecdote "ฉันรู้สึกว่าเร็วขึ้น")

**(b) 10x UX discovery** — pattern ที่ลด friction ≥ **10x** วัดจาก:
- จำนวน step ที่ user ต้องทำ (เช่น 5 step → 0.5 step)
- เวลาที่ใช้สำเร็จ task (เช่น 5 นาที → 30 วินาที)
- error rate ที่ลดลง
- install / onboarding friction

ตัวอย่าง: anonymous-first deploy (D5) — ลด first-deploy จาก 5–10 ขั้นตอน (signup + verify + OAuth + project link + deploy) เหลือ 1 ขั้นตอน

**(c) Existential risk discovery** — ค้นพบ failure mode ที่ axiom ปัจจุบันไม่ป้องกัน:
- security vulnerability ใน contract ปัจจุบัน
- upstream dependency ตาย / archive (เช่นถ้า wgpu ออก stable 1.0 → F21 ต้องคิดใหม่)
- legal / license breach
- ecosystem shift ที่ทำให้ axiom ปัจจุบันเป็นไปไม่ได้

### สิ่งที่ **ไม่** trigger v2 (reject ตั้งแต่ proposal stage)

- "ฉันชอบแบบนี้มากกว่า" — taste
- "ทีมอื่นทำแบบ X อยู่" — trend-following
- "Library ใหม่ลื่นกว่านิดหน่อย" — cosmetic
- "Code จะอ่านง่ายขึ้น" — refactoring (ทำได้ ไม่ต้องเปลี่ยน axiom)
- "เผื่อในอนาคตอาจจะ..." — speculation
- "User feedback คนเดียวบอกว่า..." — sample size 1

### Process หลัง trigger qualified

1. เสนอเป็น **`trikala-axioms-v2.md`** ฉบับใหม่ (ไม่ patch v1)
2. อธิบายว่า v1 axiom ไหนถูก supersede / removed / split + แนบ **evidence** (benchmark / measurement / risk doc)
3. เปิด PR + รอ community review ≥ 14 วันก่อน merge
4. หลัง merge: `v1.md` ยังคงอยู่ในรีโป (archive) มี note ชี้ไป v2
5. Code/docs ค่อย ๆ migrate ตาม semver (breaking ไป v2.0.0 ของ trikala)

### ผลข้างเคียงของ trigger gate

- 90% ของ proposals จะ filter ทิ้งตั้งแต่ก่อนเขียน PR — ลด review fatigue
- เหลือเฉพาะการเปลี่ยนแปลงที่มี evidence-based justification — quality สูงขึ้น
- Contributor ที่จริงจังจะเตรียม benchmark/measurement → ได้ผลงานที่ trikala นำไปใช้ได้จริง
- ไม่มี bikeshedding ทาง taste — discussion โฟกัสที่ตัวเลข
