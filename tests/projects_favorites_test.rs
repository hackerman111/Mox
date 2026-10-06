use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn test_favorite_projects_are_pinned_to_top() {
    let dir1 = PathBuf::from("/tmp/mox_proj_a");
    let dir2 = PathBuf::from("/tmp/mox_proj_b");
    let favorites: HashSet<PathBuf> = [dir2.clone()].into_iter().collect();

    let mut projects = vec![
        mox::projects::Project {
            path: dir1.clone(),
            session: None,
            is_favorite: false,
        },
        mox::projects::Project {
            path: dir2.clone(),
            session: None,
            is_favorite: false,
        },
    ];
    mox::projects::sort_projects_with_favorites(&mut projects, &favorites);

    assert_eq!(projects[0].path, dir2);
    assert!(projects[0].is_favorite);
    assert_eq!(projects[1].path, dir1);
    assert!(!projects[1].is_favorite);
}

#[test]
fn test_toggle_favorite_roundtrip() {
    let temp_file = std::env::temp_dir().join("mox_test_proj_favs");
    let temp_dir = std::env::temp_dir().join("mox_test_fav_repo");
    let _ = std::fs::remove_file(&temp_file);
    let _ = std::fs::create_dir_all(&temp_dir);

    // Toggle on
    let is_fav = mox::projects::toggle_favorite(&temp_file, &temp_dir).unwrap();
    assert!(is_fav);
    let favs = mox::projects::read_favorites(&temp_file);
    assert!(favs.contains(&std::fs::canonicalize(&temp_dir).unwrap()));

    // Toggle off
    let is_fav_again = mox::projects::toggle_favorite(&temp_file, &temp_dir).unwrap();
    assert!(!is_fav_again);
    let favs_after = mox::projects::read_favorites(&temp_file);
    assert!(!favs_after.contains(&std::fs::canonicalize(&temp_dir).unwrap()));

    let _ = std::fs::remove_file(&temp_file);
    let _ = std::fs::remove_dir_all(&temp_dir);
}
