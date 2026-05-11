import { useState, useEffect } from "react";

const FONT = "'IBM Plex Mono', 'JetBrains Mono', monospace";
const SANS = "'IBM Plex Sans', 'DM Sans', system-ui, sans-serif";

const T = {
    bg: "#0a0c10", surface: "#13161d", surface2: "#1a1e28", surface3: "#222836",
    border: "#2a3040", borderHi: "#3a4560",
    accent: "#5ce0b8", accentDim: "#2a7a62", accentFaint: "rgba(92,224,184,0.08)",
    warn: "#f0a050", danger: "#e05050", info: "#50a0f0", purple: "#a070e0",
    text: "#d0d4dc", textBright: "#f0f2f5", textDim: "#606878", textMuted: "#404858",
    bar: "#0d0f14", barBorder: "#1a1e28",
    shadow: "0 4px 24px rgba(0,0,0,0.6)", glow: "0 0 20px rgba(92,224,184,0.12)",
};

const STYLES = [
    { id: "tiling", name: "Tiling", compositor: "Sway / DWL / River", desc: "i3-style master-stack" },
    { id: "stacking", name: "Stacking", compositor: "Labwc / Wayfire", desc: "macOS / Windows floating" },
    { id: "dynamic", name: "Dynamic", compositor: "River / Wayfire", desc: "Toggle tiling ↔ floating" },
    { id: "scrollable", name: "Scrollable", compositor: "Niri", desc: "Infinite horizontal strip" },
];

// ─── Shared Components ──────────────────────────────────────

function TrxBar({ clock, style: styleName, compositor }) {
    const workspaces = styleName === "scrollable"
        ? ["←", "•", "•", "◆", "•", "•", "→"]
        : ["α", "β", "γ", "δ", "ε"];

    return (
        <div style={{
            position: "absolute", top: 0, left: 0, right: 0, height: 30,
            background: T.bar, borderBottom: `1px solid ${T.barBorder}`,
            display: "flex", alignItems: "center", padding: "0 12px",
            fontFamily: FONT, fontSize: 11, color: T.textDim, zIndex: 100,
        }}>
            <div style={{ display: "flex", gap: 2 }}>
                {workspaces.map((name, i) => (
                    <span key={i} style={{
                        background: (styleName === "scrollable" ? i === 3 : i === 0) ? T.accentDim : "transparent",
                        color: (styleName === "scrollable" ? i === 3 : i === 0) ? T.accent : T.textDim,
                        border: `1px solid ${(styleName === "scrollable" ? i === 3 : i === 0) ? T.accent + "40" : "transparent"}`,
                        borderRadius: 3, padding: "1px 7px", fontSize: 10,
                    }}>{name}</span>
                ))}
            </div>
            <div style={{ flex: 1, textAlign: "center", fontSize: 10, color: T.text }}>
                trx-foot — ~/terranox-os
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 12, fontSize: 10 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 3 }}>
                    <div style={{ width: 5, height: 5, borderRadius: "50%", background: T.accent, boxShadow: `0 0 4px ${T.accent}` }} />
                    <span style={{ fontSize: 9 }}>SENTINEL</span>
                </div>
                <span>🔊 78%</span>
                <span>⚡ 84%</span>
                <span style={{ color: T.text }}>{clock}</span>
            </div>
        </div>
    );
}

