//! WP-X3: the remaining per-build tables (hero/panel/camera/ghost/sectormanager/hs_door) are built from
//! the module ELFs on the disc. Builds every row of every table (each literal must decode). With
//! X3_EXPECT=<dir> each row's `{:?}` is compared with <dir>/<table>.txt (one row per line), the dump
//! of the compiled-in table it replaced (written by X3_DUMP=<dir> on the old tree).
use scooby_recomp::engine::iso::IsoFs;
use scooby_recomp::modules::*;

use crate::common::ISO;

macro_rules! tabs {
    ($($name:literal => $t:expr),* $(,)?) => {
        vec![$(($name, Box::new(|| (0..$t.len()).map(|b| format!("{:?}", $t[b])).collect::<Vec<String>>()) as Box<dyn Fn() -> Vec<String>>)),*]
    };
}

#[test]
fn x3_tables() {
    if IsoFs::open(ISO).is_err() {
        return;
    }
    scooby_recomp::engine::gamedata::set_iso_path(std::path::Path::new(ISO));
    let dump = std::env::var("X3_DUMP").ok();
    let expect = std::env::var("X3_EXPECT").ok();
    let all = tabs! {
        "hero_valide_current_sound" => hero::tables::VALIDE_CURRENT_SOUND,
        "hero_char_name" => hero::tables::CHAR_NAME,
        "hero_is_shaggy" => hero::tables::IS_SHAGGY,
        "hero_debug_build" => hero::tables::DEBUG_BUILD,
        "hero_check_action_events" => hero::tables::CHECK_ACTION_EVENTS,
        "hero_level_item_260" => hero::tables::LEVEL_ITEM_260,
        "hero_msg_params" => hero::msg_tab::MSG_PARAMS,
        "hero_ctor" => hero::ctor_tables::HERO_CTOR,
        "hero_nextmove" => hero::nextmove_tab::NEXTMOVE_PARAMS,
        "panel_local" => panel::local::LOCAL,
        "panel_display_info" => panel::local::DISPLAY_INFO,
        "panel_keys" => panel::keys::KEYS,
        "camera_vars" => camera::VARS,
        "camera_vars2" => camera::VARS2,
        "ghost_emitter_pos" => [ghost::emitter_pos::EMITTER_POS_1, ghost::emitter_pos::EMITTER_POS_2, ghost::emitter_pos::EMITTER_POS_3, ghost::emitter_pos::EMITTER_POS_13],
        "sectormanager" => sectormanager_builds::BUILDS,
        "hs_door_vars" => hs_door::builds::VARS,
    };
    for (name, rows) in all {
        let rows = rows();
        if let Some(d) = &dump {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(format!("{d}/{name}.txt"), rows.join("\n") + "\n").unwrap();
        }
        let mut n = 0;
        if let Some(d) = &expect {
            let t = std::fs::read_to_string(format!("{d}/{name}.txt")).unwrap_or_else(|_| panic!("no {d}/{name}.txt"));
            let old: Vec<&str> = t.lines().collect();
            assert_eq!(old.len(), rows.len(), "{name}: row count");
            for (b, (o, r)) in old.iter().zip(&rows).enumerate() {
                assert_eq!(o, r, "{name} row {b}");
                n += 1;
            }
        }
        println!("{name}: {} rows, {n} compared", rows.len());
    }
}
