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
            // nearest / in-range / parent / children: need positions and the scene graph, which the
            // platform layer owns
            0xff00_0002 | 0xff00_0004 | 0xff00_0020 | 0xff00_0040 | 0xff00_0080 | 0xff00_0100 => {
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