function Win({ title, icon, caps, active, x, y, w, h, shadow, radius, children, zIndex, resize }) {
    return (
        <div style={{
            position: "absolute", left: x, top: y, width: w, height: h,
            background: T.surface,
            border: `1px solid ${active ? T.accent + "60" : T.border}`,
            borderRadius: radius || 0,
            boxShadow: shadow ? T.shadow : active ? T.glow : "none",
            display: "flex", flexDirection: "column",
            overflow: "hidden", zIndex: zIndex || 1,
            transition: "box-shadow 0.2s",
        }}>
            {/* Title bar */}
            <div style={{
                height: 26, background: T.surface2,
                borderBottom: `1px solid ${T.border}`,
                display: "flex", alignItems: "center", padding: "0 8px", gap: 6,
                flexShrink: 0,
            }}>
                {shadow && (
                    <div style={{ display: "flex", gap: 3 }}>
                        <div style={{ width: 7, height: 7, borderRadius: "50%", background: T.danger + "70" }} />
                        <div style={{ width: 7, height: 7, borderRadius: "50%", background: T.warn + "50" }} />
                        <div style={{ width: 7, height: 7, borderRadius: "50%", background: T.accent + "50" }} />
                    </div>
                )}
                <span style={{ fontFamily: FONT, fontSize: 9, color: T.textDim }}>{icon}</span>
                <span style={{ fontFamily: SANS, fontSize: 10, color: T.text, flex: 1 }}>{title}</span>
                {caps && (
                    <div style={{ display: "flex", gap: 2 }}>
                        {caps.map((c, i) => (
                            <span key={i} style={{
                                fontSize: 7, fontFamily: FONT, color: T.accent + "b0",
                                background: T.accent + "12", border: `1px solid ${T.accent}25`,
                                borderRadius: 2, padding: "0 4px",
                            }}>{c}</span>
                        ))}
                    </div>
                )}
            </div>
            {/* Content */}
            <div style={{ flex: 1, padding: "6px 10px", fontFamily: FONT, fontSize: 10, lineHeight: 1.55, overflow: "hidden" }}>
                {children}
            </div>
            {/* Resize handle for stacking */}
            {resize && (
                <div style={{
                    position: "absolute", bottom: 0, right: 0, width: 12, height: 12,
                    cursor: "nwse-resize",
                    background: `linear-gradient(135deg, transparent 50%, ${T.textMuted}40 50%)`,
                }} />
            )}
        </div>
    );
}

function TermContent({ variant }) {
    const lines = variant === "main" ? [
        { c: T.accent, t: "antonette@terranox ~/terranox-os" },
        { c: T.textBright, t: "$ cargo build --release --target x86_64-unknown-none" },
        { c: T.textDim, t: "   Compiling terranox-syscall v0.2.0" },
        { c: T.textDim, t: "   Compiling terranox-vfs v0.2.0" },
        { c: T.accent, t: "    Finished `release` — 91 syscalls" },
        { c: T.textBright, t: "$ cargo test --lib caps" },
        { c: T.accent, t: "test cap_derive_no_escalation ... ok" },
        { c: T.accent, t: "test cap_check_valid_pid ... ok" },
        { c: T.accent, t: "test result: ok. 12 passed; 0 failed" },
        { c: T.accent, t: "antonette@terranox ~/terranox-os" },
        { c: T.textBright, t: "$ _" },
    ] : variant === "htop" ? [
        { c: T.info, t: "  CPU ████████████░░░░░░░ 62.3%" },
        { c: T.accent, t: "  MEM ██████░░░░░░░░░░░░ 34.1%" },
        { c: T.textDim, t: "  PID  USER    CAP         %CPU CMD" },
        { c: T.textBright, t: "  127  root    DRM,INPUT    8.2 compositor" },
        { c: T.textBright, t: "  234  ant     FS_RW,SPAWN  4.1 foot" },
        { c: T.textBright, t: "  289  ant     FS_RW        3.8 helix" },
        { c: T.textBright, t: "  301  ant     FS_R,NET     2.1 firefox" },
        { c: T.warn, t: "  412  ant     FS_R         0.3 nnn" },
        { c: T.textDim, t: "  518  root    DEBUG        0.1 sentinel" },
    ] : [
        { c: T.purple, t: "─── kernel/src/syscall/dispatch.rs ──" },
        { c: T.textDim, t: " 1 │ use crate::caps::cap_check;" },
        { c: T.textDim, t: " 2 │ use crate::process::current_pid;" },
        { c: T.textDim, t: " 3 │ " },
        { c: T.textBright, t: " 4 │ pub fn syscall_entry(" },
        { c: T.textBright, t: " 5 │     regs: &mut CpuRegs" },
        { c: T.textBright, t: " 6 │ ) -> i64 {" },
        { c: T.textDim, t: " 7 │     let nr = regs.rax as usize;" },
        { c: T.accent, t: " 8 │     match SYSCALL_TABLE.get(nr) {" },
        { c: T.textDim, t: " 9 │         Some(h) => {" },
        { c: T.accent, t: "10 │             cap_check(current_pid()," },
        { c: T.accent, t: "11 │                 h.required_cap)?;" },
        { c: T.textDim, t: "12 │             (h.func)(regs)" },
        { c: T.textDim, t: "13 │         }" },
        { c: T.textDim, t: "14 │         None => Err(ENOSYS)" },
        { c: T.textDim, t: "15 │     }" },
        { c: T.textDim, t: "16 │ }" },
    ];

    return <>
        {lines.map((l, i) => <div key={i} style={{ color: l.c, whiteSpace: "nowrap" }}>{l.t}</div>)}
    </>;
}

