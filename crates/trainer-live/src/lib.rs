//! Live GBA capture and opt-in BattleCrit patch transactions.
//! Transport is mGBA paused guest CPU via localhost GDB, not OS memory mutation.
//! Builds must have *complete* source-listed ROM SHA1, not just header/anchor.
//! Explicit local GPL-3.0 user-supplied payload is required for experimental Crit.
//! No online or arbitrary-address patch interface is exported to Tauri/JS.
use serde::{Serialize,Deserialize};
use sha1::{Digest as _,Sha1};
use sha2::Sha256;
use std::time::{Duration,Instant};
use thiserror::Error;
use trainer_rsp::{RspClient,RspError};
use trainer_rs::{parse_header,matches_anchor,RsProfile};
use trainer_storage::{ActionEvent,AuditStore,AuditError,EventContext};
use uuid::Uuid;

const ROM_BASE:u32=0x0800_0000;
const PREVIEW_TTL:Duration=Duration::from_secs(30);
const CAPTURE_CONTEXT:&[u8]=&[0xF0,0xFF,0x0F,0x00,0x60,0x1C,0x00,0x06,0x04,0x0E,0x03,0x2C,0x05,0xD8];
const CRIT_PREFIX:&[u8]=&[0xF0,0xB5,0x08,0x49,0x08,0x48,0x02,0x78,0x58,0x20,0x50,0x43,0x40,0x18,0xC0,0x8D];
const CRIT_TEMPLATE_SHA256:&str="c0b3b51b0bea60c6e14107c1060891b88688993e5ca67ebb32486b139551a7db";
const ROM_KNOWN:&str=include_str!("../../../fixtures/rs-known-rom-sha1.json");

