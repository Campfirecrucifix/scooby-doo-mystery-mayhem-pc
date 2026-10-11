//! Level -> logic bridge: EFLogicCode / EFLogicCodeLib entries of a pack (classes 0x27 / 0x45),
//! EFGameLogicMgr::AddLogicCode / GetLogicCode, and EFGameObject's logic binding
//! (EFGameObject::LoadFromStream: this+0x38 = GetLogicCode(glo GLO_FILE_ID); latent split on
//! UID & 0x800000; else Initialize + EFWorld::AddGameObject).
//!
//! Platform replacement (rule 2): the PS2 loads the entry's ELF image and relocates it
//! (EFLogicCode::LoadFile). The port has every module compiled in; an entry is matched to its
//! ported [`ModuleDef`] by the SHA-1 of its ELF image (the same key the module's `BUILDS` table
//! uses), falling back to the module name for single-build modules.

#![allow(non_snake_case)]

use super::Engine;
use super::pack::EFRessourcesMgr;
use crate::logic::{CodeId, ModuleDef, ObjH};
use std::collections::HashMap;

/// One code entry of a pack: `{u32 uid, u32 arg, u32 nameLen, char name[nameLen] ('.' as '\0')}`
/// then the ELF image.
#[derive(Debug, Clone)]
pub struct LogicCodeEntry {
    pub class_id: u32,
    pub uid: u32,
    pub arg: u32,
    /// e.g. "tomedoom.op2"
    pub name: String,
    /// first 8 hex digits of the SHA-1 of the ELF image
    pub sha8: String,
}

fn elf_size(b: &[u8]) -> usize {
    let r32 = |o: usize| b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes(s.try_into().unwrap()) as usize);
    let r16 = |o: usize| b.get(o..o + 2).map_or(0, |s| u16::from_le_bytes(s.try_into().unwrap()) as usize);
    let (shoff, shentsize, shnum) = (r32(0x20), r16(0x2e), r16(0x30));
    let mut end = shoff + shentsize * shnum;
    for i in 0..shnum {
        let sh = shoff + i * shentsize;
        if sh + 40 > b.len() {
            break;
        }
        if r32(sh + 4) != 8 {
            end = end.max(r32(sh + 16) + r32(sh + 20));
        }
    }
    end.min(b.len())
}

impl LogicCodeEntry {
    /// Parses a raw 0x27/0x45 entry; None if it holds no ELF image.
    pub fn parse(class_id: u32, d: &[u8]) -> Option<Self> {
        if d.len() < 12 {
            return None;
        }
        let w = |o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
        let n = w(8) as usize;
        let elf = d.get(12 + n..)?;
        if elf.len() < 52 || &elf[..4] != b"\x7fELF" {
            return None;
        }
        let raw = &d[12..12 + n];
        let end = raw.iter().rposition(|&c| c != 0).map_or(0, |i| i + 1);
        let name: String = raw[..end].iter().map(|&c| if c == 0 { '.' } else { c as char }).collect();
        // stale bytes after the first NUL: keep "<base>.<ext>" only
        let name = name.split('.').take(2).collect::<Vec<_>>().join(".");
        let img = &elf[..elf_size(elf)];
        let sha8 = super::gamedata::sha8(img);
        super::gamedata::register_elf(&sha8, img);
        Some(Self { class_id, uid: w(0), arg: w(4), name, sha8 })
    }

    pub fn base_name(&self) -> String {
        self.name.split('.').next().unwrap_or("").to_ascii_lowercase()
    }
}

type BuildFn = fn(&str) -> Option<usize>;