// ─── Layout: Tiling (i3/Sway) ───────────────────────────────

function TilingLayout() {
    const gap = 3;
    const top = 34;
    const bot = 4;
    return (
        <div style={{ position: "absolute", inset: 0 }}>
            {/* Master — left half */}
            <Win title="trx-foot — ~/terranox-os" icon="›_" active
                caps={["FS_RW", "SPAWN"]}
                x={gap} y={top} w="calc(50% - 4.5px)" h={`calc(100% - ${top + bot}px)`}>
                <TermContent variant="main" />
            </Win>
            {/* Top right */}
            <Win title="helix — dispatch.rs" icon="HX"
                caps={["FS_RW"]}
                x="calc(50% + 1.5px)" y={top} w={`calc(50% - ${gap + 1.5}px)`} h={`calc(50% - ${(top + bot) / 2 + 1.5}px)`}>
                <TermContent variant="editor" />
            </Win>
            {/* Bottom right */}
            <Win title="trx-htop — processes" icon="📊"
                caps={["DEBUG"]}
                x="calc(50% + 1.5px)" y="calc(50% + 1.5px)" w={`calc(50% - ${gap + 1.5}px)`} h={`calc(50% - ${(top + bot) / 2 + 1.5}px)`}>
                <TermContent variant="htop" />
            </Win>
        </div>
    );
}

// ─── Layout: Stacking (macOS/Windows) ───────────────────────

function StackingLayout() {
    return (
        <div style={{ position: "absolute", inset: 0 }}>
            {/* Background window */}
            <Win title="helix — dispatch.rs" icon="HX" shadow radius={8} resize
                caps={["FS_RW"]}
                x="6%" y={50} w="55%" h="70%" zIndex={1}>
                <TermContent variant="editor" />
            </Win>
            {/* Overlapping window */}
            <Win title="trx-foot — ~/terranox-os" icon="›_" active shadow radius={8} resize
                caps={["FS_RW", "SPAWN"]}
                x="25%" y={75} w="55%" h="68%" zIndex={3}>
                <TermContent variant="main" />
            </Win>
            {/* Small floating window */}
            <Win title="trx-htop — processes" icon="📊" shadow radius={8} resize
                caps={["DEBUG"]}
                x="60%" y={46} w="36%" h="42%" zIndex={2}>
                <TermContent variant="htop" />
            </Win>
        </div>
    );
}

// ─── Layout: Dynamic (toggle mode) ─────────────────────────