#[derive(Debug,Error)]
pub enum LiveError {
    #[error("mGBA GDB: {0}")]
    Rsp(#[from] RspError),
    #[error("SQLite: {0}")]
    Audit(#[from] AuditError),
    #[error("ROM profile: {0}")]
    Rom(String),
    #[error("ROM is not in the 28-image exact SHA1 allowlist")]
    UnsupportedRom,
    #[error("target game is running; pause mGBA GDB first")]
    NotPaused,
    #[error("a patch is already active or in an unresolved state")]
    Busy,
    #[error("no preview or active patch for this session")]
    Missing,
    #[error("preview was used, expired, or belongs to another plan")]
    Token,
    #[error("guest bytes, battle status, or ownership changed; refusing overwrite")]
    Conflict,
    #[error("patch recovery incomplete: keep emulator paused for diagnosis")]
    RecoveryFailed,
    #[error("experimental Hook template is missing or differs from the v4.6 user source")]
    InvalidPayload,
    #[error("this ROM is not in the 4-build BattleCrit evidence whitelist")]
    UnverifiedCrit,
}

#[derive(Deserialize)]
struct RomList { roms:Vec<RomEntry> }
#[derive(Deserialize)]
struct RomEntry {game_code:String,revision:u8,sha1:String,rom_bytes:usize}
#[derive(Clone,Debug,Serialize)]
pub struct LiveIdentity {
    pub session_id:String,
    pub game_id:String,
    pub build_id:String,
    pub game_code:String,
    pub revision:u8,
    pub sha1:String,
    pub rom_bytes:usize,
    pub gdb_paused:bool,
    pub transport:String,
    pub experimental_hook_supported:bool,
    pub active_feature:Option<String>,
    pub active_status:Option<String>,
}
#[derive(Clone,Debug,Serialize)]
pub struct LivePreview {
    pub token:String,
    pub game_id:String,
    pub feature_id:String,
    pub rom_sha1:String,
    pub spans:Vec<PreviewSpan>,
    pub expires_in_seconds:u64,
    pub risk:String,
    pub requires_confirmation:bool,
}
#[derive(Clone,Debug,Serialize)]
pub struct PreviewSpan{
    pub label:String,
    pub guest_address:String,
    pub before:String,
    pub after:String,
}
#[derive(Clone,Debug,Serialize)]
pub struct LiveReceipt {
    pub owner_id:String,
    pub feature_id:String,
    pub state:String,
    pub audit_committed:bool,
    pub emulator_running:bool,
}
#[derive(Clone)]
struct Span {
    label:&'static str,
    address:u32,
    before:Vec<u8>,
    after:Vec<u8>,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Feature {Capture, BattleCrit}
impl Feature {
    fn id(self)->&'static str{
        match self {Self::Capture=>"features.rs.capture",Self::BattleCrit=>"features.rs.battlecrit"}
    }
}
struct Plan {
    token:String,
    expires:Instant,
    feature:Feature,
    spans:Vec<Span>,
}
struct Owned {
    owner:String,
    feature:Feature,
    spans:Vec<Span>,
    failed:bool,
}
pub struct LiveEditor {
    rsp:RspClient,
    identity:LiveIdentity,
    audit:AuditStore,
    preview:Option<Plan>,
    active:Option<Owned>,
}
fn hex(bytes:&[u8])->String {
    bytes.iter().map(|b|format!("{b:02X}")).collect()
}
fn capture_addresses(code:&str)->Option<[u32;2]>{
    Some(match code.as_bytes().get(3)? {
        b'E'=>[0x0802B8A6,0x0802B8C8],
        b'J'=>[0x08028AD2,0x08028AF4],
        b'D'|b'F'=>[0x0802BA7A,0x0802BA9C],
        b'I'|b'S'=>[0x0802BA72,0x0802BA94],
        _=>return None,
    })
}
struct CritProof{
    table:u32, handler:u32, flags:u32, attacker:u32, target:u32, mons:u32,
    statuses:u32, crit:u32, main_state:u32,
}
fn crit_proof(code:&str,rev:u8)->Option<CritProof>{
    match (code,rev){
        ("AXPE",1|2)=>Some(CritProof{
            table:0x081FA6E4,handler:0x0801C879,flags:0x020239F8,
            attacker:0x02024C07,target:0x02024C08,mons:0x02024A80,
            statuses:0x02024C98,crit:0x02024C0D,main_state:0x03001BAC,
        }),
        ("AXVJ",0)=>Some(CritProof{
            table:0x081CC50C,handler:0x08019B59,flags:0x02023758,
            attacker:0x02024967,target:0x02024968,mons:0x020247E0,
            statuses:0x020249F8,crit:0x0202496D,main_state:0x03001B1C,
        }),
        ("AXPJ",0)=>Some(CritProof{
            table:0x081CC49C,handler:0x08019B59,flags:0x02023758,
            attacker:0x02024967,target:0x02024968,mons:0x020247E0,
            statuses:0x020249F8,crit:0x0202496D,main_state:0x03001B1C,
        }),
        _=>None,
    }
}
impl LiveEditor {
    /// Expensive by design: authenticate the *entire* GDB-visible ROM before
    /// accepting any live mutation, including Pokémon Box JP variants.
    pub fn connect(port:u16,audit:AuditStore)->Result<Self,LiveError>{
        let mut rsp=RspClient::connect_local(port)?;
        let header=rsp.read(ROM_BASE,0xC0)?;
        let profile=parse_header(&header).map_err(|e|LiveError::Rom(e.to_string()))?;
        let anchor=rsp.read(0x08000128,16)?;
        let actual:[u8;16]=anchor.try_into().map_err(|_|LiveError::UnsupportedRom)?;
        if !matches_anchor(&profile.game_code,&actual){return Err(LiveError::UnsupportedRom);}
        let list:RomList=serde_json::from_str(ROM_KNOWN)
            .map_err(|e|LiveError::Rom(format!("invalid evidence manifest: {e}")))?;
        let mut matching=list.roms.iter()
            .filter(|entry|entry.game_code==profile.game_code&&entry.revision==profile.revision)
            .collect::<Vec<_>>();
        if matching.is_empty(){return Err(LiveError::UnsupportedRom);}
        matching.sort_by_key(|entry|entry.rom_bytes);
        // mGBA limits a GDB memory packet to 512 bytes. Hashing is streamed with
        // bounded reads; no ROM bytes or game save are persisted to SQLite.
        let mut sha=Sha1::new();
        let mut at=0usize;
        for limit in [8*1024*1024,16*1024*1024] {
            if !matching.iter().any(|entry|entry.rom_bytes==limit){continue;}
            while at<limit {
                let amount=(limit-at).min(256);
                sha.update(rsp.read(ROM_BASE+at as u32,amount)?);
                at+=amount;
            }
            let hash=hex(&sha.clone().finalize());
            if matching.iter().any(|entry|entry.rom_bytes==limit&&entry.sha1.eq_ignore_ascii_case(&hash)){
                let session_id=Uuid::new_v4().to_string();
                let is_crit=crit_proof(&profile.game_code,profile.revision).is_some();
                return Ok(Self{
                    rsp,
                    identity:LiveIdentity{
                        session_id,game_id:profile.game.id().to_owned(),build_id:profile.build_id,
                        game_code:profile.game_code,revision:profile.revision,
                        sha1:hash,rom_bytes:limit,gdb_paused:true,transport:"mGBA localhost GDB RSP".into(),
                        experimental_hook_supported:is_crit,active_feature:None,active_status:None,
                    },
                    audit,preview:None,active:None,
                });
            }
        }
        Err(LiveError::UnsupportedRom)
    }
    pub fn status(&self)->LiveIdentity{
        let mut identity=self.identity.clone();
        identity.gdb_paused=self.rsp.is_paused();
        identity.active_feature=self.active.as_ref().map(|o|o.feature.id().into());
        identity.active_status=self.active.as_ref().map(|o|if o.failed {"restore_failed"}else{"active"}.into());
        identity
    }
    pub fn has_active(&self)->bool{self.active.is_some()}
    pub fn disconnect(&mut self)->Result<(),LiveError>{
        if self.active.is_some(){return Err(LiveError::Busy);}
        self.preview=None;
        if self.rsp.is_paused(){self.rsp.resume()?;}
        Ok(())
    }
    fn verify_identity(&mut self)->Result<(),LiveError>{
        if !self.rsp.is_paused(){return Err(LiveError::NotPaused);}
        let data=self.rsp.read(ROM_BASE,0xC0)?;
        let p=parse_header(&data).map_err(|_|LiveError::Conflict)?;
        if p.game_code!=self.identity.game_code || p.revision!=self.identity.revision {
            return Err(LiveError::Conflict);
        }
        if !matches_anchor(&p.game_code,&self.rsp.read(0x08000128,16)?
             .try_into().map_err(|_|LiveError::Conflict)?) {
            return Err(LiveError::Conflict);
        }
        Ok(())
    }
    fn event(&mut self,kind:&str,status:&str,feature:Feature,
        owner:Option<&str>,token:Option<&str>,summary:&str,error:Option<&str>)->Result<(),LiveError>{
        let row=ActionEvent::new(Uuid::new_v4().to_string(),EventContext{
            owner_id:owner.map(str::to_owned),preview_id:token.map(str::to_owned),
            session_id:self.identity.session_id.clone(),game_id:self.identity.game_id.clone(),
            feature_id:feature.id().into(),environment:"live".into(),
        },kind,status,summary,error);
        self.audit.append(&row)?;
        Ok(())
    }
    fn plan(&mut self,feature:Feature,spans:Vec<Span>,risk:&str)->Result<LivePreview,LiveError>{
        if self.active.is_some()||self.preview.is_some(){return Err(LiveError::Busy);}
        self.verify_identity()?;
        for (i,s) in spans.iter().enumerate(){
            if s.before.is_empty()||s.before.len()!=s.after.len()||s.before.len()>512{return Err(LiveError::Conflict);}
            for other in spans.iter().take(i){
                let end=s.address as u64+s.before.len() as u64;
                let other_end=other.address as u64+other.before.len() as u64;
                if (s.address as u64)<other_end&&(other.address as u64)<end{return Err(LiveError::Conflict);}
            }
            if self.rsp.read(s.address,s.before.len())?!=s.before{return Err(LiveError::Conflict);}
        }
        let token=Uuid::new_v4().to_string();
        let public=LivePreview{
            token:token.clone(),game_id:self.identity.game_id.clone(),feature_id:feature.id().into(),
            rom_sha1:self.identity.sha1.clone(),
            spans:spans.iter().map(|s|PreviewSpan{
                label:s.label.into(),guest_address:format!("0x{:08X}",s.address),
                before:hex(&s.before),after:hex(&s.after)
            }).collect(),
            expires_in_seconds:PREVIEW_TTL.as_secs(),risk:risk.into(),requires_confirmation:true,
        };
        self.event("preview","ok",feature,None,Some(&token),"Paused GBA patch preview; no bytes changed",None)?;
        self.preview=Some(Plan{token,expires:Instant::now()+PREVIEW_TTL,feature,spans});
        Ok(public)
    }
    pub fn preview_capture(&mut self)->Result<LivePreview,LiveError>{
        let [a,b]=capture_addresses(&self.identity.game_code).ok_or(LiveError::UnsupportedRom)?;
        if self.rsp.read(a+2,CAPTURE_CONTEXT.len())?!=CAPTURE_CONTEXT{return Err(LiveError::Conflict);}
        self.plan(Feature::Capture,vec![
            Span{label:"Capture branch A",address:a,before:vec![0x04,0xE0],after:vec![0x0C,0xE0]},
            Span{label:"Capture branch B",address:b,before:vec![0x00,0xD1],after:vec![0xC0,0x46]},
        ],"Two ROM halfwords, actual live emulator patch; apply only while guest CPU stopped and save backed up")
    }
    fn ensure_not_battling(&mut self,proof:&CritProof)->Result<(),LiveError>{
        let state=self.rsp.read(proof.main_state+1,1)?[0];
        if state&0x02!=0 {return Err(LiveError::Conflict);}
        Ok(())
    }
    fn verify_crit_anchors(&mut self,p:&CritProof)->Result<(),LiveError>{
        let handler=p.handler & !1;
        if self.rsp.read(handler,CRIT_PREFIX.len())?!=CRIT_PREFIX{return Err(LiveError::Conflict);}
        for (off,val) in [(0xC8,p.attacker),(0xCC,p.mons),(0x144,p.target),
            (0x148,p.statuses),(0x14C,p.flags),(0x154,p.crit),(0x16C,p.crit)]{
            if self.rsp.read(handler+off,4)?!=val.to_le_bytes(){return Err(LiveError::Conflict);}
        }
        let tail=self.rsp.read(handler+0x158,0x14)?;
        if tail[0..8]!=[0x04,0x49,0x01,0x20,0x08,0x70,0x04,0x49]
          ||tail[14..20]!=[0xF0,0xBC,0x01,0xBC,0x00,0x47]{return Err(LiveError::Conflict);}
        if self.rsp.read(p.table+16,4)?!=p.handler.to_le_bytes(){return Err(LiveError::Conflict);}
        Ok(())
    }
    fn find_cave(&mut self)->Result<u32,LiveError>{
        let low=ROM_BASE+self.identity.rom_bytes as u32-0x10000;
        let mut addr=low;
        while addr<ROM_BASE+self.identity.rom_bytes as u32-124{
            let bytes=self.rsp.read(addr,124)?;
            if bytes.iter().all(|b|*b==0xFF){return Ok(addr);}
            addr+=4;
        }
        Err(LiveError::Conflict)
    }
    /// The GPL-3.0 Thumb template must be supplied by the user locally.
    /// We accept exactly the 124-byte source binary hash; JS cannot supply
    /// arbitrary executable payloads or guest addresses.
    pub fn preview_crit(&mut self,template:&[u8])->Result<LivePreview,LiveError>{
        let proof=crit_proof(&self.identity.game_code,self.identity.revision)
            .ok_or(LiveError::UnverifiedCrit)?;
        if template.len()!=124 || hex(&Sha256::digest(template)).to_ascii_lowercase()!=CRIT_TEMPLATE_SHA256{
            return Err(LiveError::InvalidPayload);
        }
        self.verify_identity()?;
        self.ensure_not_battling(&proof)?;
        self.verify_crit_anchors(&proof)?;
        let cave=self.find_cave()?;
        let mut payload=template.to_vec();
        let values=[proof.handler,proof.flags,proof.attacker,proof.statuses,
            proof.target,proof.mons,proof.crit];
        let marks=[0x11u8,0x22,0x33,0x66,0x44,0x55,0x77];
        for (i,(&v,&marker)) in values.iter().zip(marks.iter()).enumerate(){
            let off=0x60+i*4;
            if payload[off..off+4]!=[marker;4]{return Err(LiveError::InvalidPayload);}
            payload[off..off+4].copy_from_slice(&v.to_le_bytes());
        }
        let before=self.rsp.read(cave,payload.len())?;
        if !before.iter().all(|b|*b==0xFF){return Err(LiveError::Conflict);}
        self.plan(Feature::BattleCrit,vec![
            Span{label:"staged Thumb code cave",address:cave,before,after:payload},
            Span{label:"battle command dispatch entry (activate LAST)",address:proof.table+16,
                 before:proof.handler.to_le_bytes().to_vec(),after:(cave|1).to_le_bytes().to_vec()},
        ],"EXPERIMENTAL: 4 exact ROMs only; may crash or damage gameplay. User GPL payload; never install during battle.")
    }
    pub fn cancel_preview(&mut self)->Result<(),LiveError>{
        let plan=self.preview.take().ok_or(LiveError::Missing)?;
        self.event("preview_cancelled","ok",plan.feature,None,Some(&plan.token),"Preview discarded",None)
    }
    /// Applies with GDB CPU stopped. Will not resume if rollback is uncertain.
    pub fn apply(&mut self,token:&str)->Result<LiveReceipt,LiveError>{
        let plan=self.preview.take().ok_or(LiveError::Missing)?;
        if token!=plan.token||Instant::now()>=plan.expires{return Err(LiveError::Token);}
        self.verify_identity()?;
        if plan.feature==Feature::BattleCrit {
            let p=crit_proof(&self.identity.game_code,self.identity.revision).ok_or(LiveError::Conflict)?;
            self.ensure_not_battling(&p)?;self.verify_crit_anchors(&p)?;
        }
        for s in &plan.spans{if self.rsp.read(s.address,s.before.len())?!=s.before{return Err(LiveError::Conflict);}}
        let owner=Uuid::new_v4().to_string();
        self.event("apply_intent","pending",plan.feature,Some(&owner),Some(&plan.token),
                   "Multispan live transaction intent committed before write",None)?;
        let mut written=0usize;
        for span in &plan.spans {
            if self.rsp.write(span.address,&span.after).is_err(){break;}
            written+=1;
        }
        if written!=plan.spans.len(){
            // RSP write may partially succeed even when it returned Err.
            // Only undo spans still holding our desired bytes.
            let mut safe=true;
            for span in plan.spans.iter().rev(){
                match self.rsp.read(span.address,span.before.len()){
                    Ok(b) if b==span.before=>{},
                    Ok(b) if b==span.after=>{
                        if self.rsp.write(span.address,&span.before).is_err(){safe=false;break;}
                    },
                    _=>{safe=false;break;}
                }
            }
            if safe {
                self.event("rollback_confirmed","ok",plan.feature,Some(&owner),Some(&plan.token),
                           "Original bytes restored after failed live write",None)?;
                return Err(LiveError::Conflict);
            }
            self.active=Some(Owned{owner:owner.clone(),feature:plan.feature,
                                   spans:plan.spans,failed:true});
            let _=self.event("restore_failed","error",plan.feature,Some(&owner),None,
                             "Guest bytes uncertain; GDB CPU remains stopped",Some("PARTIAL_WRITE"));
            return Err(LiveError::RecoveryFailed);
        }
        self.active=Some(Owned{owner:owner.clone(),feature:plan.feature,
                               spans:plan.spans,failed:false});
        self.event("applied","ok",plan.feature,Some(&owner),Some(&plan.token),
                   "All guest spans written and verified while stopped",None)?;
        // The live patch is actually effective once guest CPU resumes.
        self.rsp.resume()?;
        Ok(LiveReceipt{owner_id:owner,feature_id:plan.feature.id().into(),state:"active".into(),
                       audit_committed:true,emulator_running:!self.rsp.is_paused()})
    }
    pub fn restore(&mut self)->Result<LiveReceipt,LiveError>{
        let owned=self.active.as_ref().ok_or(LiveError::Missing)?;
        if !self.rsp.is_paused(){self.rsp.interrupt()?;}
        self.verify_identity()?;
        if owned.feature==Feature::BattleCrit {
            let p=crit_proof(&self.identity.game_code,self.identity.revision).ok_or(LiveError::Conflict)?;
            self.ensure_not_battling(&p)?;
            let pc=self.rsp.pc()? & !1;
            let cave=&owned.spans[0];
            if pc>=cave.address && (pc as u64)<cave.address as u64+cave.after.len() as u64{
                return Err(LiveError::Conflict);
            }
        }
        let owned=self.active.take().ok_or(LiveError::Missing)?;
        let validation=owned.spans.iter().try_for_each(|span| -> Result<(),LiveError> {
            let current=self.rsp.read(span.address,span.after.len())?;
            if current==span.after || current==span.before {Ok(())}
            else {Err(LiveError::Conflict)}
        });
        if let Err(error)=validation {
            self.active=Some(owned);
            return Err(error);
        }
        if let Err(error)=self.event("restore_intent","pending",owned.feature,Some(&owned.owner),None,
            "Live guest restore intent persisted",None){
            self.active=Some(owned);
            return Err(error);
        }
        // Reverse order is essential: unhook dispatch pointer before cave cleanup.
        // A prior failed attempt may already have restored some spans. Only
        // exact original/owned bytes are acceptable during an idempotent retry.
        let mut safe=true;
        for span in owned.spans.iter().rev(){
            match self.rsp.read(span.address,span.after.len()){
                Ok(current) if current==span.before=>{},
                Ok(current) if current==span.after=>{
                    if self.rsp.write(span.address,&span.before).is_err(){safe=false;break;}
                },
                _=>{safe=false;break;}
            }
        }
        if !safe {
            let mut unresolved=owned;
            unresolved.failed=true;
            let owner=unresolved.owner.clone();
            let feature=unresolved.feature;
            self.active=Some(unresolved);
            let _=self.event("restore_failed","error",feature,Some(&owner),None,
                             "Multispan restore not verified; keep mGBA CPU paused",Some("RESTORE_FAILED"));
            return Err(LiveError::RecoveryFailed);
        }
        if let Err(error)=self.event("restored","ok",owned.feature,Some(&owned.owner),None,
                   "Original guest bytes restored and confirmed",None){
            self.active=Some(Owned{failed:true,..owned});
            return Err(error);
        }
        self.rsp.resume()?;
        Ok(LiveReceipt{owner_id:owned.owner,feature_id:owned.feature.id().into(),
            state:"restored".into(),audit_committed:true,emulator_running:!self.rsp.is_paused()})
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn capture_26_profile_address_families(){
        assert_eq!(capture_addresses("AXVE"),Some([0x0802B8A6,0x0802B8C8]));
        assert_eq!(capture_addresses("AXPJ"),Some([0x08028AD2,0x08028AF4]));
        assert_eq!(capture_addresses("AXPF"),Some([0x0802BA7A,0x0802BA9C]));
        assert_eq!(capture_addresses("AXVS"),Some([0x0802BA72,0x0802BA94]));
        assert_eq!(capture_addresses("BPEE"),None);
    }
    #[test]
    fn four_critical_profiles_only(){
        for (code,rev) in [("AXPE",1),("AXPE",2),("AXVJ",0),("AXPJ",0)] {
            assert!(crit_proof(code,rev).is_some());
        }
        for (code,rev) in [("AXPE",0),("AXVE",2),("AXVJ",1),("AXPJ",1)] {
            assert!(crit_proof(code,rev).is_none());
        }
    }
    #[test]
    fn all_28_rom_sha1_entries_are_source_bounded(){
        let entries:RomList=serde_json::from_str(ROM_KNOWN).unwrap();
        assert_eq!(entries.roms.len(),28);
        assert!(!entries.roms.iter().any(|r|r.sha1=="CA5E3D415C4B47353A73A616878BA833F3648B7A"));
        for row in &entries.roms{
            assert_eq!(row.sha1.len(),40);
            assert!(matches!(row.rom_bytes,8388608|16777216));
            assert!(trainer_rs::profile_for(&row.game_code,row.revision).is_ok());
        }
    }
}
