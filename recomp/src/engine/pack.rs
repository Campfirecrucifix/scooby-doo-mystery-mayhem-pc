//! Level packs: DTFatRecord, DTStreamFAT, and EFRessourcesMgr's level loading
//! (Classes/DT/DTStreamFAT.cpp, Classes/EF/EFRessourcesMgr.cpp). Facts in ENGINE_PORT.md "A:".
//!
//! Bevy: `EFRessourcesMgr` is a Resource. `LoadLevel` reads the whole pack from the [`IsoFs`]
//! (I/O at level-load time, never per frame); the C++ ran it on the main thread during the
//! loading screen, with the loading screen updated per entry. Here the per-entry progress is
//! reported through `on_progress`.

#![allow(non_snake_case)]

use super::dt::stream::*;
use super::dt::property::DTObject;
use super::dt::types;
use super::level::EFGameObject;
use super::dt::descriptors::{DTDynamicPropertyContainer, DTWaypointGrp};
use super::error::EngineError;
use super::iso::IsoFs;

/// DTFatRecord {uid, size, offset, classId}; default: all 0xffffffff except classId 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DTFatRecord {
    pub uid: u32,
    pub size: u32,
    pub offset: u32,
    pub class_id: u32,
}

impl Default for DTFatRecord {
    fn default() -> Self {
        Self { uid: 0xffff_ffff, size: 0xffff_ffff, offset: 0xffff_ffff, class_id: 0 }
    }
}

/// DTStreamFAT (size 0x3c): +0xc records, +0x34 count (init -1), +0x38 version (0).
#[derive(Debug, Clone)]
pub struct DTStreamFAT {
    pub records: Vec<DTFatRecord>,
    pub count: i32,
    pub version: u32,
}

impl Default for DTStreamFAT {
    fn default() -> Self {
        Self { records: Vec::new(), count: -1, version: 0 }
    }
}

impl DTStreamFAT {
    pub const CLASS_NAME: &'static str = "STREAMFAT";

    /// LoadPropertiesBinary: LoadData(uint) version, LoadData(uint) count, SetSize(count),
    /// memset, LoadBinaryData(count*16).
    pub fn LoadPropertiesBinary(&mut self, s: &mut dyn DTBasePersistStream) -> i32 {
        let r = s.LoadData_uint(&mut self.version);
        if r != 0 {
            return r;
        }
        let mut c = 0u32;
        let r = s.LoadData_uint(&mut c);
        if r != 0 {
            return r;
        }
        self.count = c as i32;
        let mut raw = vec![0u8; c as usize * 16];
        let r = s.LoadBinaryData(&mut raw);
        self.records = raw
            .chunks_exact(16)
            .map(|q| {
                let w = |i: usize| u32::from_le_bytes(q[i * 4..i * 4 + 4].try_into().unwrap());
                DTFatRecord { uid: w(0), size: w(1), offset: w(2), class_id: w(3) }
            })
            .collect();
        r
    }

    /// Save* return 0 (unimplemented in the original).
    pub fn SavePropertiesBinary(&mut self, _s: &mut dyn DTBasePersistStream) -> i32 {
        0
    }
}

/// DTStreamable vtable subset that LoadLevel calls (GetClassID 0x08, Load 0x14, IsLoader 0x30).
pub trait DTStreamable: Send + Sync {
    fn GetClassID(&self) -> u32;
    /// Load(stream) (vtbl 0x14). `size` is the FAT record size (the C++ objects do not get it;
    /// it is passed so that not-yet-ported classes can be kept as raw bytes, see [`RawStreamable`]).
    fn Load(&mut self, s: &mut dyn DTBasePersistStream, size: u32) -> i32;
    /// IsLoader (vtbl 0x30): true -> LoadLevel deletes the object right after loading it.
    fn IsLoader(&self) -> bool {
        false
    }
    /// The parsed object when the class is ported (see [`DTObj`]).
    fn as_dt(&mut self) -> Option<&mut dyn DTObject> {
        None
    }
    /// The raw bytes when the class is not ported (see [`RawStreamable`]).
    fn as_raw(&self) -> Option<&RawStreamable> {
        None
    }
}

/// A ported class loaded through its DTObject vtable (Load = Pre/LoadFromStream/Post).
pub struct DTObj(pub Box<dyn DTObject>);

