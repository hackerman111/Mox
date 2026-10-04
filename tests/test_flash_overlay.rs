use mox::extract::model::EntityKind;
use mox::flash::overlay::{FlashConfig, FlashMode, build_flash_command};

#[test]
fn test_flash_config_construction() {
    let cfg = FlashConfig::new_jump(false);
    assert_eq!(cfg.mode, FlashMode::Jump);
    assert!(!cfg.multi_pane);

    let cfg_remote = FlashConfig::new_remote_yank(false);
    assert_eq!(cfg_remote.mode, FlashMode::RemoteYank);

    let cfg_quick = FlashConfig::new_quick_yank(None, false);
    assert_eq!(cfg_quick.mode, FlashMode::QuickYank);
}

#[test]
fn test_flash_config_other_modes() {
    let cfg_open = FlashConfig::new_quick_open(Some(EntityKind::Path), true);
    assert_eq!(cfg_open.mode, FlashMode::QuickOpen);
    assert!(cfg_open.multi_pane);
    assert_eq!(cfg_open.entity_filter, Some(EntityKind::Path));

    let cfg_motion = FlashConfig::new_char_motion('x', false, false);
    assert_eq!(cfg_motion.mode, FlashMode::CharMotion);
    assert_eq!(cfg_motion.motion_char, Some('x'));
    assert!(!cfg_motion.motion_forward);
    assert!(!cfg_motion.multi_pane);
}

#[test]
fn test_build_flash_command() {
    let cfg_jump = FlashConfig::new_jump(true);
    let cmd = build_flash_command("/usr/bin/mox", &cfg_jump, Some("test-socket"));
    assert_eq!(
        cmd,
        "'/usr/bin/mox' flash --mode jump --multi-pane --socket 'test-socket'"
    );

    let cfg_filter = FlashConfig::new_quick_yank(Some(EntityKind::Url), false);
    let cmd_filter = build_flash_command("mox", &cfg_filter, None);
    assert_eq!(cmd_filter, "'mox' flash --mode quick-yank --filter url");

    let mut cfg_motion = FlashConfig::new_char_motion('f', false, false);
    cfg_motion.target_pane = Some("%1".to_string());
    let cmd_motion = build_flash_command("mox", &cfg_motion, None);
    assert_eq!(
        cmd_motion,
        "'mox' flash --mode char-motion --char 'f' --backward --target-pane '%1'"
    );
}
