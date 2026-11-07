//! EFWorld's object map and object retrievers (EXE.1 Classes/EF/EFWorld.cpp, EF*Retriever.cpp,
//! DT/DTMap.cpp).
//!
//! The map is ported as the original hash table, not a Rust HashMap, because modules can see its
//! iteration order: retriever ids (0xffxxxxxx) walk it, and `ObjectRetrieverParams[0]` holds a cursor
//! that the module passes back on the next call (TomeDoomFUNC_ScanForMonster loops that way).
//! Ties such as "nearest monster" therefore resolve the same way they did on the PS2.
use super::ObjH;
use crate::engine::{Engine, Ptr, NULL};

/// DTMap<unsigned int, EFGameObject*>(0x25) at EFWorld+0x4154
pub const BUCKETS: u32 = 0x25;
/// cursor value meaning "start from the first object" (ObjectRetrieverParams[0] = 1)
pub const CURSOR_START: u32 = 1;
/// what a filtering retriever writes to the cursor when it runs out of objects
pub const CURSOR_DONE: u32 = 0x7fff_fffe;

struct Node<V> {
    key: u32,
    val: V,
    /// next node in the bucket chain (0 = none)
    next: u32,
}

/// DTMap<unsigned int, V>: the engine's chained hash table (DT/DTMap.cpp). New keys are prepended to
/// their bucket; iteration goes bucket by bucket, chain order. Node references are plain u32s
/// (index + 2) because modules keep them in memory (retriever cursors); they are never 0 (end) or
/// 1 (start).
pub struct DTMap<V> {
    buckets: Vec<u32>,
    nodes: Vec<Option<Node<V>>>,
    free: Vec<u32>,
    pub count: u32,
}

/// EFWorld+0x4154: id -> object
pub type ObjMap = DTMap<ObjH>;

impl<V> Default for DTMap<V> {
    fn default() -> Self {
        DTMap::new(BUCKETS)
    }
}

/// KeyToHashIndex (0x10bd20 / 0x10bd40): `divu`, i.e. unsigned `key % n` (Ghidra prints a signed %,
/// the instruction is unsigned; timer ids 0x9xxxxxxx depend on it).
pub fn KeyToHashIndex(key: u32, n: u32) -> usize {
    (key % n) as usize
}

