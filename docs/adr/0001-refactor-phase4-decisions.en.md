# ADR 0001: Phase 4 Refactoring Decisions

- **Status**: Accepted
- **Date**: 2026-10-12
- **Related**: `REFACTOR_ROADMAP.md` (W23 legacy-stack retirement Step 1), `docs/refactor/journal-retire-s1.md` §3 endpoint gap table, retire/audit journals (referenced per decision)

> **Status note (2026-10-13)**: some decisions below have been superseded by the later
> consolidation — for example "Decision 1: Merge TTL into the next ecosystem (ttlfw4 as the
> carrier, into qmodem-suite)": `qmodem-suite` is now retired and TTL ships with `qmodem`,
> while the carrier packages `luci-app-qmodem-next` / `-ttlfw4` are merged into
> `luci-app-qmodem`. The original text is kept for traceability; see
> [integration-scope.zh-cn.md](../refactor/integration-scope.zh-cn.md) for the
> superseding layout.

## Context

Phases 0–3 completed C-layer memory-safety fixes, the LuCI next frontend, the Rust async core (qmodem-pdu / qmodem-atd), and the qmodem-suite meta-package. Phase 4 (legacy-stack retirement and packaging wrap-up) faces 7 pending decisions: the overlapping TTL packages, whether to keep/migrate/retire mwan/voip/HC, when to switch the AT path and PDU packaging to Rust, and how to handle qfirehose's three high-severity vulnerabilities. This ADR records the final decisions based on the retire journals and the gap table.

## Decision 1: Merge TTL into the next ecosystem (ttlfw4 as the carrier, into qmodem-suite)

**Context**: `luci-app-qmodem-ttl` (CBI lua + iptables) and `luci-app-qmodem-ttlfw4` (JS view + fw4/nft) overlap by ~90%, share the `qmodem_ttl` uci section, and conflict bidirectionally via `PKG_CONFLICTS`; `luci-app-qmodem-next` has no TTL view or endpoint (journal-retire-ttl.md §2; journal-retire-s1.md gap table "whole gap" row).

**Decision**:
1. Keep ttlfw4 as the sole carrier (JS UI + fw4/nft backend) and add it to the `qmodem-suite` meta-package dependencies;
2. Add a TTL view to next (pure uci + `rc`/`luci setInitAction`, no new ubus endpoint needed);
3. Retire `luci-app-qmodem-ttl` (the iptables legacy variant) in Phase 4;
4. Fix the stale call to the non-existent `/etc/firewall.d/qmodem_ttl` in ttlfw4 init.d:34 (journal-retire-ttl.md §1 leftover).

**Rationale**: The two packages share the same data model; maintaining both plus the CONFLICTS mutex is pure cost; fw4/nft is the OpenWrt mainline direction and ttlfw4 has lighter dependencies (only `+luci-base`); TTL (bypassing carrier tethering restrictions) still has real demand, so outright removal is inappropriate.

**Rejected alternatives and why**:
- *Keep both packages as-is*: double maintenance cost and a mutually exclusive install experience with no benefit;
- *Keep the legacy ttl as the carrier*: iptables belongs to the fading fw3 era, and it depends on the legacy UI entry `+luci-app-qmodem`, contrary to the retirement direction;
- *Retire without migration*: the feature has demand; removal would force users to hand-write nft/iptables rules.

**Consequences**: the suite gains an fw4 dependency; after retiring ttl, the now-dead side of the bidirectional CONFLICTS must be dropped; the next TTL view becomes a Phase 4 frontend deliverable.

## Decision 2: mwan stays in legacy maintenance mode (no migration, no deprecation, not in suite)

**Context**: `luci-app-qmodem-mwan` is a pure uci→mwan3 offline config generator with no ubus/HTTP custom endpoints; next has no matching page and its ACL lacks `mwan3`/`qmodem_mwan` (journal-retire-mwan.md §2); the feature heavily overlaps the upstream `luci-app-mwan3` official UI, but `99-setup-mwan` presets multi-WAN members for 3 mass-production devices (HC-G80/ailf gs2410/huasifei ws3006), and the `add_by=modem` incremental management in `modem_mwan.sh` (never touching user-written mwan3 sections) does not exist upstream.

**Decision**: keep it in legacy maintenance mode alongside the old stack: no migration to next, no deprecation, no inclusion in qmodem-suite; if next ever needs mwan capability, prefer directing users to `luci-app-mwan3` directly.

**Rejected alternatives and why**:
- *Migrate to next*: requires a new JS view + ACL + backend wiring — high cost for limited value over upstream;
- *Deprecate directly*: breaks factory presets on three mass-production devices;
- *Include in suite*: overlaps the upstream mwan3 ecosystem; folding it into the meta-package adds needless dependency coupling.

