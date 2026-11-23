//! M11 opt-in DP live short-opcode transactions (EXPERIMENTAL, never cave).
//! Native GDB reads/writes are mediated by M10 OS-bound full-ROM identity,
//! *both* independently stopped CPU stubs, interpreter-at-load observation,
//! source-specific opcode signatures, and append-only fsynced ownership log.
//! This is NOT the unproven atomic-stop/JIT/Overlay11 cave safe-point.
//! The user must explicitly acknowledge loss-of-save/crash risks.
use trainer_dp::m11::{DurableRecord, LiveFeature, PatchPlan, Phase};

#[derive(Clone, Debug, serde::Serialize)]
pub struct M11Preview {
    pub plan: PatchPlan,
    pub pid: u32,
    pub process_created_filetime: String,
    pub rom_epoch: String,
    pub arm9_pc: u32,
    pub arm7_pc: u32,
    pub journal_last_phase: Option<Phase>,
    pub write_risk_ack_required: bool,
    pub warning: String,
}
#[cfg(windows)]
mod real {
    use super::*;
    use std::{
        fs::{self, OpenOptions},
        io::Write,
        path::PathBuf,
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };
    use trainer_dp::{
        m10::{same_loaded_rom_session, Identity, Packet},
        m11::{
            apply_guarded, plan, restore_guarded, same_owner_session, signature_preflight,
            GuardedBus,
        },
        GuestReader,
    };
    use trainer_rsp::RspClient;