impl DTStreamable for DTObj {
    fn GetClassID(&self) -> u32 {
        self.0.GetClassID()
    }
    fn Load(&mut self, s: &mut dyn DTBasePersistStream, _size: u32) -> i32 {
        self.0.Load(s)
    }
    fn IsLoader(&self) -> bool {
        self.0.IsLoader()
    }
    fn as_dt(&mut self) -> Option<&mut dyn DTObject> {
        Some(self.0.as_mut())
    }
}

/// Stand-in for a registered class whose parser is not ported yet: consumes exactly the FAT
/// record's bytes. Not an original class (the C++ would parse the entry); keeps the sequential
/// stream in step so that later entries still load.
#[derive(Debug, Clone)]
pub struct RawStreamable {
    pub class_id: u32,
    pub uid: u32,
    pub data: Vec<u8>,
}

impl DTStreamable for RawStreamable {
    fn GetClassID(&self) -> u32 {
        self.class_id
    }
    fn Load(&mut self, s: &mut dyn DTBasePersistStream, size: u32) -> i32 {
        self.data = vec![0u8; size as usize];
        let r = s.LoadBinaryData(&mut self.data);
        if self.data.len() >= 4 {
            self.uid = u32::from_le_bytes(self.data[..4].try_into().unwrap());
        }
        r
    }
    fn as_raw(&self) -> Option<&RawStreamable> {
        Some(self)
    }
}

/// DTDynamicInstanciator::GeInfoByClsID + info->ctor(): returns None for unregistered or
/// type-2 classes (LoadLevel then skips the entry... see the quirk in LoadLevel).
pub trait DTDynamicInstanciator {
    fn Instanciate(&mut self, class_id: u32) -> Option<Box<dyn DTStreamable>>;
}

/// Default instanciator: ported classes are parsed (level::CreateInstance), every other
/// non-type-2 class becomes a RawStreamable.
#[derive(Default)]
pub struct RawInstanciator;

impl DTDynamicInstanciator for RawInstanciator {
    fn Instanciate(&mut self, class_id: u32) -> Option<Box<dyn DTStreamable>> {
        // Ids missing from CLASS_TABLE (0x1f, 0x30, 0x3f) are registered by their own modules in
        // the C++ (DTDynamicPropertyContainer, DTWaypointGrp, DTHotspotDescriptor); kept raw too.
        if let Some(o) = super::level::CreateInstance(class_id) {
            return Some(Box::new(DTObj(o)));
        }
        let type2 = types::CLASS_TABLE.iter().any(|e| e.1 == class_id && e.2 == 2);
        (!type2).then(|| Box::new(RawStreamable { class_id, uid: 0xffff_ffff, data: Vec::new() }) as Box<dyn DTStreamable>)
    }
}

/// externalfiledatabuffer {char name[200]; void* buf; uint size}
#[derive(Debug, Clone)]
pub struct ExternalFileDataBuffer {
    pub name: String,
    pub buf: Vec<u8>,
}

/// EFRessourcesMgr (Bevy Resource).
#[derive(Default, bevy_ecs::resource::Resource)]
pub struct EFRessourcesMgr {
    /// this+0x54
    pub level_file_name: Option<String>,
    /// this+0x58 = strdup("")
    pub level_ext: String,
    /// this+0x61 loading flag
    pub loading: bool,
    /// this+0x64 requested external files
    pub requested_external: Vec<String>,
    /// this+0x74 loaded external files
    pub external_files: Vec<ExternalFileDataBuffer>,
    /// Objects kept by LoadLevel (the C++ objects register themselves into their managers in
    /// their Load/OnLoadFinish; until those managers exist, they are kept here in pack order).
    pub objects: Vec<Box<dyn DTStreamable>>,
    /// EFWorld::AddGameObject / EFGameLogicMgr latent map registrations made by EFGameObject::
    /// LoadFromStream, in load order (see level::EFGameObject: the world steps are deferred).
    pub game_objects: Vec<EFGameObject>,
    /// DTWpGrpMgr::AddWaypointGrp registrations made by DTWaypointGrp::LoadFromStream, in order.
    pub waypoint_groups: Vec<DTWaypointGrp>,
    /// raw files LoadLevel reads besides the pack (TXD/FT/PAL/TX/LS), keyed by path
    pub side_files: Vec<(String, Vec<u8>)>,
}

