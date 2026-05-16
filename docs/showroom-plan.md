# Showroom plan (alpha.2 deliverable)

> **สถานะ**: vision spec, ยังไม่ implement.
> **เป้าหมายส่งมอบ**: `examples/showroom` — runnable demo ที่โชว์ capability ทั้งหมดของ trikala บนจอเดียว

## เป้าหมายของ artifact

คนเปิด `cargo run -p trikala-showroom` ครั้งแรกแล้วต้อง:

1. **เห็นทันที** ว่า framework ทำได้แค่ไหน — visual evidence ก่อน technical explanation
2. **เชื่อมโยง** กับเกมที่เคยเล่น (dev ที่มาโซนนี้น่าจะเป็น gamer มาก่อน)
3. **ตัดสินใจ** ได้ในวินาทีว่าจะไปทางไหนต่อ — gameplay / story / visual
4. **คุยกับ AI ต่อได้** — Claude Code / Cursor อ่าน showroom + AGENT.md แล้วเสนอ extension path ที่ตรงกับ codebase ทันที ไม่ใช่เดาเอง

## สิ่งที่ต้องโชว์บนจอเดียว

| Capability | สิ่งที่เห็น | Reference game สำหรับ feel |
|---|---|---|
| **Landscape / terrain** | mesh จาก heightmap หรือ procedural | Firewatch, Sable |
| **3D model** | glTF character / prop กลางจอ | Among Us 3D, low-poly mannequin |
| **Font / text** | text รวมไทย (axiom F8 บังคับ Thai shaping) | Disco Elysium, Hades dialogue |
| **HUD UI** | egui panel เช่น HP / minimap / FPS / settings | Celeste, Hollow Knight |
| **Input** | WASD camera + mouse look | universal |

## โครงสร้างโค้ดสำหรับ AI navigation

`src/main.rs` แบ่ง section ชัดเจน — comment header ใหญ่ ๆ พอที่ grep หาเจอใน 1 วินาที:

```rust
// ═══════════════════════════════════════════════════
// SECTION 1: WINDOW + RENDERING SURFACE
// ═══════════════════════════════════════════════════

// ═══════════════════════════════════════════════════
// SECTION 2: LANDSCAPE — terrain mesh + textures
// ═══════════════════════════════════════════════════
```

**ทำไม section ใหญ่**: เวลา human / AI อยาก replace landscape ด้วย sprite atlas → search "SECTION 2" → ลบ → ใส่ใหม่ → จบ ไม่ต้องอ่านทั้งไฟล์

## Decision tree (จะอยู่ใน README หน้าหลัก)

```
อยากทำเกมแบบไหน?
- เน้น gameplay      → templates/2d-platformer  (alpha.2)
- เน้นเนื้อเรื่อง       → templates/visual-novel    (alpha.3)
- เน้น visual         → templates/3d-arena       (alpha.2)
- ไม่รู้ ลองดูก่อน    → cargo run -p trikala-showroom
```

## Reality check vs alpha.1 foundation

| Capability | trikala alpha.1 | งานที่ต้องเพิ่ม |
|---|---|---|
| wgpu + winit | มี | - |
| Font rendering | ไม่มี | integrate `glyphon` หรือ `wgpu_text` |
| HUD UI | ไม่มี | integrate `egui-wgpu` |
| glTF model | ไม่มี | integrate `gltf` crate (skeletal animation = optional v1) |
| Terrain mesh | ไม่มี | procedural heightmap หรือ static glTF terrain |
| Asset bundling | ไม่มี | กำหนดที่เก็บ + license-clean (CC0/MIT) |

ประเมินงาน: **2–3 วัน focused dev** + เวลาคัดเลือก asset

## Open decisions (ต้องตกลงก่อนเริ่ม implement)

1. **F29 axiom** บอก template ≤ 300 บรรทัด — showroom จะยาวกว่านี้
   - (a) ปรับ F29: showroom เป็น exempt class แยกจาก template
   - (b) แตกเป็น 4 mini-demo (`examples/{terrain,model,text,hud}`) แล้ว showroom รวมจอผ่าน orchestrator เล็ก ๆ
2. **Asset licensing** — glTF / font / texture ต้อง CC0 หรือ MIT
   (axiom I5 ห้ามใช้ asset จาก 3chess)
3. **ภาษา default ใน showroom text** — ไทยหรืออังกฤษ?
4. **Camera default** — first-person / third-person / top-down?
   (ส่งสัญญาณ vibe ของ framework)

## Acceptance criteria สำหรับ alpha.2

- [ ] `cargo run -p trikala-showroom` build < 5 นาทีครั้งแรก, < 30 วินาที incremental
- [ ] หน้าต่างเดียวเห็นครบ 5 capability ข้างบน
- [ ] โค้ดทุก section commented ให้ human / AI navigate ภายใน 30 วินาที
- [ ] README ของ examples/showroom/ บอก reference game สำหรับ feel ของแต่ละส่วน
- [ ] `AGENT.md` มี anchor ให้ Claude Code อ่าน showroom แล้วเสนอ extension path ทันที

## Status ตอนนี้ (alpha.1)

- ✅ `examples/showroom/` directory สร้างแล้วเป็น stub (compile ผ่าน, print message)
- ✅ `docs/showroom-plan.md` (ไฟล์นี้) — spec ที่ AI / contributor ใช้เป็นแนวทาง
- ⏳ implement งานจริง → alpha.2
