//! WP-N1: menu-side engine behaviour checks.
use scooby_recomp::engine::audio::mgr::EFNewAudioMgr;

/// EFNewAudioMgr_BC::SetVolumeByGroup (0x0023f530) treats the group as a bit mask; the options menu's
/// music volume (miscHUDFUNC_SetMusicVolume, mask 0xa8) sets groups 3, 5 and 7.
#[test]
fn set_volume_by_group_is_a_mask() {
    let mut m = EFNewAudioMgr::new();
    m.SetVolumeByGroup(0xa8, 0.25);
    for i in 0..32 {
        let want = if [3, 5, 7].contains(&i) { 0.25 } else { 1.0 };
        assert_eq!(m.group_volume[i], want, "group {i}");
        assert_eq!(m.GetGroupVolume(1 << i), want);
    }
}