/// Every ported module: (def, build lookup by sha8; None = single build, matched by name).
pub fn PortedModules() -> Vec<(&'static ModuleDef, Option<BuildFn>)> {
    use crate::libs::*;
    use crate::modules::*;
    vec![
        (&anim_mngr::MODULE, None),
        (&api_c_functions::MODULE, Some(api_c_functions::build_index as BuildFn)),
        (&api_text::MODULE, None),
        (&misc_func::MODULE, None),
        (&api_list::MODULE, None),
        (&api_sector_clipping::MODULE, None),
        (&api_path_finding::MODULE, Some(api_path_finding::build_index as BuildFn)),
        (&debug::MODULE, None),
        (&snd_mngr::MODULE, None),
        (&api_memcard::MODULE, None),
        (&api_loadsave_debug::MODULE, Some(api_loadsave_debug::build_index as BuildFn)),
        (&memorymanager::MODULE, Some(memorymanager::build_index)),
        (&sectormanager::MODULE, Some(sectormanager::build_index)),
        (&dustdevil::MODULE, Some(dustdevil::build_index)),
        (&fireghost::MODULE, Some(fireghost::build_index)),
        (&fireghostboss::MODULE, None),
        (&generic_fx::MODULE, None),
        (&l01_camera::MODULE, None),
        (&ghost::MODULE, Some(ghost::build_index)),
        (&hero::MODULE, Some(hero::build_index)),
        (&hero_ghost::MODULE, Some(hero_ghost::build_index)),
        (&mummy::MODULE, Some(mummy::build_index)),
        (&mummyride::MODULE, None),
        (&mummy_binding::MODULE, None),
        (&panel::MODULE, Some(panel::build_index)),
        (&poltergeist::MODULE, Some(poltergeist::build_index)),
        (&scientist::MODULE, Some(scientist::build_index)),
        (&shadows::MODULE, Some(shadows::build_index)),
        (&tod_beam::MODULE, Some(tod_beam::build_index)),
        (&tomedoom::MODULE, Some(tomedoom::build_index)),
        (&wisps::MODULE, Some(wisps::build_index)),
        (&wispsourcenew::MODULE, Some(wispsourcenew::build_index)),
        (&zombie::MODULE, Some(zombie::build_index)),
        (&zombieboss::MODULE, Some(zombieboss::build_index)),
        (&zombieboss_binding::MODULE, None),
        (&rollingladder::MODULE, Some(rollingladder::build_index)),
        (&cartstype01::MODULE, Some(cartstype01::build_index)),
        (&beamoflight::MODULE, Some(beamoflight::build_index)),
        (&inv::MODULE, Some(inv::build_index)),
        (&costume::MODULE, Some(costume::build_index)),
        (&pizza::MODULE, Some(pizza::build_index)),
        (&candle::MODULE, Some(candle::build_index)),
        (&chkpnt::MODULE, Some(chkpnt::build_index)),
        (&misc_hud::MODULE, None),
        (&enemybar::MODULE, Some(enemybar::build_index)),
        (&enmyspot::MODULE, Some(enmyspot::build_index)),
        (&herospot::MODULE, Some(herospot::build_index)),
        (&rdrmngr::MODULE, Some(rdrmngr::build_index)),
        (&panel_child::MODULE, Some(panel_child::build_index)),
        (&stand_in_obj::MODULE, Some(stand_in_obj::build_index)),
        (&invnorthpos::MODULE, Some(invnorthpos::build_index)),
        (&roomlocator::MODULE, Some(roomlocator::build_index)),
        (&hs_tutorial::MODULE, Some(hs_tutorial::build_index)),
        (&hs_door_l03::MODULE, Some(hs_door_l03::build_index)),
        (&chest_level1::MODULE, Some(chest_level1::build_index)),
        (&chest_level3a::MODULE, Some(chest_level3a::build_index)),
        (&coll_level1::MODULE, Some(coll_level1::build_index)),
        (&coll_level3a::MODULE, Some(coll_level3a::build_index)),
        (&inv_tutorial::MODULE, Some(inv_tutorial::build_index)),
        (&velma::MODULE, Some(velma::build_index)),
        (&inv_control::MODULE, Some(inv_control::build_index)),
        (&inv_control_gen::MODULE, Some(inv_control_gen::build_index)),
        (&savepanel::MODULE, Some(savepanel::build_index)),
        (&zvelma::MODULE, Some(zvelma::build_index)),
        (&costume_scooby::MODULE, Some(costume_scooby::build_index)),
        (&costume_shaggy::MODULE, Some(costume_shaggy::build_index)),
        (&costume_scooby_scientist::MODULE, Some(costume_scooby_scientist::build_index)),
        (&costume_shaggy_scientist::MODULE, Some(costume_shaggy_scientist::build_index)),
        (&costume_scooby_snorkel::MODULE, Some(costume_scooby_snorkel::build_index)),
        (&costume_scooby_cowboy::MODULE, Some(costume_scooby_cowboy::build_index)),
        (&costume_shaggy_cowboy::MODULE, Some(costume_shaggy_cowboy::build_index)),
        (&costume_scooby_mummy::MODULE, Some(costume_scooby_mummy::build_index)),
        (&costume_shaggy_mummy::MODULE, Some(costume_shaggy_mummy::build_index)),
        (&costume_scooby_security_guard::MODULE, Some(costume_scooby_security_guard::build_index)),
        (&costume_shaggy_security_guard::MODULE, Some(costume_shaggy_security_guard::build_index)),
        (&costume_scooby_spacesuit::MODULE, Some(costume_scooby_spacesuit::build_index)),
        (&costume_shaggy_spacesuit::MODULE, Some(costume_shaggy_spacesuit::build_index)),
        (&zfred::MODULE, Some(zfred::build_index)),
        (&zdaphne::MODULE, Some(zdaphne::build_index)),
        (&shaggy_level2b::MODULE, Some(shaggy_level2b::build_index)),
        (&vault_door::MODULE, Some(vault_door::build_index)),
        (&streetbelle::MODULE, Some(streetbelle::build_index)),
        (&shorelineclip::MODULE, Some(shorelineclip::build_index)),
        (&shoreline2::MODULE, Some(shoreline2::build_index)),
        (&shoreline::MODULE, Some(shoreline::build_index)),
        (&safedialframe::MODULE, Some(safedialframe::build_index)),
        (&safe_door::MODULE, Some(safe_door::build_index)),
        (&rideobstacle::MODULE, Some(rideobstacle::build_index)),
        (&radar_dish::MODULE, Some(radar_dish::build_index)),
        (&obstlatant::MODULE, Some(obstlatant::build_index)),
        (&obj_launcher_level02::MODULE, Some(obj_launcher_level02::build_index)),
        (&mcrock::MODULE, Some(mcrock::build_index)),
        (&mcboulder::MODULE, Some(mcboulder::build_index)),
        (&mc_lantern::MODULE, Some(mc_lantern::build_index)),
        (&hs_safe::MODULE, Some(hs_safe::build_index)),
        (&hs_looping::MODULE, Some(hs_looping::build_index)),
        (&hs_door_l04::MODULE, Some(hs_door_l04::build_index)),
        (&hs_door_l02::MODULE, Some(hs_door_l02::build_index)),
        (&hs_cutscene::MODULE, Some(hs_cutscene::build_index)),
        (&hs_changelevel::MODULE, Some(hs_changelevel::build_index)),
        (&hs_cam_l04::MODULE, Some(hs_cam_l04::build_index)),
        (&hs_automaticjump::MODULE, Some(hs_automaticjump::build_index)),
        (&gramophone::MODULE, Some(gramophone::build_index)),
        (&fireplace::MODULE, Some(fireplace::build_index)),
        (&fireballbig::MODULE, Some(fireballbig::build_index)),
        (&fatcowboy::MODULE, Some(fatcowboy::build_index)),
        (&fake_alien_b::MODULE, Some(fake_alien_b::build_index)),
        (&fake_alien_a::MODULE, Some(fake_alien_a::build_index)),
        (&ennemymcart::MODULE, Some(ennemymcart::build_index)),
        (&electricarc_local::MODULE, Some(electricarc_local::build_index)),
        (&deadendlatant::MODULE, Some(deadendlatant::build_index)),
        (&watercastleyard::MODULE, Some(watercastleyard::build_index)),
        (&panneaux_csq::MODULE, Some(panneaux_csq::build_index)),
        (&hs_sprinkler::MODULE, Some(hs_sprinkler::build_index)),
        (&hs_displaytext::MODULE, Some(hs_displaytext::build_index)),
        (&hs_commun::MODULE, Some(hs_commun::build_index)),
        (&crane::MODULE, Some(crane::build_index)),
        (&colomn::MODULE, Some(colomn::build_index)),
        (&coll_level2a::MODULE, Some(coll_level2a::build_index)),
        (&ceiling_sprinkler::MODULE, Some(ceiling_sprinkler::build_index)),
        (&catapult_02a::MODULE, Some(catapult_02a::build_index)),
        (&castlegate::MODULE, Some(castlegate::build_index)),
        (&cartstype03::MODULE, Some(cartstype03::build_index)),
        (&gamecontroller::MODULE, Some(gamecontroller::build_index)),
        (&flash_poly::MODULE, Some(flash_poly::build_index)),
        (&fireballbig_expl::MODULE, Some(fireballbig_expl::build_index)),
        (&democontrol::MODULE, Some(democontrol::build_index)),
        (&coll_level9::MODULE, Some(coll_level9::build_index)),
        (&coll_level5::MODULE, Some(coll_level5::build_index)),
        (&coll_level4::MODULE, Some(coll_level4::build_index)),
        (&coll_level2::MODULE, Some(coll_level2::build_index)),
        (&bbride::MODULE, Some(bbride::build_index)),
        (&animated_csq::MODULE, Some(animated_csq::build_index)),
        (&coll_level3::MODULE, Some(coll_level3::build_index)),
        (&shockwave::MODULE, Some(shockwave::build_index)),
        (&shgride::MODULE, Some(shgride::build_index)),
        (&scoreboard::MODULE, Some(scoreboard::build_index)),
        (&scbride::MODULE, Some(scbride::build_index)),
        (&minigame_initialtimer::MODULE, Some(minigame_initialtimer::build_index)),
        (&hs_rideobst::MODULE, Some(hs_rideobst::build_index)),
        (&hs_changetrack::MODULE, Some(hs_changetrack::build_index)),
        (&camera::MODULE, Some(camera::build_index)),
        (&alligator::MODULE, Some(alligator::build_index)),
        (&throwable_base::MODULE, Some(throwable_base::build_index)),
        (&fireball::MODULE, Some(fireball::build_index)),
        (&falling_key::MODULE, Some(falling_key::build_index)),
        (&automaticdoor::MODULE, Some(automaticdoor::build_index)),
        (&moviecamera::MODULE, Some(moviecamera::build_index)),
        (&transparent::MODULE, Some(transparent::build_index)),
        (&bridge::MODULE, Some(bridge::build_index)),
        (&clamp::MODULE, Some(clamp::build_index)),
        (&lantern::MODULE, Some(lantern::build_index)),
        (&powerup::MODULE, Some(powerup::build_index)),
        (&teleport::MODULE, Some(teleport::build_index)),
        (&teleport_floor::MODULE, Some(teleport_floor::build_index)),
        (&rotatingdoors::MODULE, Some(rotatingdoors::build_index)),
        (&daphne::MODULE, Some(daphne::build_index)),
        (&fred::MODULE, Some(fred::build_index)),
        (&hs_checkpoint::MODULE, Some(hs_checkpoint::build_index)),
        (&green_pulse::MODULE, Some(green_pulse::build_index)),
        (&force_field_local::MODULE, Some(force_field_local::build_index)),
        (&velma_gen::MODULE, Some(velma_gen::build_index)),
        (&button::MODULE, Some(button::build_index)),
        (&hs_door_l01::MODULE, Some(hs_door_l01::build_index)),
        (&hs_door::MODULE, Some(hs_door::build_index)),
        (&level01_projectors::MODULE, Some(level01_projectors::build_index)),
        (&velmanew::MODULE, Some(velmanew::build_index)),
        (&pagecs::MODULE, Some(pagecs::build_index)),
        (&ghost_lock::MODULE, Some(ghost_lock::build_index)),
        (&planefog::MODULE, Some(planefog::build_index)),
        (&door::MODULE, Some(door::build_index)),
        (&obj_launcher_level3a::MODULE, Some(obj_launcher_level3a::build_index)),
        (&l03_camera::MODULE, Some(l03_camera::build_index)),
        (&title_camera::MODULE, Some(title_camera::build_index)),
        (&title_camera::OVERALL, Some(title_camera::overall_build_index)),
        (&tbcs::MODULE, Some(tbcs::build_index)),
        (&panel_gameart::MODULE, Some(panel_gameart::build_index)),
        (&panel_credits::MODULE, Some(panel_credits::build_index)),
        (&joypanel::MODULE, Some(joypanel::build_index)),
        (&fearexplosion::MODULE, None),
        (&fearwave::MODULE, Some(fearwave::build_index)),
        (&dome::MODULE, Some(dome::build_index)),
        (&torche::MODULE, Some(torche::build_index)),
        (&animated_water::MODULE, Some(animated_water::build_index)),
        (&fearghost::MODULE, None),
        (&switch::MODULE, Some(switch::build_index)),
        (&switch::MODULE_CUTSWITCH, Some(switch::build_index)),
        (&switch_fred::MODULE, None),
        (&zombiegroup::MODULE, None),
        (&switchdoor::MODULE, None),
        (&hs_launchenemy::MODULE, Some(hs_launchenemy::build_index)),
        (&fakezombie::MODULE, None),
        (&fakezombie::MODULE_B, None),
        (&redknight::MODULE, None),
        (&swampcreature::MODULE, None),
        (&velmaboss::MODULE, None),
        (&force_field_zombie_local::MODULE, None),
        (&fakeghost::MODULE, Some(fakeghost::build_index)),
        (&inv_switchsolver_01a::MODULE, None),
        (&title_monsters::MODULE_SMALL, None),
        (&title_monsters::MODULE_MED, None),
        (&title_monsters::MODULE_BIG, None),
        (&title_monsters::MODULE_MUMMY, None),
        (&puzzlemonsters::MODULE_GHOST, None),
        (&puzzlemonsters::MODULE_MEDGHOST, None),
        (&puzzlemonsters::MODULE_BIGGHOST, None),
        (&puzzlemonsters::MODULE_MUMMY, None),
        (&puzzlemonsters::MODULE_MINDY, None),
        (&stand_in_obj_more::MODULE, None),
        (&stand_in_obj_more::MODULE_CUTSCENE, None),
        (&stand_in_obj_more::MODULE_PHYSIC, None),
        (&cut::MODULE, None),
        (&cut::MODULE1, None),
        (&cut::MODULE2, None),
        (&cut::MODULE3, None),
        (&cut::MODULE4, None),
        (&cut::MODULE5, None),
        (&cut::MODULE6, None),
        (&cut::MODULE7, None),
        (&cut::MODULE8, None),
        (&armyman::MODULE, Some(armyman::build_index)),
        (&bankrobber::MODULE, Some(bankrobber::build_index)),
        (&blackcowboy::MODULE, Some(blackcowboy::build_index)),
        (&arrow::MODULE, Some(arrow::build_index)),
        (&airboat::MODULE, Some(airboat::build_index)),
        (&boatradar::MODULE, Some(boatradar::build_index)),
        (&cagecrane_base::MODULE, Some(cagecrane_base::build_index)),
        (&cardboard_alien::MODULE, Some(cardboard_alien::build_index)),
        (&sarcophage::MODULE, Some(sarcophage::build_index)),
        (&lockerdoor::MODULE, Some(lockerdoor::build_index)),
        (&vendingmachine::MODULE, Some(vendingmachine::build_index)),
        (&video_camhead::MODULE, Some(video_camhead::build_index)),
        (&quai::MODULE, Some(quai::build_index)),
        (&ring::MODULE, Some(ring::build_index)),
        (&rain::MODULE, Some(rain::build_index)),
        (&trailbike::MODULE, Some(trailbike::build_index)),
        (&cartstype02::MODULE, Some(cartstype02::build_index)),
        (&chest_level2::MODULE, Some(chest_level2::build_index)),
        (&l04_camera::MODULE, Some(l04_camera::build_index)),
        (&l04a_camera::MODULE, Some(l04a_camera::build_index)),
        (&l05_camera::MODULE, Some(l05_camera::build_index)),
        (&l02_camera::MODULE, Some(l02_camera::build_index)),
        (&l09_camera::MODULE, Some(l09_camera::build_index)),
        (&loader_camera::MODULE, Some(loader_camera::build_index)),
        (&hs_door_l02c::MODULE, Some(hs_door_l02c::build_index)),
        (&hs_inout_crypt::MODULE, Some(hs_inout_crypt::build_index)),
        (&hs_keycard::MODULE, Some(hs_keycard::build_index)),
        (&hs_open_door::MODULE, Some(hs_open_door::build_index)),
        (&hs_lap::MODULE, Some(hs_lap::build_index)),
        (&hs_goback::MODULE, Some(hs_goback::build_index)),
        (&hs_blockhero::MODULE, Some(hs_blockhero::build_index)),
        (&hs_cam_l03::MODULE, Some(hs_cam_l03::build_index)),
        (&hs_cam_l02::MODULE, Some(hs_cam_l02::build_index)),
        (&hs_cam_l01::MODULE, Some(hs_cam_l01::build_index)),
        (&frankenstein::MODULE, Some(frankenstein::build_index)),
        (&fallingbrick::MODULE, Some(fallingbrick::build_index)),
        (&fallingrock::MODULE, Some(fallingrock::build_index)),
        (&rolling_barrel::MODULE, Some(rolling_barrel::build_index)),
        (&rolling_boulder::MODULE, Some(rolling_boulder::build_index)),
        (&swinging_blades::MODULE, Some(swinging_blades::build_index)),
        (&crate_falling::MODULE, Some(crate_falling::build_index)),
        (&crate_swinging::MODULE, Some(crate_swinging::build_index)),
        (&lava_geiser::MODULE, Some(lava_geiser::build_index)),
        (&shortcircuit::MODULE, Some(shortcircuit::build_index)),
        (&steamemitter::MODULE, Some(steamemitter::build_index)),
        (&steamemitterroom03::MODULE, Some(steamemitterroom03::build_index)),
        (&fan::MODULE, Some(fan::build_index)),
        (&hatch::MODULE, Some(hatch::build_index)),
        (&ladder::MODULE, Some(ladder::build_index)),
        (&ladder_level2a::MODULE, Some(ladder_level2a::build_index)),
        (&rope::MODULE, Some(rope::build_index)),
        (&stairs::MODULE, Some(stairs::build_index)),
        (&valve::MODULE, Some(valve::build_index)),
        (&l05_valve::MODULE, Some(l05_valve::build_index)),
        (&weightbutton::MODULE, Some(weightbutton::build_index)),
        (&weightpuzzle::MODULE, Some(weightpuzzle::build_index)),
        (&wring::MODULE, Some(wring::build_index)),
        (&wind::MODULE, Some(wind::build_index)),
        (&selena::MODULE, Some(selena::build_index)),
        (&bad_guys::MODULE, Some(bad_guys::build_index)),
        (&mercenaryleveltwoa::MODULE, Some(mercenaryleveltwoa::build_index)),
        (&mercenaryleveltwob::MODULE, Some(mercenaryleveltwob::build_index)),
        (&securityguardlevelone::MODULE, Some(securityguardlevelone::build_index)),
        (&patrollingguardlevelone::MODULE, Some(patrollingguardlevelone::build_index)),
        (&truckmercenary::MODULE, Some(truckmercenary::build_index)),
        (&robot::MODULE, Some(robot::build_index)),
        (&robotcrawl::MODULE, Some(robotcrawl::build_index)),
        (&jeremyrhodes_level4::MODULE, Some(jeremyrhodes_level4::build_index)),
        (&earllevelthreea::MODULE, Some(earllevelthreea::build_index)),
        (&earlleveltwoa::MODULE, Some(earlleveltwoa::build_index)),
        (&earlleveltwob::MODULE, Some(earlleveltwob::build_index)),
        (&earlleveltwoc::MODULE, Some(earlleveltwoc::build_index)),
        (&billybob_follow::MODULE, Some(billybob_follow::build_index)),
        (&billybob_level2b::MODULE, Some(billybob_level2b::build_index)),
        (&billybob_level4::MODULE, Some(billybob_level4::build_index)),
        (&chest_level1b::MODULE, Some(chest_level1b::build_index)),
        (&chest_level2a::MODULE, Some(chest_level2a::build_index)),
        (&chest_level3b::MODULE, Some(chest_level3b::build_index)),
        (&chest_level4::MODULE, Some(chest_level4::build_index)),
        (&chest_level4a::MODULE, Some(chest_level4a::build_index)),
        (&chest_level4b::MODULE, Some(chest_level4b::build_index)),
        (&chest_scientist::MODULE, Some(chest_scientist::build_index)),
        (&coll_level1a::MODULE, Some(coll_level1a::build_index)),
        (&coll_level1b::MODULE, Some(coll_level1b::build_index)),
        (&coll_level1c::MODULE, Some(coll_level1c::build_index)),
        (&coll_level2b::MODULE, Some(coll_level2b::build_index)),
        (&coll_level2c::MODULE, Some(coll_level2c::build_index)),
        (&coll_level3b::MODULE, Some(coll_level3b::build_index)),
        (&coll_level4b::MODULE, Some(coll_level4b::build_index)),
        (&button_l3b::MODULE, Some(button_l3b::build_index)),
        (&button_level1::MODULE, Some(button_level1::build_index)),
        (&button_level1c::MODULE, Some(button_level1c::build_index)),
        (&arrowbutton::MODULE, Some(arrowbutton::build_index)),
        (&framebutton::MODULE, Some(framebutton::build_index)),
        (&robotbutton::MODULE, Some(robotbutton::build_index)),
        (&panelbutton::MODULE, Some(panelbutton::build_index)),
        (&obj_launcher_level1c::MODULE, Some(obj_launcher_level1c::build_index)),
        (&obj_launcher_level3b::MODULE, Some(obj_launcher_level3b::build_index)),
        (&obj_launcher_level4::MODULE, Some(obj_launcher_level4::build_index)),
        (&obj_launcher_level5::MODULE, Some(obj_launcher_level5::build_index)),
        (&keycagelevelthreea::MODULE, Some(keycagelevelthreea::build_index)),
        (&keycageleveltwoa::MODULE, Some(keycageleveltwoa::build_index)),
        (&robotlift::MODULE, Some(robotlift::build_index)),
        (&robotvalve::MODULE, Some(robotvalve::build_index)),
        (&rocket::MODULE, Some(rocket::build_index)),
        (&siegetower::MODULE, Some(siegetower::build_index)),
        (&thrownobj::MODULE, Some(thrownobj::build_index)),
        (&traindoor::MODULE, Some(traindoor::build_index)),
        (&treebranche::MODULE, Some(treebranche::build_index)),
        (&truck::MODULE, Some(truck::build_index)),
        (&jeep::MODULE, Some(jeep::build_index)),
        (&wagon::MODULE, Some(wagon::build_index)),
        (&mining_cart::MODULE, Some(mining_cart::build_index)),
        (&labcart::MODULE, Some(labcart::build_index)),
        (&lightbridge::MODULE, Some(lightbridge::build_index)),
        (&mechbridge::MODULE, Some(mechbridge::build_index)),
        (&lockeddowndoors::MODULE, Some(lockeddowndoors::build_index)),
        (&fountain::MODULE, Some(fountain::build_index)),
        (&fountain_gate::MODULE, Some(fountain_gate::build_index)),
        (&eau_fountain::MODULE, Some(eau_fountain::build_index)),
        (&water::MODULE, Some(water::build_index)),
        (&water_big::MODULE, Some(water_big::build_index)),
        (&water_gates::MODULE, Some(water_gates::build_index)),
        (&water_leak::MODULE, Some(water_leak::build_index)),
        (&watersprinkler::MODULE, Some(watersprinkler::build_index)),
        (&swampwater::MODULE, Some(swampwater::build_index)),
        (&swampwater2::MODULE, Some(swampwater2::build_index)),
        (&liquidfloor::MODULE, Some(liquidfloor::build_index)),
        (&humidfloor::MODULE, Some(humidfloor::build_index)),
        (&hs_console::MODULE, Some(hs_console::build_index)),
        (&hs_forcefield::MODULE, Some(hs_forcefield::build_index)),
        (&hs_found_shaggy::MODULE, Some(hs_found_shaggy::build_index)),
        (&hs_gear::MODULE, Some(hs_gear::build_index)),
        (&hs_grid::MODULE, Some(hs_grid::build_index)),
        (&hs_mercenary::MODULE, Some(hs_mercenary::build_index)),
        (&hs_piano::MODULE, Some(hs_piano::build_index)),
        (&hs_poltergeist::MODULE, Some(hs_poltergeist::build_index)),
        (&hs_puzzlepanel::MODULE, Some(hs_puzzlepanel::build_index)),
        (&hs_robotcommand::MODULE, Some(hs_robotcommand::build_index)),
        (&hs_robotdoor::MODULE, Some(hs_robotdoor::build_index)),
        (&hs_shaggywait::MODULE, Some(hs_shaggywait::build_index)),
        (&hs_window_l03::MODULE, Some(hs_window_l03::build_index)),
        (&hs_wpuzzle::MODULE, Some(hs_wpuzzle::build_index)),
        (&hotspot_diagramvo::MODULE, Some(hotspot_diagramvo::build_index)),
        (&hotspot_forcefield::MODULE, Some(hotspot_forcefield::build_index)),
        (&altar::MODULE, Some(altar::build_index)),
        (&barrier::MODULE, Some(barrier::build_index)),
        (&big_pipe::MODULE, Some(big_pipe::build_index)),
        (&boat::MODULE, Some(boat::build_index)),
        (&boatdoor::MODULE, Some(boatdoor::build_index)),
        (&carrousel::MODULE, Some(carrousel::build_index)),
        (&castledoor::MODULE, Some(castledoor::build_index)),
        (&catapult_base::MODULE, Some(catapult_base::build_index)),
        (&cell_bars::MODULE, Some(cell_bars::build_index)),
        (&coffindoor::MODULE, Some(coffindoor::build_index)),
        (&conelight::MODULE, Some(conelight::build_index)),
        (&desert_bridge::MODULE, Some(desert_bridge::build_index)),
        (&egyptian_door::MODULE, Some(egyptian_door::build_index)),
        (&mcart::MODULE, Some(mcart::build_index)),
        (&mchelmet::MODULE, Some(mchelmet::build_index)),
        (&safepuzzle::MODULE, Some(safepuzzle::build_index)),
        (&teleport_lockdown::MODULE, Some(teleport_lockdown::build_index)),
        (&controller::MODULE, Some(controller::build_index)),
        (&dirt::MODULE, Some(dirt::build_index)),
        (&electricalpanel::MODULE, Some(electricalpanel::build_index)),
        (&electricalwire_base::MODULE, Some(electricalwire_base::build_index)),
        (&emergency_light::MODULE, Some(emergency_light::build_index)),
        (&feargun::MODULE, Some(feargun::build_index)),
        (&firealarm::MODULE, Some(firealarm::build_index)),
        (&food::MODULE, Some(food::build_index)),
        (&launch_control::MODULE, Some(launch_control::build_index)),
        (&ligth_l04::MODULE, Some(ligth_l04::build_index)),
        (&ligthening_controller::MODULE, Some(ligthening_controller::build_index)),
        (&moon::MODULE, Some(moon::build_index)),
        (&panel_light::MODULE, Some(panel_light::build_index)),
        (&panellight::MODULE, Some(panellight::build_index)),
        (&pianodialframe::MODULE, Some(pianodialframe::build_index)),
        (&pianolid::MODULE, Some(pianolid::build_index)),
        (&pianopuzzle::MODULE, Some(pianopuzzle::build_index)),
        (&pipes_elbow_local::MODULE, Some(pipes_elbow_local::build_index)),
        (&pipes_long_local::MODULE, Some(pipes_long_local::build_index)),
        (&pipes_mansion::MODULE, Some(pipes_mansion::build_index)),
        (&pipes_mechanical_local::MODULE, Some(pipes_mechanical_local::build_index)),
        (&pipes_pivots_local::MODULE, Some(pipes_pivots_local::build_index)),
        (&puzzlecontroller::MODULE, Some(puzzlecontroller::build_index)),
        (&puzzlepanel::MODULE, Some(puzzlepanel::build_index)),
        (&racecontroller::MODULE, Some(racecontroller::build_index)),
        (&minigamecontroller::MODULE, Some(minigamecontroller::build_index)),
        (&rlight::MODULE, Some(rlight::build_index)),
        (&sarcophageded::MODULE, Some(sarcophageded::build_index)),
        (&shovel::MODULE, Some(shovel::build_index)),
        (&shrubs::MODULE, Some(shrubs::build_index)),
        (&splash_waterhouse::MODULE, Some(splash_waterhouse::build_index)),
        (&statlight::MODULE, Some(statlight::build_index)),
        (&stpanel::MODULE, Some(stpanel::build_index)),
        (&video_cameras::MODULE, Some(video_cameras::build_index)),
        (&video_campivot::MODULE, Some(video_campivot::build_index)),
        (&videocamhead::MODULE, Some(videocamhead::build_index)),
        (&videocampivot::MODULE, Some(videocampivot::build_index)),
        (&visual_on_off_stuff::MODULE, Some(visual_on_off_stuff::build_index)),
        (&water_whell_splash::MODULE, Some(water_whell_splash::build_index)),
        (&level03b_ambient01::MODULE, Some(level03b_ambient01::build_index)),
        (&level03b_desklight01::MODULE, Some(level03b_desklight01::build_index)),
        (&level03b_directional01::MODULE, Some(level03b_directional01::build_index)),
        (&level03b_projectors::MODULE, Some(level03b_projectors::build_index)),
        (&level03b_projectors_ctrl::MODULE, Some(level03b_projectors_ctrl::build_index)),
        (&level03b_window::MODULE, Some(level03b_window::build_index)),
        (&perso::MODULE, Some(perso::build_index)),
        (&loader::MODULE, Some(loader::build_index)),
        (&hs_sound::MODULE, Some(hs_sound::build_index)),
        (&inv_ambiantsound::MODULE, Some(inv_ambiantsound::build_index)),
        (&inv_cutrocket::MODULE, Some(inv_cutrocket::build_index)),
        (&inv_gameplay::MODULE, Some(inv_gameplay::build_index)),
        (&inv_race::MODULE, Some(inv_race::build_index)),
        (&inv_racecontroller::MODULE, Some(inv_racecontroller::build_index)),
    ]
}