impl EFRessourcesMgr {
    /// OpenStream(name, &fd, &isP2R, &err): .p2r/.P2R/.phr/.PHR -> DTBinaryPersistStream,
    /// isP2R = true; anything else -> err = 0xfffffffd and no stream.
    pub fn OpenStream(name: &str, file: Vec<u8>, is_p2r: &mut bool, err: &mut u32) -> Option<DTBinaryPersistStream> {
        if [".p2r", ".P2R", ".phr", ".PHR"].iter().any(|x| name.contains(x)) {
            *is_p2r = true;
            Some(DTBinaryPersistStream::new(file))
        } else {
            *err = 0xffff_fffd;
            None
        }
    }

    /// LoadLevelTextureDictionnary(idx, mode) / LoadFontsAndPalettes / LoadTexts / LoadPalette
    /// file probing: reads every file the C++ would read, in the same order, into `side_files`
    /// (the parsers belong to WP-C/F). Returns the number of files found.
    fn LoadSideFiles(&mut self, fs: &IsoFs, prefix: &str, level_dir: &str, lang: &str) -> usize {
        let mut n = 0usize;
        let mut take = |me: &mut Self, p: String| -> bool {
            if fs.size(&p) > 0 {
                if let Ok(b) = fs.read(&p) {
                    me.side_files.push((p, b));
                    n += 1;
                    return true;
                }
            }
            false
        };
        // LoadLevelTextureDictionnary(0,-1): lang first, GEN fallback (mode 1 sticks), until missing
        let mut mode = -1;
        let mut idx = 0;
        loop {
            let dir = if mode == -1 || mode == 0 { lang } else { "GEN" };
            let p = format!("{prefix}LEVEL\\{level_dir}\\{dir}\\TEXDIC_{idx}.TXD");
            if take(self, p) {
                idx += 1;
                if mode == -1 {
                    mode = 0;
                }
            } else if mode == -1 {
                mode = 1;
            } else {
                break;
            }
        }
        if idx == 0 {
            eprintln!(" ***  Could Not Load any TXD Files!!! ");
        }
        // LoadFontsAndPalettes(0,-1)
        let mut mode = -1;
        let mut idx = 0;
        loop {
            let dir = if mode == -1 || mode == 0 { lang } else { "GEN" };
            let p = format!("{prefix}LEVEL\\{level_dir}\\{dir}\\FONT{idx}.FT");
            if !take(self, p) {
                if mode == -1 {
                    mode = 1;
                    continue;
                }
                break;
            }
            if mode == -1 {
                mode = 0;
            }
            if idx == 0 {
                take(self, format!("{prefix}LEVEL\\{level_dir}\\{lang}\\GLOBAL.PAL"));
                take(self, format!("{prefix}LEVEL\\{level_dir}\\{lang}\\FONT0.PAL"));
            } else {
                let d = if mode == 0 { lang } else { "GEN" };
                take(self, format!("{prefix}LEVEL\\{level_dir}\\{d}\\FONT{idx}.PAL"));
            }
            idx += 1;
        }
        if idx == 0 {
            eprintln!(" ***  Could Not Load any Font Files!!! ");
        }
        // LoadTexts
        if !take(self, format!("{prefix}LEVEL\\{level_dir}\\{lang}\\TEXT.TX")) && !take(self, format!("{prefix}LEVEL\\{level_dir}\\GEN\\TEXT.TX")) {
            eprintln!(" ***  Could Not Load any TEXT Files!!! ");
        }
        n
    }