impl<V> DTMap<V> {
    pub fn new(buckets: u32) -> Self {
        DTMap { buckets: vec![0; buckets as usize], nodes: Vec::new(), free: Vec::new(), count: 0 }
    }
    fn nb(&self) -> u32 {
        self.buckets.len() as u32
    }
    #[track_caller]
    fn node(&self, r: u32) -> &Node<V> {
        self.nodes
            .get(r.wrapping_sub(2) as usize)
            .and_then(|n| n.as_ref())
            .unwrap_or_else(|| panic!("bad DTMap node reference {r:#x}"))
    }
    /// whether a node reference still points at a live node
    pub fn alive(&self, r: u32) -> bool {
        r >= 2 && matches!(self.nodes.get((r - 2) as usize), Some(Some(_)))
    }
    /// key of a node
    pub fn key(&self, r: u32) -> u32 {
        self.node(r).key
    }
    /// value of a node
    pub fn val_ref(&self, r: u32) -> &V {
        &self.node(r).val
    }
    pub fn val_mut(&mut self, r: u32) -> &mut V {
        let i = r.wrapping_sub(2) as usize;
        &mut self.nodes[i].as_mut().expect("bad DTMap node reference").val
    }
    /// DTMap::Find: node reference of `key`, 0 if absent
    pub fn find_node(&self, key: u32) -> u32 {
        let mut r = self.buckets[KeyToHashIndex(key, self.nb())];
        while r != 0 {
            let n = self.node(r);
            if n.key == key {
                return r;
            }
            r = n.next;
        }
        0
    }
    pub fn get(&self, key: u32) -> Option<&V> {
        match self.find_node(key) {
            0 => None,
            r => Some(&self.node(r).val),
        }
    }
    pub fn get_mut(&mut self, key: u32) -> Option<&mut V> {
        match self.find_node(key) {
            0 => None,
            r => Some(self.val_mut(r)),
        }
    }
    /// DTMap::Insert: replace the value of an existing key, else prepend a new node to its bucket.
    pub fn insert(&mut self, key: u32, val: V) {
        let r = self.find_node(key);
        if r != 0 {
            *self.val_mut(r) = val;
            return;
        }
        let b = KeyToHashIndex(key, self.nb());
        let node = Node { key, val, next: self.buckets[b] };
        let r = match self.free.pop() {
            Some(r) => {
                self.nodes[(r - 2) as usize] = Some(node);
                r
            }
            None => {
                self.nodes.push(Some(node));
                self.nodes.len() as u32 + 1
            }
        };
        self.buckets[b] = r;
        self.count += 1;
    }
    /// DTMap::Remove; returns the removed value
    pub fn remove(&mut self, key: u32) -> Option<V> {
        let b = KeyToHashIndex(key, self.nb());
        let mut prev = 0u32;
        let mut r = self.buckets[b];
        while r != 0 {
            let (k, next) = {
                let n = self.node(r);
                (n.key, n.next)
            };
            if k == key {
                if prev == 0 {
                    self.buckets[b] = next;
                } else {
                    self.nodes[(prev - 2) as usize].as_mut().unwrap().next = next;
                }
                let n = self.nodes[(r - 2) as usize].take().unwrap();
                self.free.push(r);
                self.count -= 1;
                return Some(n.val);
            }
            prev = r;
            r = next;
        }
        None
    }
    /// first node of the first non-empty bucket
    pub fn first(&self) -> u32 {
        self.buckets.iter().copied().find(|&r| r != 0).unwrap_or(0)
    }
    /// first node of the first non-empty bucket after the bucket of `key`
    pub fn first_after_bucket_of(&self, key: u32) -> u32 {
        let start = KeyToHashIndex(key, self.nb()) + 1;
        self.buckets[start.min(self.buckets.len())..].iter().copied().find(|&r| r != 0).unwrap_or(0)
    }
    /// DTMap::GetNext: next node in the chain, else the head of the next non-empty bucket, else 0
    pub fn next(&self, r: u32) -> u32 {
        if r == 0 {
            return 0;
        }
        let n = self.node(r);
        if n.next != 0 {
            return n.next;
        }
        self.first_after_bucket_of(n.key)
    }
    /// the chain successor only (map node +8)
    pub fn chain_next(&self, r: u32) -> u32 {
        self.node(r).next
    }
    /// node references in iteration order (snapshot)
    pub fn nodes_in_order(&self) -> Vec<u32> {
        let mut v = Vec::with_capacity(self.count as usize);
        let mut r = self.first();
        while r != 0 {
            v.push(r);
            r = self.next(r);
        }
        v
    }
}

impl<V: Copy> DTMap<V> {
    /// value of a node
    pub fn val(&self, r: u32) -> V {
        self.node(r).val
    }
    pub fn find(&self, key: u32) -> Option<V> {
        self.get(key).copied()
    }
}

impl Engine {
    /// One step of a map-walking retriever: read the cursor from `params[0]` (1 = start), take that
    /// node's object and store the following node back. Returns None at the end.
    fn retriever_step(&mut self, params: Ptr) -> Option<ObjH> {
        let mut node = if params == NULL || self.mem.r32(params) == CURSOR_START {
            self.rt.map.first()
        } else {
            self.mem.r32(params)
        };
        if node == 0 {
            return None;
        }
        let v = self.rt.map.val(node);
        node = self.rt.map.next(node);
        if params != NULL {
            self.mem.w32(params, node);
        }
        Some(v)
    }