/// Finds the ported module + build of a code entry (sha8, then file name, then single-build module name).
pub fn FindPortedModule(e: &LogicCodeEntry) -> Option<(&'static ModuleDef, usize)> {
    let mods = PortedModules();
    let base = e.base_name();
    // builds are keyed by sha8, or by the module's file name when that name is unique
    for key in [e.sha8.as_str(), base.as_str()] {
        for (def, f) in &mods {
            if let Some(b) = f.and_then(|f| f(key)) {
                return Some((def, b));
            }
        }
    }
    mods.iter().find(|(d, f)| f.is_none() && d.name == base).map(|(d, _)| (*d, 0))
}

/// What the level's logic link produced (for diagnostics; the C++ has no such report).
#[derive(Debug, Default)]
pub struct LevelLogicReport {
    pub codes: Vec<LogicCodeEntry>,
    /// code entries with no ported module (their objects get no logic, as GetLogicCode == NULL)
    pub unported: Vec<LogicCodeEntry>,
    pub objects: usize,
    pub latents: usize,
}

/// EFGameLogicMgr+0x240: logic UID -> loaded code.
#[derive(Debug, Default, Clone)]
pub struct LogicCodeMap(pub HashMap<u32, CodeId>);

impl LogicCodeMap {
    /// EFGameLogicMgr::GetLogicCode(uid): NULL when absent.
    pub fn GetLogicCode(&self, uid: u32) -> Option<CodeId> {
        self.0.get(&uid).copied()
    }
}