**Consequences**: leftover observation item — the Makefile conffiles lists `/etc/config/modem_mwan` by mistake (actual file: `/etc/config/qmodem_mwan`); it changes packaging behavior and will be handled during Phase 4 wrap-up (journal-retire-mwan.md §3.5).

## Decision 3: voip stays independent (not in suite, installed as a single package)

**Context**: `luci-app-qmodem-voip` is an experimental single-line voice frontend with a self-contained dependency chain (`+qmodem-voip +qmodem-sipd +luci-base`) and zero coupling to the legacy `luci-app-qmodem`; next's acl.d/menu.d has zero matches for `voip|sip` (journal-retire-voip.md §2); ACL granularity is sound, no injection/XSS high-risk patterns; maintenance cost estimated at <1 person-day per quarter.

**Decision**: keep it independently; exclude from Phase 4 legacy retirement and from qmodem-suite; keep single-package installation; when the backend (voipd/qmodem_sipd) moves to Rust later, the frontend ubus contract (method names/snapshot fields) is preserved as-is.

**Rejected alternatives and why**:
- *Include in suite*: experimental + hardware-whitelisted (RM520N-GL only); inclusion would widen the meta-package's install surface and dependencies, violating the suite's "core ready-to-use" positioning;
- *Migrate to next*: the feature is orthogonal; next's coverage (modem info/dialing/AT/SMS) excludes the voice chain, so migration yields no benefit;
- *Deprecate*: the cost is minimal with no security burden; deprecation would lose more than maintenance costs.

**Consequences**: browser API evolution (AudioWorklet fallback already built in) and whitelist expansion are the long-term maintenance points; network-plane isolation (voipd/sipd sandboxing) proceeds separately per vendored-audit §4.

## Decision 4: HC — keep luci-app-qmodem-hc until set_sim_slot equivalence is verified on real hardware

**Context**: HC-G80 SIM switching uses `fw_printenv`/`fw_setenv` (uboot env `sim2`) + direct GPIO writes (`/sys/class/gpio/sim`, `/sys/class/gpio/4g` pulse reset), a completely different hardware path from next's `set_sim_slot` (rpcd → vendor AT script such as `AT+QUIMSLOT`); next's ACL has no qmodem_hc entries, so the gap holds (journal-retire-hc.md §1–2).

**Decision**: until the 7-step real-device verification checklist in journal-retire-hc.md §2 completes (vendor set_sim_slot availability → GPIO/env baseline capture → next-path drive observation → next-boot slot write → reset-semantics comparison → reboot persistence → conclusion recorded), `luci-app-qmodem-hc` is kept (legacy annotation + device-level `@TARGET_mediatek_...hc-g80` restriction unchanged). If steps 1–6 prove non-equivalence, next must gain a dedicated HC ubus method (e.g. `qmodem.hc set_sim`) or the package logic must be absorbed as a vendor script before retirement.

**Rejected alternatives and why**:
- *Retire with the legacy stack outright*: breaks HC-G80 SIM switching and the uboot env/GPIO factory consistency;
- *Port the GPIO logic into next immediately*: without verifying whether the vendor AT path can drive HC hardware (checklist steps 1 and 3 undone), the porting may be wasted and introduces a device-specific branch.

**Consequences**: the HC row in the Phase 4 retirement list stays "blocked by verification"; verification requires real hardware scheduling; the known logic suspicion (`_get_netdev` in modem_sim.sh misusing the three-argument `config_load`) stays on record to be confirmed during verification (journal-retire-hc.md §1.3.4).

## Decision 5: at_tool default stays 0 (tom_modem); switching to 2 (Rust atd) requires gray-release criteria

**Context**: `qmodem.main.at_tool` has three paths: 0=tom_modem (default, unchanged behavior), 1=sms_tool_q, 2=qmodem-atd (Rust, unix-socket JSON line protocol; a missing socket errors out without fallback, journal-atd-bridge.md). Rust-side quality work is done (journal-atd-quality, bench-atd) but real-device data is insufficient.

**Decision**:
1. Keep the `at_tool` default at `'0'` (tom_modem);
2. Gray-release criteria for flipping the default to `'2'` (both must hold):
   - N consecutive days (N=7 suggested, defined by CI gating) of fixture replay with zero diff (byte-identical at()/fastat() output across the three paths);
   - 72h stable real-device operation: no daemon crashes, no abnormal fallback, no regressions in dialing/SMS/monitoring.