function DynamicLayout() {
    const [tiled, setTiled] = useState(true);
    const gap = 3;
    const top = 34;
    const bot = 4;

    return (
        <div style={{ position: "absolute", inset: 0 }}>
            {/* Mode toggle indicator */}
            <div style={{
                position: "absolute", top: top + 6, left: "50%", transform: "translateX(-50%)",
                zIndex: 50, display: "flex", gap: 0,
                background: T.surface, border: `1px solid ${T.border}`, borderRadius: 6,
                overflow: "hidden",
            }}>
                <button onClick={() => setTiled(true)} style={{
                    padding: "4px 14px", border: "none", fontFamily: FONT, fontSize: 9, cursor: "pointer",
                    background: tiled ? T.accentDim : "transparent",
                    color: tiled ? T.accent : T.textDim,
                }}>▣ TILED</button>
                <button onClick={() => setTiled(false)} style={{
                    padding: "4px 14px", border: "none", fontFamily: FONT, fontSize: 9, cursor: "pointer",
                    background: !tiled ? T.accentDim : "transparent",
                    color: !tiled ? T.accent : T.textDim,
                }}>▢ FLOAT</button>
            </div>

            {tiled ? (
                <>
                    <Win title="trx-foot — ~/terranox-os" icon="›_" active
                        caps={["FS_RW", "SPAWN"]}
                        x={gap} y={top} w="calc(50% - 4.5px)" h={`calc(100% - ${top + bot}px)`}>
                        <TermContent variant="main" />
                    </Win>
                    <Win title="helix — dispatch.rs" icon="HX"
                        caps={["FS_RW"]}
                        x="calc(50% + 1.5px)" y={top} w={`calc(50% - ${gap + 1.5}px)`} h={`calc(50% - ${(top + bot) / 2 + 1.5}px)`}>
                        <TermContent variant="editor" />
                    </Win>
                    <Win title="trx-htop — processes" icon="📊"
                        caps={["DEBUG"]}
                        x="calc(50% + 1.5px)" y="calc(50% + 1.5px)" w={`calc(50% - ${gap + 1.5}px)`} h={`calc(50% - ${(top + bot) / 2 + 1.5}px)`}>
                        <TermContent variant="htop" />
                    </Win>
                </>
            ) : (
                <>
                    <Win title="helix — dispatch.rs" icon="HX" shadow radius={8} resize
                        caps={["FS_RW"]}
                        x="8%" y={55} w="52%" h="65%" zIndex={1}>
                        <TermContent variant="editor" />
                    </Win>
                    <Win title="trx-foot — ~/terranox-os" icon="›_" active shadow radius={8} resize
                        caps={["FS_RW", "SPAWN"]}
                        x="28%" y={80} w="52%" h="62%" zIndex={3}>
                        <TermContent variant="main" />
                    </Win>
                    <Win title="trx-htop — processes" icon="📊" shadow radius={8} resize
                        caps={["DEBUG"]}
                        x="62%" y={50} w="34%" h="40%" zIndex={2}>
                        <TermContent variant="htop" />
                    </Win>
                </>
            )}
        </div>
    );
}

// ─── Layout: Scrollable (Niri) ──────────────────────────────