/// Runs, for a loaded pack, what the C++ does while the pack streams in:
/// 1. each 0x27/0x45 entry: EFLogicCode(Lib)::LoadFile + EFGameLogicMgr::AddLogicCode (pack order;
///    libraries' Register runs at load, inside `load_module`);
/// 2. each EF_GLO of the trailing object container (pack order): logic binding by GLO_FILE_ID, then
///    latent (UID & 0x800000 -> EFGameLogicMgr latent map) or EFWorld::AddGameObject.
/// Returns the handles of the added objects in load order.
pub fn LinkLevelLogic(e: &mut Engine, res: &EFRessourcesMgr, map: &mut LogicCodeMap, report: &mut LevelLogicReport) -> Vec<ObjH> {
    for o in &res.objects {
        let Some(raw) = o.as_raw() else { continue };
        if raw.class_id != 0x27 && raw.class_id != 0x45 {
            continue;
        }
        let Some(entry) = LogicCodeEntry::parse(raw.class_id, &raw.data) else { continue };
        match FindPortedModule(&entry) {
            Some((def, build)) => {
                let c = e.load_module(def, build);
                // AddLogicCode: DTMap::SetAt (a later entry with the same uid replaces it)
                map.0.insert(entry.uid, c);
            }
            None => report.unported.push(entry.clone()),
        }
        report.codes.push(entry);
    }
    let mut out = Vec::new();
    for g in res.game_objects.iter() {
        let code = map.GetLogicCode(g.glo.glo_file_id);
        if g.IsLatent() {
            e.rt.latents.insert(g.uid, code);
            report.latents += 1;
        } else {
            let h = e.add_object(g.uid, code);
            // EFGameObject::LoadFile: +0x15c |= DTGlo OBJECT_FLAGS (0x4000 = logic updated first,
            // which is how library objects construct before their clients)
            if let Some(o) = e.rt.objects[h].as_mut() {
                o.logic_flags |= g.glo.object_flags;
            }
            out.push(h);
            report.objects += 1;
        }
    }
    out
}