    /// world position of object `id` (its frame LTM translation), None = no frame
    fn obj_world_pos(&mut self, id: u32) -> Option<[f32; 3]> {
        let out = self.mem.alloca(0x10);
        let _b = super::LogicBind::new(&mut self.rt);
        if self.host.APIgameobj_GetPosition(&mut self.mem, id, 0x7fff_ffff, out) == 0 {
            return None;
        }
        Some([self.mem.rf(out), self.mem.rf(out + 4), self.mem.rf(out + 8)])
    }
    /// retriever centre (EFInRange/EFNearestRetriever +0x14): params[3..6] offset, transformed by the
    /// LTM of the centre object params[2] (RwV3dTransformPoints: rotated + translated) when it exists
    fn retriever_centre(&mut self, params: Ptr) -> [f32; 3] {
        let off = [self.mem.rf(params + 0xc), self.mem.rf(params + 0x10), self.mem.rf(params + 0x14)];
        let id = self.mem.r32(params + 8);
        if id == 0 {
            return off;
        }
        let Some(p) = self.obj_world_pos(id) else { return off };
        if off == [0.0; 3] {
            return p;
        }
        let out = self.mem.alloca(0x10);
        let mut axis = |w: &mut Self, f: fn(&mut dyn crate::engine::Host, &mut crate::engine::Mem, u32, u32, Ptr) -> Ptr| {
            f(w.host.as_mut(), &mut w.mem, id, 0x7fff_ffff, out);
            [w.mem.rf(out), w.mem.rf(out + 4), w.mem.rf(out + 8)]
        };
        let r = axis(self, |h, m, a, b, c| h.APIgameobj_GetLookRight(m, a, b, c));
        let u = axis(self, |h, m, a, b, c| h.APIgameobj_GetLookUp(m, a, b, c));
        let t = axis(self, |h, m, a, b, c| h.APIgameobj_GetLookAt(m, a, b, c));
        [0, 1, 2].map(|i| p[i] + r[i] * off[0] + u[i] * off[1] + t[i] * off[2])
    }
    /// group filter of the id (+0x154 == params[1]) / mask (+0x158 & params[1]) retrievers, flag 8 clear
    fn retriever_match(&self, h: ObjH, want: u32, mask: bool) -> bool {
        let o = self.rt.obj(h);
        (if mask { want & o.group_mask != 0 } else { want == o.group_id }) && o.flags & 8 == 0
    }
    /// P20 R52/R53: EFInRangeIdRetriever / EFInRangeMaskRetriever::GetObject (0x00208e50 / 0x00209210),
    /// retrievers 0xff000080 / 0xff000100. params: [0] cursor (1 = start, else the previous hit's id),
    /// [1] group id / mask, [2] centre object (0 = none), [3..6] offset, [6] range, [7] epsilon. Returns
    /// the nearest matching object farther than the previous hit + epsilon and within range; the cursor
    /// becomes its id, or 0x7ffffffe at the end. Without them Poltergeist_BaseFUNC_ChooseThrowable never
    /// found a throwable and TomeDoomFUNC_ContactWisps never reached a wisp.
    fn in_range_retriever(&mut self, params: Ptr, mask: bool) -> Option<ObjH> {
        let sp = self.mem.sp();
        let r = (|| {
            if self.mem.rf(params + 0x1c) <= 0.0 {
                self.mem.wf(params + 0x1c, f32::MIN_POSITIVE);
            }
            let eps = self.mem.rf(params + 0x1c);
            let range = self.mem.rf(params + 0x18);
            let c = self.retriever_centre(params);
            let d2 = |q: [f32; 3]| (q[2] - c[2]) * (q[2] - c[2]) + (q[0] - c[0]) * (q[0] - c[0]) + (q[1] - c[1]) * (q[1] - c[1]);
            let mut max = range * range;
            let cur = self.mem.r32(params);
            let min = if cur == CURSOR_START { 0.0 } else { self.obj_world_pos(cur).map_or(0.0, |q| eps + d2(q)) };
            let want = self.mem.r32(params + 4);
            let mut best = None;
            let mut node = self.rt.map.first();
            while node != 0 {
                let h = self.rt.map.val(node);
                node = self.rt.map.next(node);
                if !self.retriever_match(h, want, mask) {
                    continue;
                }
                let id = self.rt.obj(h).id;
                let Some(q) = self.obj_world_pos(id) else { continue };
                let d = d2(q);
                if d <= max && min <= d {
                    max = if eps < d { d - eps } else { 0.0 };
                    best = Some((h, id));
                }
            }
            match best {
                Some((h, id)) if id != cur => {
                    self.mem.w32(params, id);
                    Some(h)
                }
                _ => {
                    self.mem.w32(params, CURSOR_DONE);
                    None
                }
            }
        })();
        self.mem.set_sp(sp);
        r
    }
    /// P20 R53: EFNearestIDRetriever / EFNearestMaskRetriever::GetObject (0x002086f0.. / 0x002084a0),
    /// retrievers 0xff000002 / 0xff000004: walk the map from the cursor (params[0]: 1 = first node) to
    /// its end (cursor = 0 afterwards) and return the nearest matching object strictly within range
    /// params[6] of the centre. EFNearestRetriever::AcknowledgeNearest: the retriever is created per
    /// call (+0xc = 0), so any nearest hit is returned.
    fn nearest_retriever(&mut self, params: Ptr, mask: bool) -> Option<ObjH> {
        let sp = self.mem.sp();
        let r = (|| {
            let range = self.mem.rf(params + 0x18);
            let c = self.retriever_centre(params);
            let want = self.mem.r32(params + 4);
            let mut best_d = range * range;
            let mut best = None;
            let cur = self.mem.r32(params);
            let mut node = if cur == CURSOR_START { self.rt.map.first() } else { cur };
            while node != 0 {
                let h = self.rt.map.val(node);
                node = self.rt.map.next(node);
                if !self.retriever_match(h, want, mask) {
                    continue;
                }
                let Some(q) = self.obj_world_pos(self.rt.obj(h).id) else { continue };
                let d = (c[2] - q[2]) * (c[2] - q[2]) + (c[0] - q[0]) * (c[0] - q[0]) + (c[1] - q[1]) * (c[1] - q[1]);
                if d < best_d {
                    best_d = d;
                    best = Some(h);
                }
            }
            self.mem.w32(params, 0);
            best
        })();
        self.mem.set_sp(sp);
        r
    }
    /// EFWorld::GetRetriever(id, params) + the retriever's GetObject (vtable +0x10): the next object
    /// selected by retriever id `id`. Outer None = no retriever has that id (GetRetriever returns NULL).
    pub fn retriever_get_object(&mut self, id: u32, params: Ptr) -> Option<Option<ObjH>> {
        match id {
            // EFAllRetriever: every object, no filter
            0xff00_0001 => Some(self.retriever_step(params)),
            // EFGroupIDRetriever (params[1] == object group id) / EFGroupMaskRetriever
            // (params[1] & object group mask); both skip objects with flag 8 and mark the end
            0xff00_0008 | 0xff00_0010 => loop {
                let o = self.retriever_step(params);
                let Some(h) = o else {
                    // params is dereferenced without a NULL check here in the original
                    self.mem.w32(params, CURSOR_DONE);
                    return Some(None);
                };
                let want = self.mem.r32(params + 4);
                let obj = self.rt.obj(h);
                let hit = if id == 0xff00_0008 { want == obj.group_id } else { want & obj.group_mask != 0 };
                if hit && obj.flags & 8 == 0 {
                    return Some(Some(h));
                }
            },
            // GAME_NO_RETRIEVERS=1 (bisect): the pre-R52 behaviour (no position retrievers);
            // =ff000002,ff000100 disables only the listed ones
            0xff00_0080 | 0xff00_0100 | 0xff00_0002 | 0xff00_0004
                if std::env::var("GAME_NO_RETRIEVERS").is_ok_and(|v| v == "1" || v.split(',').any(|t| u32::from_str_radix(t.trim_start_matches("0x"), 16) == Ok(id))) =>
            {
                if params != NULL { self.mem.w32(params, CURSOR_DONE); }
                Some(None)
            }
            0xff00_0080 | 0xff00_0100 | 0xff00_0002 | 0xff00_0004 if params != NULL => {
                let pv: Vec<u32> = (0..8).map(|i| self.mem.r32(params + 4 * i)).collect();
                let r = if id & 0x180 != 0 { self.in_range_retriever(params, id == 0xff00_0100) } else { self.nearest_retriever(params, id == 0xff00_0004) };
                // GAME_RETRLOG=1 (debug): every position retriever call (params before, hit uid)
                if std::env::var_os("GAME_RETRLOG").is_some() {
                    eprintln!("RETR {id:#x} params {pv:x?} -> {:?}", r.map(|h| format!("{:#x}", self.rt.obj(h).id)));
                }
                Some(r)
            }
            // parent / children: need the scene graph, which the platform layer owns
            0xff00_0002 | 0xff00_0004 | 0xff00_0080 | 0xff00_0100 | 0xff00_0020 | 0xff00_0040 => {
                let r = self.host.retriever_get_object(&mut self.mem, id, params);
                // WP-V1: a host without this retriever yields nothing; mark the cursor done like
                // the map-walking retrievers so cursor loops (`while (params[0] != 0x7ffffffe)`,
                // e.g. Poltergeist_BaseFUNC_ChooseThrowable) terminate instead of spinning
                if r.is_none() && params != NULL {
                    self.mem.w32(params, CURSOR_DONE);
                }
                Some(r.and_then(|i| self.rt.map.find(i)))
            }
            _ => None,
        }
    }
}
