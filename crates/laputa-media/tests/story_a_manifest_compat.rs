use std::fs;

use laputa_domain::MediaKind;
use laputa_media::ProjectStore;
use tempfile::tempdir;

#[test]
fn story_a_manifest_without_understanding_state_remains_openable() {
    let temp = tempdir().expect("temp project");
    let state_dir = temp.path().join(".laputa");
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(
        state_dir.join("project.json"),
        r#"{
  "schema_version": 1,
  "id": "00000000-0000-4000-8000-000000000001",
  "name": "Story A fixture",
  "assets": [
    {
      "id": "00000000-0000-4000-8000-000000000002",
      "original_path": "/story-a/photo.jpg",
      "display_name": "photo.jpg",
      "media_kind": "image",
      "source_sha256": "story-a-revision",
      "byte_len": 42,
      "technical": {
        "width": 1920,
        "height": 1080,
        "duration_ms": null,
        "video_codec": null,
        "has_audio": false
      }
    }
  ]
}"#,
    )
    .expect("write Story A manifest fixture");

    let project = ProjectStore::at(temp.path())
        .open()
        .expect("Story A manifest should remain openable");

    assert_eq!(project.schema_version, 1);
    assert_eq!(project.name, "Story A fixture");
    assert_eq!(project.assets.len(), 1);
    assert_eq!(project.assets[0].media_kind, MediaKind::Image);
    assert_eq!(project.assets[0].source_sha256, "story-a-revision");
}