    /// LoadLevel: see ENGINE_PORT.md "EFRessourcesMgr::LoadLevel order". Returns the C++ error
    /// code (0 ok); Err only for host failures (missing pack file).
    pub fn LoadLevel(
        &mut self,
        fs: &IsoFs,
        prefix: &str,
        level_dir: &str,
        lang: &str,
        inst: &mut dyn DTDynamicInstanciator,
        mut on_progress: impl FnMut(usize, usize),
    ) -> Result<u32, EngineError> {
        self.loading = true;
        let level_dir = level_dir.to_ascii_uppercase();
        let level_file = self.level_file_name.clone().unwrap_or_default().to_ascii_uppercase();
        let path = format!("{prefix}LEVEL\\{level_dir}\\GEN\\{level_file}");
        // loading screen file (LOAD.LS) and the side files
        let ls = format!("{prefix}LEVEL\\{level_dir}\\{lang}\\LOAD.LS");
        if fs.size(&ls) > 0 {
            self.side_files.push((ls.clone(), fs.read(&ls)?));
        }
        self.LoadSideFiles(fs, prefix, &level_dir, lang);

        let file = fs.read(&path)?;
        let mut is_p2r = false;
        let mut err = 0u32;
        let Some(mut stream) = Self::OpenStream(&level_file, file, &mut is_p2r, &mut err) else {
            self.loading = false;
            return Ok(err);
        };
        let mut fat = DTStreamFAT::default();
        // DTStreamable::LoadFromStream(fat, stream): binary stream -> LoadPropertiesBinary
        let r = fat.LoadPropertiesBinary(&mut stream);
        if r != 0 {
            self.loading = false;
            return Ok(r as u32);
        }
        let total = fat.count.max(0) as usize + self.requested_external.len();
        for (i, rec) in fat.records.clone().iter().enumerate() {
            let obj = inst.Instanciate(rec.class_id);
            // quirk: the C++ calls obj->Load with obj == NULL for unregistered ids (crash on PS2);
            // every id in the shipped packs is registered, so None means "skip the bytes" here.
            let Some(mut obj) = obj else {
                stream.ReadRaw(rec.size as usize);
                on_progress(i + 1, total);
                continue;
            };
            let r = if is_p2r && matches!(rec.class_id, 0x27 | 0x45 | 0x49 | 0x22) {
                let buf = stream.ReadRaw(rec.size as usize);
                let mut ms = DTMemoryPersistStream::new(buf);
                obj.Load(&mut ms, rec.size)
            } else {
                obj.Load(&mut stream, rec.size)
            };
            if r != 0 {
                err = r as u32;
            }
            self.Register(obj.as_mut());
            if !obj.IsLoader() {
                self.objects.push(obj);
            }
            if err != 0 {
                break;
            }
            on_progress(i + 1, total);
        }
        // quirk: the trailing DTDynamicPropertyContainer (0x1f) is freed after the loop
        if self.objects.last().is_some_and(|o| o.GetClassID() == types::DT_DYNAMIC_PROPERTY_CONTAINER) {
            self.objects.pop();
        }
        for name in std::mem::take(&mut self.requested_external) {
            self.LoadExternalFile(fs, prefix, &name);
        }
        self.loading = false;
        Ok(err)
    }

    /// The registrations the loaded object made into the engine managers during its Load:
    /// EF_GLO containers hand their game objects to the world, waypoint groups to DTWpGrpMgr.
    fn Register(&mut self, obj: &mut dyn DTStreamable) {
        let Some(o) = obj.as_dt() else { return };
        let any: &mut dyn std::any::Any = o.as_any_mut();
        if let Some(c) = any.downcast_mut::<DTDynamicPropertyContainer>() {
            for mut g in std::mem::take(&mut c.objects) {
                if let Some(go) = g.as_any_mut().downcast_mut::<EFGameObject>() {
                    self.game_objects.push(std::mem::take(go));
                } else {
                    c.objects.push(g);
                }
            }
        } else if let Some(w) = any.downcast_mut::<DTWaypointGrp>() {
            self.waypoint_groups.push(w.clone());
        }
    }

    /// RequestExternalFileLoad(name)
    pub fn RequestExternalFileLoad(&mut self, name: &str) {
        self.requested_external.push(name.to_string());
    }

    /// LoadExternalFile(prefix, name): quirk: prefix unused (DTFopen(name)).
    pub fn LoadExternalFile(&mut self, fs: &IsoFs, _prefix: &str, name: &str) {
        if let Ok(buf) = fs.read(name) {
            self.external_files.push(ExternalFileDataBuffer { name: name.chars().take(199).collect(), buf });
        }
    }

    pub fn GetExternalFileBuffer(&self, name: &str) -> Option<&[u8]> {
        self.external_files.iter().find(|f| f.name == name).map(|f| f.buf.as_slice())
    }

    pub fn GetExternalFileSize(&self, name: &str) -> u32 {
        self.external_files.iter().find(|f| f.name == name).map_or(0, |f| f.buf.len() as u32)
    }

    /// DoesLevelFileExist(levelDir, name): strtok(name, ".") then "LEVEL\%s\GEN\%s%s" with ext.
    pub fn DoesLevelFileExist(&self, fs: &IsoFs, level_dir: &str, name: &str) -> bool {
        let base = name.split('.').next().unwrap_or("");
        fs.size(&format!("LEVEL\\{level_dir}\\GEN\\{base}{}", self.level_ext).to_ascii_uppercase()) > 0
    }

    /// Terminate: frees external buffers and every loaded resource.
    pub fn Terminate(&mut self) {
        self.external_files.clear();
        self.objects.clear();
        self.side_files.clear();
        self.game_objects.clear();
        self.waypoint_groups.clear();
    }
}
