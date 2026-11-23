//! Strict, opt-in mGBA GDB Guest memory transactions for retail FireRed/LeafGreen.
//! This is not CE host-process memory access, a general-purpose patch interface,
//! or a validated in-game end-to-end test. ROM SHA1, all exact function bytes,
//! single owner, paused CPU, SQLite intents and readback are mandatory.
use crate::{profile_from_header, FrlgRomProfile};
use serde::Serialize;
use sha1::{Digest as _, Sha1};
use sha2::Sha256;
use std::time::{Duration, Instant};
use thiserror::Error;
use trainer_rsp::{RspClient, RspError};
use trainer_storage::{ActionEvent, AuditError, AuditStore, EventContext};
use uuid::Uuid;

const ROM_BASE: u32 = 0x0800_0000;
const ROM_SIZE: usize = 16 * 1024 * 1024;
const CAVE_START: u32 = 0x08FF_0000;
const CAVE_SIZE: usize = 0x10000;
const MAIL: u32 = 0x0203_FFC0;
const WILD_PAYLOAD_LEN: usize = 116;
const WILD_PAYLOAD_SHA256: &str =
    "83a069853f427e86bc62c2ed3a56741c94ed13301d58321b02a5ac39698fed6a";
const PREVIEW_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Error)]
pub enum LiveError {
    #[error("mGBA GDB: {0}")]
    Rsp(#[from] RspError),
    #[error("SQLite audit: {0}")]
    Audit(#[from] AuditError),
    #[error("unsupported FRLG ROM or exact retail SHA1 mismatch")]
    Rom,
    #[error("a preview or live patch is already owned")]
    Busy,
    #[error("no live session, preview or owned patch")]
    Missing,
    #[error("preview mismatch or expired")]
    Token,
    #[error("guest bytes, ROM hash, mailbox, or session no longer match")]
    Conflict,
    #[error("cannot install or remove an experimental hook in battle")]
    Battle,
    #[error("the v2.0.4 user-supplied wild Thumb payload differs from the trusted source")]
    Payload,
    #[error("recovery did not verify; keep mGBA CPU stopped")]
    Recovery,
    #[error("operation does not apply to this feature")]
    Feature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Feature {
    Capture,
    Wall,
    Wild,
}
impl Feature {
    fn id(self) -> &'static str {
        match self {
            Self::Capture => "features.frlg.capture",
            Self::Wall => "features.frlg.wall",
            Self::Wild => "features.frlg.wild.seed",
        }
    }
    fn parse(id: &str) -> Result<Self, LiveError> {
        match id {
            "capture" => Ok(Self::Capture),
            "wall" => Ok(Self::Wall),
            "wild" => Ok(Self::Wild),
            _ => Err(LiveError::Feature),
        }
    }
}

#[derive(Clone)]
struct Span {
    label: &'static str,
    address: u32,
    before: Vec<u8>,
    after: Vec<u8>,
}
struct Preview {
    token: String,
    feature: Feature,
    expires: Instant,
    spans: Vec<Span>,
}
struct Owner {
    id: String,
    feature: Feature,
    spans: Vec<Span>,
    restore_failed: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub session_id: String,
    pub game_id: String,
    pub profile_id: String,
    pub game_code: String,
    pub revision: u8,
    pub language: String,
    pub rom_sha1: String,
    pub transport: String,
    pub guest_paused: bool,
    pub active_feature: Option<String>,
    pub active_state: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PreviewSpan {
    pub label: String,
    pub address: String,
    pub before: String,
    pub after: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Diff {
    pub token: String,
    pub feature_id: String,
    pub rom_sha1: String,
    pub expires_in_seconds: u64,
    pub spans: Vec<PreviewSpan>,
    pub experimental: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct Receipt {
    pub owner_id: String,
    pub feature_id: String,
    pub status: String,
    pub guest_running: bool,
    pub audit_recorded: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct WildMailbox {
    pub target_seed: u32,
    pub mode: u8,
    pub active_hook: u8,
    pub last_trigger: u8,
    pub hit_count: u32,
    pub last_old_seed: u32,
}
pub struct Editor {
    rsp: RspClient,
    audit: AuditStore,
    session: String,
    profile: FrlgRomProfile,
    preview: Option<Preview>,
    owner: Option<Owner>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|x| format!("{x:02x}")).collect()
}
fn unhex(src: &str) -> Result<Vec<u8>, LiveError> {
    if src.len() % 2 != 0 {
        return Err(LiveError::Conflict);
    }
    src.as_bytes()
        .chunks_exact(2)
        .map(|p| {
            let s = std::str::from_utf8(p).map_err(|_| LiveError::Conflict)?;
            u8::from_str_radix(s, 16).map_err(|_| LiveError::Conflict)
        })
        .collect()
}
fn overlaps(a: &Span, b: &Span) -> bool {
    let a_end = a.address as u64 + a.before.len() as u64;
    let b_end = b.address as u64 + b.before.len() as u64;
    (a.address as u64) < b_end && (b.address as u64) < a_end
}
fn encode_jump(cave: u32) -> [u8; 8] {
    let mut stub = [0x00, 0x4B, 0x18, 0x47, 0, 0, 0, 0];
    stub[4..].copy_from_slice(&(cave | 1).to_le_bytes());
    stub
}
fn patch_wild_template(
    template: &[u8],
    expected_entry: &[u8],
    seed: u32,
    return_at: u32,
) -> Result<Vec<u8>, LiveError> {
    if template.len() != WILD_PAYLOAD_LEN
        || hex(&Sha256::digest(template)) != WILD_PAYLOAD_SHA256
        || expected_entry.len() != 8
        || template[0x46..0x4E] != [0xAA, 0xAB, 0xAC, 0xAD, 0xAE, 0xAF, 0xB0, 0xB1]
        || template[0x5C..0x60] != [0x11; 4]
        || template[0x70..0x74] != [0x23, 0x22, 0x22, 0x22]
    {
        return Err(LiveError::Payload);
    }
    let mut bytes = template.to_vec();
    bytes[0x46..0x4E].copy_from_slice(expected_entry);
    bytes[0x5C..0x60].copy_from_slice(&seed.to_le_bytes());
    bytes[0x70..0x74].copy_from_slice(&(return_at | 1).to_le_bytes());
    Ok(bytes)
}

impl Editor {
    pub fn connect(port: u16, audit: AuditStore) -> Result<Self, LiveError> {
        let mut rsp = RspClient::connect_local(port)?;
        let header = rsp.read(ROM_BASE, 0xC0)?;
        let profile = profile_from_header(&header).map_err(|_| LiveError::Rom)?;
        let hash = Self::hash_guest_rom(&mut rsp, None)?;
        if hash != profile.sha1 {
            return Err(LiveError::Rom);
        }
        Ok(Self {
            rsp,
            audit,
            session: Uuid::new_v4().to_string(),
            profile,
            preview: None,
            owner: None,
        })
    }
    pub fn has_owner(&self) -> bool {
        self.owner.is_some()
    }
    pub fn status(&self) -> Status {
        Status {
            session_id: self.session.clone(),
            game_id: self.profile.game_id.clone(),
            profile_id: self.profile.profile_id.clone(),
            game_code: self.profile.game_code.clone(),
            revision: self.profile.revision,
            language: self.profile.language.clone(),
            rom_sha1: self.profile.sha1.clone(),
            transport: "localhost mGBA GDB RSP".into(),
            guest_paused: self.rsp.is_paused(),
            active_feature: self.owner.as_ref().map(|o| o.feature.id().into()),
            active_state: self.owner.as_ref().map(|o| {
                if o.restore_failed {
                    "restore_failed".into()
                } else {
                    "active".into()
                }
            }),
        }
    }
    pub fn disconnect(&mut self) -> Result<(), LiveError> {
        if self.owner.is_some() {
            return Err(LiveError::Busy);
        }
        self.preview = None;
        if self.rsp.is_paused() {
            self.rsp.resume()?;
        }
        Ok(())
    }
    /// Normalize only verified owner bytes to their ROM retail originals before SHA1.
    /// Never adopt or restore an unknown third-party ROM patch or stale save-state.
    fn hash_guest_rom(rsp: &mut RspClient, owner: Option<&Owner>) -> Result<String, LiveError> {
        if !rsp.is_paused() {
            return Err(LiveError::Conflict);
        }
        let rom_spans = owner
            .map(|o| {
                o.spans
                    .iter()
                    .filter(|s| (ROM_BASE..CAVE_START + CAVE_SIZE as u32).contains(&s.address))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for span in &rom_spans {
            let now = rsp.read(span.address, span.after.len())?;
            if now != span.after && now != span.before {
                return Err(LiveError::Conflict);
            }
        }
        let mut sha = Sha1::new();
        let mut off = 0usize;
        while off < ROM_SIZE {
            let at = ROM_BASE + off as u32;
            let count = (ROM_SIZE - off).min(256);
            let mut data = rsp.read(at, count)?;
            for span in &rom_spans {
                let start = (span.address as i64 - at as i64).max(0) as usize;
                let limit = ((span.address as i64 + span.before.len() as i64 - at as i64)
                    .min(count as i64))
                    .max(0) as usize;
                for index in start..limit {
                    let pos = (at as u64 + index as u64 - span.address as u64) as usize;
                    if data[index] != span.after[pos] && data[index] != span.before[pos] {
                        return Err(LiveError::Conflict);
                    }
                    data[index] = span.before[pos];
                }
            }
            sha.update(&data);
            off += count;
        }
        Ok(hex(&sha.finalize()))
    }
    fn verify_rom(&mut self) -> Result<(), LiveError> {
        let header = self.rsp.read(ROM_BASE, 0xC0)?;
        let current = profile_from_header(&header).map_err(|_| LiveError::Conflict)?;
        if current.profile_id != self.profile.profile_id {
            return Err(LiveError::Conflict);
        }
        if Self::hash_guest_rom(&mut self.rsp, self.owner.as_ref())? != self.profile.sha1 {
            return Err(LiveError::Conflict);
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn event(
        &mut self,
        kind: &str,
        status: &str,
        feature: Feature,
        owner: Option<&str>,
        token: Option<&str>,
        description: &str,
        error: Option<&str>,
    ) -> Result<(), LiveError> {
        self.audit.append(&ActionEvent::new(
            Uuid::new_v4().to_string(),
            EventContext {
                owner_id: owner.map(str::to_owned),
                preview_id: token.map(str::to_owned),
                session_id: self.session.clone(),
                game_id: self.profile.game_id.clone(),
                feature_id: feature.id().into(),
                environment: "live-frlg".into(),
            },
            kind,
            status,
            description,
            error,
        ))?;
        Ok(())
    }
    fn verify_halfword_context(&mut self, feature: Feature) -> Result<Span, LiveError> {
        let (address, context, original, replacement, label) = match feature {
            Feature::Capture => (
                self.profile.capture_patch,
                &self.profile.expected_capture_context,
                [0x2F, 0xD9],
                [0xC0, 0x46],
                "Cmd_handleballthrow / BLS to Thumb NOP",
            ),
            Feature::Wall => (
                self.profile.wall_patch,
                &self.profile.expected_wall_context,
                [0xC0, 0x20],
                [0x00, 0x21],
                "Collision halfword / MOVS r1, #0",
            ),
            Feature::Wild => return Err(LiveError::Feature),
        };
        let expected = unhex(context)?;
        let start = address.checked_sub(4).ok_or(LiveError::Conflict)?;
        if expected.len() != 10 || expected[4..6] != original {
            return Err(LiveError::Conflict);
        }
        if self.rsp.read(start, expected.len())? != expected {
            return Err(LiveError::Conflict);
        }
        if feature == Feature::Capture {
            let entry = unhex(&self.profile.expected_capture_entry)?;
            if self.rsp.read(self.profile.capture_entry, entry.len())? != entry
                || address < self.profile.capture_entry
                || address - self.profile.capture_entry >= 0x400
            {
                return Err(LiveError::Conflict);
            }
        }
        Ok(Span {
            address,
            label,
            before: original.to_vec(),
            after: replacement.to_vec(),
        })
    }
    fn safe_overworld(&mut self) -> Result<(), LiveError> {
        // pret/pokefirered: gMain.callback2 is at +4 and inBattle at +0x439.
        let byte = self.rsp.read(self.profile.callback2 + 0x435, 1)?[0];
        if byte & 0x02 != 0 {
            return Err(LiveError::Battle);
        }
        Ok(())
    }
    fn find_empty_cave(&mut self) -> Result<u32, LiveError> {
        let area = self.rsp.read(CAVE_START, CAVE_SIZE)?;
        for start in (0..=CAVE_SIZE - WILD_PAYLOAD_LEN).step_by(4) {
            if area[start..start + WILD_PAYLOAD_LEN]
                .iter()
                .all(|b| *b == 0xFF)
            {
                return Ok(CAVE_START + start as u32);
            }
        }
        Err(LiveError::Conflict)
    }
    pub fn preview(
        &mut self,
        feature_name: &str,
        wild_template: Option<&[u8]>,
    ) -> Result<Diff, LiveError> {
        if self.preview.is_some() || self.owner.is_some() {
            return Err(LiveError::Busy);
        }
        let feature = Feature::parse(feature_name)?;
        self.verify_rom()?;
        let spans = if feature == Feature::Wild {
            self.safe_overworld()?;
            let template = wild_template.ok_or(LiveError::Payload)?;
            let original = unhex(&self.profile.expected_wild_entry)?;
            if self.rsp.read(self.profile.wild_entry, original.len())? != original {
                return Err(LiveError::Conflict);
            }
            let mail_before = self.rsp.read(MAIL, 16)?;
            if mail_before.iter().any(|byte| *byte != 0) {
                return Err(LiveError::Conflict);
            }
            let cave = self.find_empty_cave()?;
            let payload = patch_wild_template(
                template,
                &original,
                self.profile.rng_seed,
                self.profile.wild_entry + 8,
            )?;
            vec![
                Span {
                    label: "Wild Seed mail (must be clean zero)",
                    address: MAIL,
                    before: mail_before,
                    after: vec![0; 16],
                },
                Span {
                    label: "Staged v2.0.4 116B Thumb payload",
                    address: cave,
                    before: vec![0xFF; WILD_PAYLOAD_LEN],
                    after: payload,
                },
                Span {
                    label: "TryGenerateWildMon entry (activate LAST)",
                    address: self.profile.wild_entry,
                    before: original,
                    after: encode_jump(cave).to_vec(),
                },
            ]
        } else {
            vec![self.verify_halfword_context(feature)?]
        };
        for (i, span) in spans.iter().enumerate() {
            if span.before.len() != span.after.len() || span.before.len() > 512 {
                return Err(LiveError::Conflict);
            }
            for other in spans.iter().take(i) {
                if overlaps(span, other) {
                    return Err(LiveError::Conflict);
                }
            }
            if self.rsp.read(span.address, span.before.len())? != span.before {
                return Err(LiveError::Conflict);
            }
        }
        let token = Uuid::new_v4().to_string();
        let result = Diff {
            token: token.clone(),
            feature_id: feature.id().into(),
            rom_sha1: self.profile.sha1.clone(),
            expires_in_seconds: PREVIEW_TTL.as_secs(),
            spans: spans
                .iter()
                .map(|span| PreviewSpan {
                    label: span.label.to_string(),
                    address: format!("0x{:08X}", span.address),
                    before: hex(&span.before),
                    after: hex(&span.after),
                })
                .collect(),
            experimental: true,
        };
        self.event(
            "preview",
            "ok",
            feature,
            None,
            Some(&token),
            "FRLG paused CPU preview, no guest mutation",
            None,
        )?;
        self.preview = Some(Preview {
            token,
            feature,
            expires: Instant::now() + PREVIEW_TTL,
            spans,
        });
        Ok(result)
    }
    pub fn cancel_preview(&mut self) -> Result<(), LiveError> {
        let plan = self.preview.take().ok_or(LiveError::Missing)?;
        self.event(
            "preview_cancel",
            "ok",
            plan.feature,
            None,
            Some(&plan.token),
            "FRLG preview dropped without guest mutation",
            None,
        )
    }
    pub fn apply(&mut self, token: &str) -> Result<Receipt, LiveError> {
        let plan = self.preview.take().ok_or(LiveError::Missing)?;
        if plan.token != token || Instant::now() >= plan.expires {
            return Err(LiveError::Token);
        }
        self.verify_rom()?;
        if plan.feature == Feature::Wild {
            self.safe_overworld()?;
        } else {
            self.verify_halfword_context(plan.feature)?;
        }
        if Instant::now() >= plan.expires {
            return Err(LiveError::Token);
        }
        for span in &plan.spans {
            if self.rsp.read(span.address, span.before.len())? != span.before {
                return Err(LiveError::Conflict);
            }
        }
        let id = Uuid::new_v4().to_string();
        self.event(
            "apply_intent",
            "pending",
            plan.feature,
            Some(&id),
            Some(token),
            "FRLG intent committed before paused guest mutation",
            None,
        )?;
        let mut written = 0;
        for span in &plan.spans {
            if span.before != span.after && self.rsp.write(span.address, &span.after).is_err() {
                break;
            }
            written += 1;
        }
        if written != plan.spans.len() {
            let rollback = self.rollback_spans(&plan.spans);
            if rollback.is_ok() {
                self.event(
                    "rollback",
                    "ok",
                    plan.feature,
                    Some(&id),
                    Some(token),
                    "FRLG original guest bytes restored",
                    None,
                )?;
                return Err(LiveError::Conflict);
            }
            self.owner = Some(Owner {
                id: id.clone(),
                feature: plan.feature,
                spans: plan.spans,
                restore_failed: true,
            });
            let _ = self.event(
                "restore_failed",
                "error",
                plan.feature,
                Some(&id),
                None,
                "FRLG uncertain partial write; keep Guest paused",
                Some("PARTIAL_WRITE"),
            );
            return Err(LiveError::Recovery);
        }
        self.owner = Some(Owner {
            id: id.clone(),
            feature: plan.feature,
            spans: plan.spans,
            restore_failed: false,
        });
        self.event(
            "applied",
            "ok",
            plan.feature,
            Some(&id),
            Some(token),
            "FRLG guest writes acknowledged and readback verified",
            None,
        )?;
        self.rsp.resume()?;
        Ok(Receipt {
            owner_id: id,
            feature_id: plan.feature.id().into(),
            status: "active".into(),
            guest_running: true,
            audit_recorded: true,
        })
    }
    fn rollback_spans(&mut self, spans: &[Span]) -> Result<(), LiveError> {
        // Entry pointer always first to restore (reverse install order).
        for span in spans.iter().rev() {
            let current = self.rsp.read(span.address, span.before.len())?;
            if current == span.before {
                continue;
            }
            if current != span.after {
                return Err(LiveError::Conflict);
            }
            self.rsp.write(span.address, &span.before)?;
        }
        Ok(())
    }
    /// Opt-in one-shot ordinary encounters only. Source v2.0.4 intentionally
    /// uses flags==1 and sets saved flags=0 for that call; Sweet Scent is separate.
    pub fn arm_wild_once(&mut self, target_seed: u32) -> Result<WildMailbox, LiveError> {
        if self.owner.as_ref().map(|o| o.feature) != Some(Feature::Wild) {
            return Err(LiveError::Feature);
        }
        if !self.rsp.is_paused() {
            self.rsp.interrupt()?;
        }
        self.verify_rom()?;
        self.safe_overworld()?;
        let mail = self.rsp.read(MAIL, 16)?;
        if !matches!(mail[4], 0 | 1) || !matches!(mail[5], 0 | 1) || mail[7] != 0 {
            return Err(LiveError::Conflict);
        }
        if mail[4] != 0 {
            return Err(LiveError::Busy);
        }
        let owner_id = self.owner.as_ref().ok_or(LiveError::Missing)?.id.clone();
        self.event(
            "wild_arm_intent",
            "pending",
            Feature::Wild,
            Some(&owner_id),
            None,
            "One-shot wild Seed mailbox armed on paused Guest",
            None,
        )?;
        let mut armed = target_seed.to_le_bytes().to_vec();
        armed.extend_from_slice(&[1, 1]);
        if self.rsp.write(MAIL, &armed).is_err() {
            // Do not resume Guest if the mailbox is uncertain.
            return Err(LiveError::Recovery);
        }
        self.event(
            "wild_armed",
            "ok",
            Feature::Wild,
            Some(&owner_id),
            None,
            "One-shot flags==1 wild path armed",
            None,
        )?;
        self.rsp.resume()?;
        Ok(WildMailbox {
            target_seed,
            mode: 1,
            active_hook: 1,
            last_trigger: mail[6],
            hit_count: u32::from_le_bytes(mail[8..12].try_into().map_err(|_| LiveError::Conflict)?),
            last_old_seed: u32::from_le_bytes(mail[12..16].try_into().map_err(|_| LiveError::Conflict)?),
        })
    }
    pub fn wild_mailbox(&mut self) -> Result<WildMailbox, LiveError> {
        if !self.rsp.is_paused() {
            return Err(LiveError::Conflict);
        }
        self.verify_rom()?;
        if self.owner.as_ref().map(|o| o.feature) != Some(Feature::Wild) {
            return Err(LiveError::Feature);
        }
        let b = self.rsp.read(MAIL, 16)?;
        Ok(WildMailbox {
            target_seed: u32::from_le_bytes(b[0..4].try_into().map_err(|_| LiveError::Conflict)?),
            mode: b[4],
            active_hook: b[5],
            last_trigger: b[6],
            hit_count: u32::from_le_bytes(b[8..12].try_into().map_err(|_| LiveError::Conflict)?),
            last_old_seed: u32::from_le_bytes(b[12..16].try_into().map_err(|_| LiveError::Conflict)?),
        })
    }
    pub fn restore(&mut self) -> Result<Receipt, LiveError> {
        if self.owner.is_none() {
            return Err(LiveError::Missing);
        }
        if !self.rsp.is_paused() {
            self.rsp.interrupt()?;
        }
        self.verify_rom()?;
        let feature = self.owner.as_ref().ok_or(LiveError::Missing)?.feature;
        if feature == Feature::Wild {
            self.safe_overworld()?;
            let cave = self.owner.as_ref().ok_or(LiveError::Missing)?.spans[1].address;
            let pc = self.rsp.pc()? & !1;
            if (cave..cave + WILD_PAYLOAD_LEN as u32).contains(&pc) {
                return Err(LiveError::Conflict);
            }
        }
        let owned = self.owner.take().ok_or(LiveError::Missing)?;
        // A live wild one-shot changes mailbox counters and mode as intended.
        // Validate the entire ROM owner before changing the entry pointer.
        for span in &owned.spans {
            let actual = self.rsp.read(span.address, span.after.len())?;
            if span.address == MAIL {
                if feature != Feature::Wild || actual[7] != 0 ||
                    !matches!(actual[4], 0 | 1) || !matches!(actual[5], 0 | 1) ||
                    !matches!(actual[6], 0 | 1) {
                    self.owner = Some(owned);
                    return Err(LiveError::Conflict);
                }
            } else if actual != span.after && actual != span.before {
                self.owner = Some(owned);
                return Err(LiveError::Conflict);
            }
        }
        if let Err(error) = self.event(
            "restore_intent",
            "pending",
            feature,
            Some(&owned.id),
            None,
            "FRLG restore intent committed before changing Guest",
            None,
        ) {
            self.owner = Some(owned);
            return Err(error);
        }
        let mut ok = true;
        for span in owned.spans.iter().rev() {
            if span.address == MAIL {
                // Account for legitimate self-mutating hit count / last Seed.
                if self.rsp.write(MAIL, &span.before).is_err() {
                    ok = false;
                    break;
                }
            } else {
                let current = self.rsp.read(span.address, span.before.len());
                match current {
                    Ok(data) if data == span.before => {}
                    Ok(data) if data == span.after => {
                        if self.rsp.write(span.address, &span.before).is_err() {
                            ok = false;
                            break;
                        }
                    }
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
        }
        if !ok {
            let id = owned.id.clone();
            self.owner = Some(Owner {
                restore_failed: true,
                ..owned
            });
            let _ = self.event(
                "restore_failed",
                "error",
                feature,
                Some(&id),
                None,
                "FRLG restore incomplete; keep Guest paused, no blind writes",
                Some("RESTORE_FAILED"),
            );
            return Err(LiveError::Recovery);
        }
        if let Err(error) = self.event(
            "restored",
            "ok",
            feature,
            Some(&owned.id),
            None,
            "FRLG owned Guest original bytes restored",
            None,
        ) {
            self.owner = Some(Owner {
                restore_failed: true,
                ..owned
            });
            return Err(error);
        }
        self.rsp.resume()?;
        Ok(Receipt {
            owner_id: owned.id,
            feature_id: feature.id().into(),
            status: "restored".into(),
            guest_running: true,
            audit_recorded: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_feature_whitelist() {
        assert_eq!(Feature::parse("capture").unwrap(), Feature::Capture);
        assert_eq!(Feature::parse("wall").unwrap(), Feature::Wall);
        assert_eq!(Feature::parse("wild").unwrap(), Feature::Wild);
        assert!(Feature::parse("teleport").is_err());
        assert!(Feature::parse("raw-write").is_err());
    }
    #[test]
    fn thumb_jump_stub_leaves_odd_mode_pointer() {
        assert_eq!(
            encode_jump(0x08FF0000),
            [0, 0x4B, 0x18, 0x47, 0x01, 0x00, 0xFF, 0x08]
        );
    }
    #[test]
    fn strict_cave_patch_template_and_reject_untrusted_payload() {
        let bad = [0u8; 116];
        assert!(patch_wild_template(&bad, &[0; 8], 0, 0).is_err());
        assert!(patch_wild_template(&bad[..20], &[0; 8], 0, 0).is_err());
    }
    #[test]
    fn source_halfword_context_has_only_correct_offset() {
        let p = crate::profile_for("BPRE", 1).unwrap();
        let cap = unhex(&p.expected_capture_context).unwrap();
        let wall = unhex(&p.expected_wall_context).unwrap();
        assert_eq!(&cap[4..6], &[0x2F, 0xD9]);
        assert_eq!(&wall[4..6], &[0xC0, 0x20]);
        assert_eq!(p.capture_patch, 0x0802D6D2);
        assert_eq!(p.wall_patch, 0x08058E42);
    }
}