**Rejected alternatives and why**:
- *Flip the default to 2 immediately*: at_tool=2 semantics are "error out if the chosen tool is absent", so it breaks out-of-the-box where the Rust atd is not installed; the timeout_ms (1s/3s) vs outer watchdog (2s/8s) combination is verified but with a small sample;
- *No criteria, switch by feel*: the gray release becomes unmeasurable and rollback has no trigger.

**Consequences**: the gray-release gate must be automated (fixture replay as a daily CI job); when flipping, update the at-tool-selection docs and UCI comments in the same change; the socket-unreachable fallback to tom_modem (introduced this version) does not change these criteria.

## Decision 6: PDU packaging switch — move to rust/qmodem-pdu once the SDK ships a Rust toolchain

**Context**: rust/qmodem-pdu is at 64 tests green (TS 23.038 conformance + semantic cross-checks against C pdu.c + property/roundtrip, journal-pdu-conform.md) and fixed the SCTS timezone-semantics bug the C version never covered (all timezones beyond ±2:30 were wrong); C `libqmodem-sms/pdu.c` remains the packaging default; whether the build SDK ships a Rust toolchain is uncontrollable (cross-build script `scripts/cross-build-rust.sh` is ready, journal-rust-cross.md).

**Decision**: keep the C implementation as the packaging default; switch packaging to `rust/qmodem-pdu` when the target build SDK provides a Rust toolchain (`cross-build-rust.sh` usable); if the C side later adds timezone parsing, adopt the standard semi-octet semantics (TS 23.040 §9.2.3.11) directly rather than replicating the old bug.

**Rejected alternatives and why**:
- *Switch to Rust fully and immediately*: packaging fails outright on toolchain-less SDKs, breaking release determinism;
- *Maintain both implementations in parallel long-term*: the known semantic divergences (C zeroes total≤1, C lacks the international TOA `+` prefix, C ignores the timezone octet) would drift over time; dual consistency is costly.

**Consequences**: the outdated timezone semantics in qmodem-pdu README:32 await an update (journal-pdu-conform.md §4 leftover); the switch needs a toolchain-probe switch in the packaging layer.

## Decision 7: Handling qfirehose's three high-severity issues (source verification + usb2tcp disabled by default; upstream patch tracking)

**Context**: `docs/security/vendored-audit.md` table #1–#3 (all in upstream QFirehose V1.4.21 proprietary code): ① 8 `system(zip_cmd_buf)` sites in `firehose_protocol.c` etc. — malicious upgrade-package shell injection (high); ② XML `strncpy` without NUL + `fh_xml_set_value` strcpy write-back into a fixed 512-byte buffer — overflow (high); ③ `usb2tcp.c:189-216` `bind(INADDR_ANY)` unauthenticated TCP→USB passthrough — remote flashing (high; enabled only when `usb2tcp_port` is passed via CLI).

**Decision**:
1. Short-term runtime mitigations: upgrade-package source verification (signature check in addition to the built-in md5); disable usb2tcp by default (usable only when `usb2tcp_port` is passed explicitly via CLI, with documented warnings);
2. Long-term: track as an upstream patch item (QFirehose_Linux_Android V1.4.21) and fix via upstream version updates;
3. Operating constraint: run manually by root only.

**Rejected alternatives and why**:
- *Deep local modification of the proprietary source*: the QFirehose license forbids reverse engineering/modification, and a local fork cannot track upstream — a maintenance dead end;
- *Remove qfirehose outright*: Qualcomm-platform flashing depends on it; removal is a feature regression, not a security treatment.

**Consequences**: packaging must enforce "disabled by default" (no wrapper/service enabling usb2tcp installed); record the upstream tracking item in an issue and re-check the three findings whenever the upstream ReleaseNote version changes.

## Overall consequences and checkpoints

- The Phase 4 retirement scope narrows to: luci-app-qmodem-ttl (Decision 1) plus delivering the next TTL view; mwan/voip/HC are kept per the decisions above.
- Both "conditional switches" (Decisions 5 and 6) are gated by measurable criteria; no switches without evidence.
- The qfirehose security mitigations (Decision 7) are release-blocking and land before the other decisions.

## References

- docs/refactor/journal-retire-s1.md (gap table)
- docs/refactor/journal-retire-ttl.md / journal-retire-mwan.md / journal-retire-voip.md / journal-retire-hc.md
- docs/refactor/journal-atd-bridge.md / journal-atd-quality.md / bench-atd.md / journal-tom-modem.md
- docs/refactor/journal-pdu-conform.md / journal-rust-cross.md / journal-pkg-suite.md
- docs/security/vendored-audit.md