function ScrollableLayout() {
    const [scrollX, setScrollX] = useState(0);
    const top = 34;
    const colW = 420;
    const gap = 4;
    const totalW = colW * 5 + gap * 4;

    return (
        <div style={{ position: "absolute", inset: 0, overflow: "hidden" }}>
            {/* Scroll hint arrows */}
            <button onClick={() => setScrollX(Math.min(scrollX + colW + gap, totalW - 800))}
                style={{
                    position: "absolute", left: 8, top: "50%", transform: "translateY(-50%)",
                    zIndex: 50, background: T.surface2, border: `1px solid ${T.border}`,
                    borderRadius: 20, width: 28, height: 28, color: T.accent,
                    fontFamily: FONT, fontSize: 14, cursor: "pointer", display: "flex",
                    alignItems: "center", justifyContent: "center",
                }}>◀</button>
            <button onClick={() => setScrollX(Math.max(scrollX - colW - gap, 0))}
                style={{
                    position: "absolute", right: 8, top: "50%", transform: "translateY(-50%)",
                    zIndex: 50, background: T.surface2, border: `1px solid ${T.border}`,
                    borderRadius: 20, width: 28, height: 28, color: T.accent,
                    fontFamily: FONT, fontSize: 14, cursor: "pointer", display: "flex",
                    alignItems: "center", justifyContent: "center",
                }}>▶</button>

            {/* Scrollable strip */}
            <div style={{
                position: "absolute", top: top, bottom: 4, left: 0,
                display: "flex", gap: gap,
                transform: `translateX(-${scrollX}px)`,
                transition: "transform 0.4s cubic-bezier(0.25, 0.1, 0.25, 1)",
                padding: `0 ${gap}px`,
            }}>
                {/* Column 1: file manager (off-screen left) */}
                <Win title="nnn — ~/projects" icon="📁"
                    caps={["FS_RW"]}
                    x={0} y={0} w={colW} h="100%"
                    style={{ position: "relative", width: colW, flexShrink: 0 }}>
                    <div style={{ color: T.textDim }}>
                        <div style={{ color: T.info }}>  ~/projects/</div>
                        <div>  drwxr-xr-x  terranox-os/</div>
                        <div>  drwxr-xr-x  sigilvm/</div>
                        <div>  drwxr-xr-x  hermetica-os/</div>
                        <div>  drwxr-xr-x  genesis-rt/</div>
                    </div>
                </Win>

                {/* Column 2: terminal */}
                <Win title="trx-foot — ~/terranox-os" icon="›_" active
                    caps={["FS_RW", "SPAWN"]}
                    x={0} y={0} w={colW} h="100%"
                    style={{ position: "relative", width: colW, flexShrink: 0 }}>
                    <TermContent variant="main" />
                </Win>

                {/* Column 3: editor */}
                <Win title="helix — dispatch.rs" icon="HX"
                    caps={["FS_RW"]}
                    x={0} y={0} w={colW} h="100%"
                    style={{ position: "relative", width: colW, flexShrink: 0 }}>
                    <TermContent variant="editor" />
                </Win>

                {/* Column 4: htop */}
                <Win title="trx-htop — processes" icon="📊"
                    caps={["DEBUG"]}
                    x={0} y={0} w={colW} h="100%"
                    style={{ position: "relative", width: colW, flexShrink: 0 }}>
                    <TermContent variant="htop" />
                </Win>

                {/* Column 5: browser (off-screen right) */}
                <Win title="firefox" icon="🌐"
                    caps={["FS_R", "NET"]}
                    x={0} y={0} w={colW} h="100%"
                    style={{ position: "relative", width: colW, flexShrink: 0 }}>
                    <div style={{ color: T.textDim, textAlign: "center", paddingTop: 40 }}>
                        <div style={{ fontSize: 24, marginBottom: 8 }}>🌐</div>
                        <div style={{ color: T.text }}>firefox</div>
                        <div style={{ fontSize: 9, marginTop: 4 }}>scroll → to focus</div>
                    </div>
                </Win>
            </div>

            {/* Scroll position indicator */}
            <div style={{
                position: "absolute", bottom: 10, left: "50%", transform: "translateX(-50%)",
                display: "flex", gap: 4, zIndex: 50,
            }}>
                {[0, 1, 2, 3, 4].map(i => {
                    const colStart = i * (colW + gap);
                    const visible = scrollX <= colStart + colW && scrollX + 800 >= colStart;
                    const focused = Math.abs(scrollX - colStart + 200) < colW / 2;
                    return (
                        <div key={i} onClick={() => setScrollX(Math.max(0, colStart - 200))} style={{
                            width: focused ? 20 : 6, height: 6, borderRadius: 3,
                            background: focused ? T.accent : visible ? T.textDim : T.textMuted,
                            cursor: "pointer", transition: "all 0.3s",
                        }} />
                    );
                })}
            </div>
        </div>
    );
}

// ─── Fix Win for scrollable (allow style override) ──────────
// We need to handle the scrollable case where Win is positioned via flexbox

