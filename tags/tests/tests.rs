use tags::Tags;

#[test]
fn test_empty_file() {
    assert!(Tags::try_from(&[]).unwrap().tags().is_empty());
}

#[test]
fn test_file_without_tags() {
    let content = include_bytes!("assets/no_tags.mp3");
    assert!(Tags::try_from(content).unwrap().tags().is_empty());
}

#[test]
fn test_file_with_tags() {
    let content = include_bytes!("assets/tags.mp3");
    let tags = Tags::try_from(content).unwrap();

    assert_eq!(tags.track_title(), Some("Yet Another Track"));
    assert_eq!(tags.track_artist(), Some("Unknown Artist"));
    assert_eq!(tags.track_number(), Some("7"));
    assert_eq!(tags.genre(), Some("Gangsta Rap"));
    assert_eq!(tags.get("RecordingDate"), Some("2022"));
    assert_eq!(tags.album_title(), Some("Collection of Nothing"));
    assert_eq!(tags.comment(), Some("Should be some meaningful comment"));
}

#[test]
fn test_file_with_custom_tags() {
    let content = include_bytes!("assets/custom_tags.mp3");
    let tags = Tags::try_from(content).unwrap();

    assert_eq!(tags.get("HoursSpentRefactoring"), Some("Inf"));
    assert_eq!(tags.get("CustomerApproval"), Some("95"));
}
