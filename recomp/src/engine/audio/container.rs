//! Child selection shared by EFSoundContainer::PlaySound (0x00268940) and
//! EFSoundEvent_BC::PlaySound (0x002420d0; event data +0xc kind, +0x24 count, +0x40 list, state
//! this+0x6c). Types 4/5 exist only in the container version (events: index 0).
//! (EFSoundContainerData +0x8 kind, +0xc count, +0x1c children; instance state this+0x40).
use super::sbh::Rand;
use super::tables::EFSoundContainerData;

pub const CT_RANDOM: u32 = 0; // random, never the same twice (count 1 -> 0, count 2 -> toggle)
pub const CT_SHUFFLE: u32 = 2; // re-shuffle children in place, then play index 0
pub const CT_SEQUENCE: u32 = 3; // next index, wrapping
pub const CT_ALL: u32 = 4; // play every child
pub const CT_BY_OBJECT: u32 = 5; // index from the owner object's ushort at (+0x28)->+0xb8->+0x16

/// Per-instance state (EFSoundContainer +0x40 current index, initialised to -1).
#[derive(Debug, Clone)]
pub struct ContainerState {
    pub index: i32,
}
impl Default for ContainerState {
    fn default() -> Self { ContainerState { index: -1 } }
}

/// Returns the child ids to play. `d.children` may be permuted (type 2 shuffles the shared
/// container data, as the original does). `obj_index` = the owner value for type 5 (None -> 0).
pub fn select(d: &mut EFSoundContainerData, st: &mut ContainerState, rng: &mut Rand, obj_index: Option<u32>) -> Vec<u32> {
    select_list(d.kind, &mut d.children, true, st, rng, obj_index)
}

/// Generic form; `container` enables types 4 and 5.
pub fn select_list(kind: u32, children: &mut Vec<u32>, container: bool, st: &mut ContainerState, rng: &mut Rand, obj_index: Option<u32>) -> Vec<u32> {
    let kind = if !container && (kind == CT_ALL || kind == CT_BY_OBJECT) { 1 } else { kind };
    let n = children.len() as i32;
    match kind {
        CT_RANDOM => {
            st.index = match n {
                1 => 0,
                2 => (st.index == 0) as i32,
                _ if n > 0 => {
                    let mut i = rng.rand() % n;
                    if i == st.index {
                        i = (i + 1) % n;
                    }
                    i
                }
                _ => 0,
            }
        }
        CT_BY_OBJECT => {
            st.index = obj_index.map_or(0, |v| v as i32);
            if children.get(st.index as usize).map_or(true, |&c| c == u32::MAX) {
                st.index = 0;
            }
            if obj_index.is_none() {
                return vec![];
            }
        }
        CT_SHUFFLE if n > 1 => {
            let src: Vec<u32> = children.clone();
            children.iter_mut().for_each(|c| *c = u32::MAX);
            for v in src {
                loop {
                    let k = (rng.rand() % n) as usize;
                    if children[k] == u32::MAX {
                        children[k] = v;
                        break;
                    }
                }
            }
            st.index = 0;
        }
        CT_SEQUENCE => {
            st.index = if st.index == -1 { 0 } else { st.index + 1 };
            if st.index >= n {
                st.index = 0;
            }
        }
        _ => st.index = 0,
    }
    if kind == CT_ALL {
        return children.iter().copied().filter(|&c| c != u32::MAX).collect();
    }
    children.get(st.index as usize).copied().filter(|&c| c != u32::MAX).into_iter().collect()
}