function WinFlex({ title, icon, caps, active, shadow, radius, resize, children, style: overrideStyle }) {
    return (
        <div style={{
            background: T.surface,
            border: `1px solid ${active ? T.accent + "60" : T.border}`,
            borderRadius: radius || 0,
            boxShadow: shadow ? T.shadow : active ? T.glow : "none",
            display: "flex", flexDirection: "column",
            overflow: "hidden",
            ...overrideStyle,
        }}>
            <div style={{
                height: 26, background: T.surface2,
                borderBottom: `1px solid ${T.border}`,
                display: "flex", alignItems: "center", padding: "0 8px", gap: 6,
                flexShrink: 0,
            }}>
                <span style={{ fontFamily: FONT, fontSize: 9, color: T.textDim }}>{icon}</span>
                <span style={{ fontFamily: SANS, fontSize: 10, color: T.text, flex: 1 }}>{title}</span>
                {caps && (
                    <div style={{ display: "flex", gap: 2 }}>
                        {caps.map((c, i) => (
                            <span key={i} style={{
                                fontSize: 7, fontFamily: FONT, color: T.accent + "b0",
                                background: T.accent + "12", border: `1px solid ${T.accent}25`,
                                borderRadius: 2, padding: "0 4px",
                            }}>{c}</span>
                        ))}
                    </div>
                )}
            </div>
            <div style={{ flex: 1, padding: "6px 10px", fontFamily: FONT, fontSize: 10, lineHeight: 1.55, overflow: "hidden" }}>
                {children}
            </div>
        </div>
    );
}

// ─── Updated Scrollable Layout using flex ────────────────────

function ScrollableLayoutV2() {
    const [scrollX, setScrollX] = useState(0);
    const top = 34;
    const colW = 380;
    const gap = 4;

    const columns = [
        { title: "nnn — ~/projects", icon: "📁", caps: ["FS_RW"], content: "files" },
        { title: "trx-foot — ~/terranox-os", icon: "›_", caps: ["FS_RW", "SPAWN"], content: "main", active: true },
        { title: "helix — dispatch.rs", icon: "HX", caps: ["FS_RW"], content: "editor" },
        { title: "trx-htop — processes", icon: "📊", caps: ["DEBUG"], content: "htop" },
        { title: "firefox", icon: "🌐", caps: ["FS_R", "NET"], content: "browser" },
    ];

    return (
        <div style={{ position: "absolute", inset: 0, overflow: "hidden" }}>
            <button onClick={() => setScrollX(Math.max(scrollX - colW - gap, 0))}
                style={{
                    position: "absolute", left: 8, top: "50%", transform: "translateY(-50%)",
                    zIndex: 50, background: T.surface2, border: `1px solid ${T.border}`,
                    borderRadius: 20, width: 28, height: 28, color: T.accent,
                    fontFamily: FONT, fontSize: 14, cursor: "pointer", display: "flex",
                    alignItems: "center", justifyContent: "center",
                }}>◀</button>
            <button onClick={() => setScrollX(Math.min(scrollX + colW + gap, (columns.length - 2) * (colW + gap)))}
                style={{
                    position: "absolute", right: 8, top: "50%", transform: "translateY(-50%)",
                    zIndex: 50, background: T.surface2, border: `1px solid ${T.border}`,
                    borderRadius: 20, width: 28, height: 28, color: T.accent,
                    fontFamily: FONT, fontSize: 14, cursor: "pointer", display: "flex",
                    alignItems: "center", justifyContent: "center",
                }}>▶</button>

            <div style={{
                position: "absolute", top, bottom: 20, left: gap,
                display: "flex", gap,
                transform: `translateX(-${scrollX}px)`,
                transition: "transform 0.4s cubic-bezier(0.25, 0.1, 0.25, 1)",
            }}>
                {columns.map((col, i) => (
                    <WinFlex key={i} title={col.title} icon={col.icon} caps={col.caps} active={col.active}
                        style={{ width: colW, flexShrink: 0, height: "100%" }}>
                        {col.content === "main" ? <TermContent variant="main" /> :
                            col.content === "editor" ? <TermContent variant="editor" /> :
                                col.content === "htop" ? <TermContent variant="htop" /> :
                                    col.content === "files" ? (
                                        <div style={{ color: T.textDim }}>
                                            <div style={{ color: T.info }}>  ~/projects/</div>
                                            <div>  drwxr-xr-x  terranox-os/</div>
                                            <div>  drwxr-xr-x  sigilvm/</div>
                                            <div>  drwxr-xr-x  hermetica-os/</div>
                                            <div>  drwxr-xr-x  genesis-rt/</div>
                                        </div>
                                    ) : (
                                        <div style={{ color: T.textDim, textAlign: "center", paddingTop: 30 }}>
                                            <div style={{ fontSize: 20, marginBottom: 6 }}>🌐</div>
                                            <div style={{ color: T.text, fontSize: 11 }}>firefox</div>
                                            <div style={{ fontSize: 9, marginTop: 4, color: T.textMuted }}>scroll → to focus</div>
                                        </div>
                                    )}
                    </WinFlex>
                ))}
            </div>

            <div style={{
                position: "absolute", bottom: 6, left: "50%", transform: "translateX(-50%)",
                display: "flex", gap: 4, zIndex: 50,
            }}>
                {columns.map((_, i) => {
                    const isCenter = Math.round(scrollX / (colW + gap)) === i || (scrollX === 0 && i <= 1);
                    return (
                        <div key={i} onClick={() => setScrollX(Math.max(0, i * (colW + gap) - 200))} style={{
                            width: isCenter ? 16 : 6, height: 6, borderRadius: 3,
                            background: isCenter ? T.accent : T.textMuted,
                            cursor: "pointer", transition: "all 0.3s",
                        }} />
                    );
                })}
            </div>
        </div>
    );
}

