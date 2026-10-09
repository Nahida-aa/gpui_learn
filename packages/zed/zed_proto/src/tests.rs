use super::*;

#[test]
fn test_converting_peer_id_from_and_to_u64() {
    let peer_id = PeerId {
        owner_id: 10,
        id: 3,
    };
    assert_eq!(PeerId::from_u64(peer_id.as_u64()), peer_id);
    let peer_id = PeerId {
        owner_id: u32::MAX,
        id: 3,
    };
    assert_eq!(PeerId::from_u64(peer_id.as_u64()), peer_id);
    let peer_id = PeerId {
        owner_id: 10,
        id: u32::MAX,
    };
    assert_eq!(PeerId::from_u64(peer_id.as_u64()), peer_id);
    let peer_id = PeerId {
        owner_id: u32::MAX,
        id: u32::MAX,
    };
    assert_eq!(PeerId::from_u64(peer_id.as_u64()), peer_id);
}

#[test]
fn test_split_repository_update_keeps_branch_list_on_final_chunk() {
    let update = UpdateRepository {
        updated_statuses: vec![
            StatusEntry::default(),
            StatusEntry::default(),
            StatusEntry::default(),
        ],
        branch_list: vec![Branch {
            ref_name: "refs/heads/main".into(),
            ..Default::default()
        }],
        branch_list_error: Some("partial branch scan".into()),
        ..Default::default()
    };

    let chunks = split_repository_update(update).collect::<Vec<_>>();

    assert_eq!(chunks.len(), 3);
    assert!(chunks[0].branch_list.is_empty());
    assert!(chunks[1].branch_list.is_empty());
    assert_eq!(chunks[2].branch_list.len(), 1);
    assert_eq!(chunks[2].branch_list[0].ref_name, "refs/heads/main");
    assert_eq!(chunks[0].branch_list_error, None);
    assert_eq!(chunks[1].branch_list_error, None);
    assert_eq!(
        chunks[2].branch_list_error.as_deref(),
        Some("partial branch scan")
    );
    assert!(!chunks[0].is_last_update);
    assert!(!chunks[1].is_last_update);
    assert!(chunks[2].is_last_update);
}