    static SINGLE_WRITER: OnceLock<Mutex<()>> = OnceLock::new();
    fn writer_lock() -> &'static Mutex<()> {
        SINGLE_WRITER.get_or_init(|| Mutex::new(()))
    }

    struct GdbBus(RspClient);
    impl GuestReader for GdbBus {
        fn read_guest(&mut self, address: u32, length: usize) -> Result<Vec<u8>, String> {
            if length == 0 || length > 72 {
                return Err("M11 refuses unbounded Guest read".into());
            }
            self.0.read(address, length).map_err(|e| e.to_string())
        }
    }
    impl GuardedBus for GdbBus {
        fn write_guest(&mut self, address: u32, bytes: &[u8]) -> Result<(), String> {
            if !matches!(bytes.len(), 2 | 72) || !address.is_multiple_of(2) {
                return Err(
                    "M11 source-only halfword or unreferenced 72B Cave write required".into(),
                );
            }
            self.0.write(address, bytes).map_err(|e| e.to_string())
        }
    }

    struct Session {
        plan: PatchPlan,
        identity: Identity,
        packet: Packet,
        arm9: GdbBus,
        _arm7: RspClient,
        pc9: u32,
        pc7: u32,
    }
    fn open_stopped(
        pid: u32,
        code: &str,
        rev: u8,
        port9: u16,
        port7: u16,
        feature: LiveFeature,
    ) -> Result<Session, String> {
        let p = plan(code, rev, feature)?;
        let (packet, identity) =
            trainer_runtime_win::read_dp_core_identity(pid, code, rev, port9, port7)?;
        if !identity.diagnostic_identity_verified
            || !identity.full_rom_matches_catalog
            || identity.core_sha256 != p.expected_rom_sha256
        {
            return Err("M11 cannot write: running ROM must match trusted complete retail SHA256 with OS/PID/GDB binding".into());
        }
        if identity.jit_enabled_when_loaded {
            return Err("M11 cannot write: JIT was enabled when this ROM loaded; use a separately tested interpreter-only DeSmuME build".into());
        }
        let mut arm9 = RspClient::connect_local(port9)
            .map_err(|e| format!("M11 ARM9 stop connection: {e}"))?;
        let mut arm7 = RspClient::connect_local(port7)
            .map_err(|e| format!("M11 ARM7 stop connection: {e}"))?;
        if !arm9.is_paused() || !arm7.is_paused() {
            return Err(
                "M11 requires manually stopped ARM9 and ARM7; does not auto-interrupt/resume"
                    .into(),
            );
        }
        let pc9 = arm9.pc().map_err(|e| format!("ARM9 PC: {e}"))?;
        let pc7 = arm7.pc().map_err(|e| format!("ARM7 PC: {e}"))?;
        if arm9.pc().map_err(|e| e.to_string())? != pc9
            || arm7.pc().map_err(|e| e.to_string())? != pc7
        {
            return Err("M11 stopped Guest CPU PC unstable; refuse patch".into());
        }
        // Do not patch an instruction which ARM9 is currently executing.
        if (pc9 & !1).abs_diff(p.address) < 32 {
            return Err("M11 ARM9 PC is within 32 bytes of patch site".into());
        }
        let (again, second) =
            trainer_runtime_win::read_dp_core_identity(pid, code, rev, port9, port7)?;
        if !same_loaded_rom_session(&packet, &again) || !second.diagnostic_identity_verified {
            return Err("M11 ROM/process session changed while checking CPU stops".into());
        }
        Ok(Session {
            plan: p,
            identity,
            packet,
            arm9: GdbBus(arm9),
            _arm7: arm7,
            pc9,
            pc7,
        })
    }
    fn ensure_still_same(
        s: &Session,
        pid: u32,
        code: &str,
        rev: u8,
        p9: u16,
        p7: u16,
    ) -> Result<(), String> {
        let (again, identity) = trainer_runtime_win::read_dp_core_identity(pid, code, rev, p9, p7)?;
        if !same_loaded_rom_session(&s.packet, &again) || !identity.diagnostic_identity_verified {
            return Err("M11 core ROM identity/epoch invalidated during write transaction".into());
        }
        Ok(())
    }
    fn journal_path(plan: &PatchPlan) -> Result<PathBuf, String> {
        let root = std::env::var_os("LOCALAPPDATA")
            .ok_or("LOCALAPPDATA unavailable: cannot create durable ownership record")?;
        let folder = PathBuf::from(root)
            .join("GameTrainer")
            .join("DP-M11-journal");
        fs::create_dir_all(&folder).map_err(|e| format!("M11 create durable journal dir: {e}"))?;
        Ok(folder.join(format!("{}-{}.jsonl", plan.game_code, plan.feature.id())))
    }
    fn read_last(plan: &PatchPlan) -> Result<Option<DurableRecord>, String> {
        let path = journal_path(plan)?;
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&path).map_err(|e| format!("M11 journal read: {e}"))?;
        let mut last = None;
        for line in contents.lines() {
            if line.trim().is_empty() {
                return Err("M11 journal corrupt: blank row".into());
            }
            let entry: DurableRecord =
                serde_json::from_str(line).map_err(|e| format!("M11 journal row invalid: {e}"))?;
            if entry.plan.game_code != plan.game_code
                || entry.plan.feature != plan.feature
                || entry.plan.address != plan.address
                || entry.plan.original != plan.original
                || entry.plan.replacement != plan.replacement
            {
                return Err("M11 journal contains a conflicting source/patch entry".into());
            }
            last = Some(entry);
        }
        Ok(last)
    }
    fn append(row: &DurableRecord) -> Result<(), String> {
        let path = journal_path(&row.plan)?;
        let line = serde_json::to_string(row).map_err(|e| format!("M11 journal serialize: {e}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("M11 journal open: {e}"))?;
        writeln!(file, "{line}").map_err(|e| format!("M11 journal write: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("M11 journal fsync: {e}"))
    }
    pub fn preview(
        pid: u32,
        code: &str,
        rev: u8,
        p9: u16,
        p7: u16,
        feature: LiveFeature,
    ) -> Result<M11Preview, String> {
        let _guard = writer_lock()
            .lock()
            .map_err(|_| "M11 write mutex poisoned")?;
        let mut session = open_stopped(pid, code, rev, p9, p7, feature)?;
        signature_preflight(&mut session.arm9, &session.plan)?;
        let last = read_last(&session.plan)?;
        if last.as_ref().is_some_and(|x| x.phase != Phase::Restored) {
            return Err("M11 an earlier patch journal is pending/active/uncertain; restore or diagnose before another apply".into());
        }
        Ok(M11Preview{
            plan:session.plan,pid,process_created_filetime:session.identity.process_created_filetime,
            rom_epoch:session.identity.rom_epoch,
            arm9_pc:session.pc9,arm7_pc:session.pc7,
            journal_last_phase:last.map(|x|x.phase),
            write_risk_ack_required:true,
            warning:"EXPERIMENTAL Guest code write: two independent CPU stops are not atomic; interpreter-at-load is not a runtime cache guarantee. Back up SAV/savestate and stop all other Cheat Engine tools.".into(),
        })
    }
    pub fn apply(
        pid: u32,
        code: &str,
        rev: u8,
        p9: u16,
        p7: u16,
        feature: LiveFeature,
        ack: &str,
    ) -> Result<DurableRecord, String> {
        if ack != "I_ACCEPT_DP_M11_EXPERIMENTAL_GUEST_WRITES" {
            return Err("M11 explicit experimental Guest write acknowledgement missing".into());
        }
        let _guard = writer_lock()
            .lock()
            .map_err(|_| "M11 write mutex poisoned")?;
        let mut session = open_stopped(pid, code, rev, p9, p7, feature)?;
        signature_preflight(&mut session.arm9, &session.plan)?;
        if read_last(&session.plan)?.is_some_and(|x| x.phase != Phase::Restored) {
            return Err(
                "M11 previous durable journal is not restored; refuse overlapping ownership".into(),
            );
        }
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let mut record = DurableRecord {
            owner_id: format!("dp-m11-{pid}-{id}"),
            phase: Phase::Prepared,
            process_id: pid,
            process_created_filetime: session.identity.process_created_filetime.clone(),
            rom_epoch: session.identity.rom_epoch.clone(),
            arm9_port: p9,
            arm7_port: p7,
            plan: session.plan.clone(),
        };
        // Fsync intent BEFORE any Guest executable memory mutation.
        append(&record)?;
        let result = apply_guarded(&mut session.arm9, &record.plan)
            .and_then(|()| ensure_still_same(&session, pid, code, rev, p9, p7));
        record.phase = if result.is_ok() {
            Phase::Applied
        } else {
            Phase::Uncertain
        };
        let saved = append(&record);
        if let Err(e) = saved {
            return Err(format!("M11 CRITICAL: write phase result cannot be fsynced: {e}; inspect journal and Guest before continuing"));
        }
        result.map_err(|e| format!("M11 Guest write uncertain, durable journal preserved: {e}"))?;
        Ok(record)
    }
    pub fn restore(
        pid: u32,
        code: &str,
        rev: u8,
        p9: u16,
        p7: u16,
        feature: LiveFeature,
    ) -> Result<DurableRecord, String> {
        let _guard = writer_lock()
            .lock()
            .map_err(|_| "M11 write mutex poisoned")?;
        let mut session = open_stopped(pid, code, rev, p9, p7, feature)?;
        let mut record =
            read_last(&session.plan)?.ok_or("M11 no owned live patch in durable journal")?;
        if record.phase == Phase::Restored {
            return Err("M11 patch already restored".into());
        }
        if !same_owner_session(
            &record,
            pid,
            &session.identity.process_created_filetime,
            &session.identity.rom_epoch,
            &session.identity.core_sha256,
            p9,
            p7,
        ) {
            return Err("M11 journal ownership belongs to a different ROM/process/epoch; refusing blind restoration".into());
        }
        let result = restore_guarded(&mut session.arm9, &record.plan)
            .and_then(|()| ensure_still_same(&session, pid, code, rev, p9, p7));
        record.phase = if result.is_ok() {
            Phase::Restored
        } else {
            Phase::Uncertain
        };
        append(&record)?;
        result.map_err(|e| format!("M11 original restore not proven; journal retained: {e}"))?;
        Ok(record)
    }
}
#[cfg(windows)]
#[tauri::command]
pub async fn m11_dp_preview(
    process_id: u32,
    game_code: String,
    revision: u8,
    arm9_port: u16,
    arm7_port: u16,
    feature: LiveFeature,
) -> Result<M11Preview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        real::preview(
            process_id, &game_code, revision, arm9_port, arm7_port, feature,
        )
    })
    .await
    .map_err(|e| format!("M11 preview worker failed: {e}"))?
}
#[cfg(windows)]
#[tauri::command]
pub async fn m11_dp_apply(
    process_id: u32,
    game_code: String,
    revision: u8,
    arm9_port: u16,
    arm7_port: u16,
    feature: LiveFeature,
    experimental_ack: String,
) -> Result<DurableRecord, String> {
    tauri::async_runtime::spawn_blocking(move || {
        real::apply(
            process_id,
            &game_code,
            revision,
            arm9_port,
            arm7_port,
            feature,
            &experimental_ack,
        )
    })
    .await
    .map_err(|e| format!("M11 apply worker failed: {e}"))?
}
#[cfg(windows)]
#[tauri::command]
pub async fn m11_dp_restore(
    process_id: u32,
    game_code: String,
    revision: u8,
    arm9_port: u16,
    arm7_port: u16,
    feature: LiveFeature,
) -> Result<DurableRecord, String> {
    tauri::async_runtime::spawn_blocking(move || {
        real::restore(
            process_id, &game_code, revision, arm9_port, arm7_port, feature,
        )
    })
    .await
    .map_err(|e| format!("M11 restore worker failed: {e}"))?
}
#[cfg(not(windows))]
#[tauri::command]
pub async fn m11_dp_preview(
    _process_id: u32,
    _game_code: String,
    _revision: u8,
    _arm9_port: u16,
    _arm7_port: u16,
    _feature: LiveFeature,
) -> Result<M11Preview, String> {
    Err("M11 live Guest write laboratory only supported on Windows".into())
}
#[cfg(not(windows))]
#[tauri::command]
pub async fn m11_dp_apply(
    _process_id: u32,
    _game_code: String,
    _revision: u8,
    _arm9_port: u16,
    _arm7_port: u16,
    _feature: LiveFeature,
    _experimental_ack: String,
) -> Result<DurableRecord, String> {
    Err("M11 live Guest write laboratory only supported on Windows".into())
}
#[cfg(not(windows))]
#[tauri::command]
pub async fn m11_dp_restore(
    _process_id: u32,
    _game_code: String,
    _revision: u8,
    _arm9_port: u16,
    _arm7_port: u16,
    _feature: LiveFeature,
) -> Result<DurableRecord, String> {
    Err("M11 live Guest write laboratory only supported on Windows".into())
}