// ─── Main ───────────────────────────────────────────────────

export default function DesktopComparison() {
    const [activeStyle, setActiveStyle] = useState("tiling");
    const [clock, setClock] = useState("00:00");

    useEffect(() => {
        const tick = () => setClock(new Date().toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", hour12: false }));
        tick();
        const id = setInterval(tick, 1000);
        return () => clearInterval(id);
    }, []);

    const currentStyle = STYLES.find(s => s.id === activeStyle);

    return (
        <div style={{ width: "100%", height: "100vh", background: T.bg, display: "flex", flexDirection: "column", fontFamily: SANS }}>
            <style>{`
        @import url('https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600&family=IBM+Plex+Sans:wght@400;600;700&display=swap');
        * { box-sizing: border-box; margin: 0; padding: 0; }
      `}</style>

            {/* Style selector */}
            <div style={{
                display: "flex", alignItems: "center", gap: 0,
                background: T.surface, borderBottom: `1px solid ${T.border}`,
                flexShrink: 0,
            }}>
                {STYLES.map(s => (
                    <button key={s.id} onClick={() => setActiveStyle(s.id)} style={{
                        flex: 1, padding: "10px 8px", border: "none", cursor: "pointer",
                        background: activeStyle === s.id ? T.accentFaint : "transparent",
                        borderBottom: `2px solid ${activeStyle === s.id ? T.accent : "transparent"}`,
                        display: "flex", flexDirection: "column", alignItems: "center", gap: 2,
                        transition: "all 0.2s",
                    }}>
                        <span style={{ fontFamily: SANS, fontSize: 12, fontWeight: 700, color: activeStyle === s.id ? T.accent : T.textDim }}>{s.name}</span>
                        <span style={{ fontFamily: FONT, fontSize: 9, color: T.textMuted }}>{s.compositor}</span>
                    </button>
                ))}
            </div>

            {/* Desktop area */}
            <div style={{ flex: 1, position: "relative", overflow: "hidden" }}>
                <TrxBar clock={clock} style={activeStyle} compositor={currentStyle.compositor} />

                {activeStyle === "tiling" && <TilingLayout />}
                {activeStyle === "stacking" && <StackingLayout />}
                {activeStyle === "dynamic" && <DynamicLayout />}
                {activeStyle === "scrollable" && <ScrollableLayoutV2 />}
            </div>

            {/* Description bar */}
            <div style={{
                padding: "8px 16px", background: T.surface, borderTop: `1px solid ${T.border}`,
                display: "flex", justifyContent: "space-between", alignItems: "center",
                fontFamily: FONT, fontSize: 10, color: T.textDim, flexShrink: 0,
            }}>
                <span>
                    <span style={{ color: T.accent, fontWeight: 700 }}>{currentStyle.name}</span>
                    <span style={{ color: T.textMuted }}> — {currentStyle.desc}</span>
                </span>
                <span style={{ color: T.textMuted }}>
                    Compositor: {currentStyle.compositor} · trx-shell (Rust) · trx-bar · trx-launcher · trx-notify
                </span>
            </div>
        </div>
    );
}