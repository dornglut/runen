use runen_core_ir::{Projection, StorageInstanceId, StorageRegion};
use runen_exec_oracle::{AccessKind, ordinary_accesses_conflict};

fn region(instance: u64, fields: &[u32]) -> StorageRegion {
    StorageRegion {
        instance: StorageInstanceId(instance),
        projections: fields.iter().copied().map(Projection::Field).collect(),
    }
}

fn conflicts(
    left_kind: AccessKind,
    left: &StorageRegion,
    right_kind: AccessKind,
    right: &StorageRegion,
) -> bool {
    ordinary_accesses_conflict(left_kind, right_kind, left.overlaps(right))
}

#[test]
fn core_structural_overlap_drives_exec_ordinary_conflict() {
    let root = region(1, &[]);
    let same_root = region(1, &[]);
    let left = region(1, &[0]);
    let left_child = region(1, &[0, 1]);
    let right = region(1, &[1]);
    let foreign_left = region(2, &[0]);

    assert!(root.overlaps(&same_root));
    assert!(!conflicts(
        AccessKind::Read,
        &root,
        AccessKind::Read,
        &same_root,
    ));
    assert!(conflicts(
        AccessKind::Read,
        &root,
        AccessKind::StateChange,
        &same_root,
    ));

    assert!(root.overlaps(&left_child));
    assert!(conflicts(
        AccessKind::StateChange,
        &root,
        AccessKind::Read,
        &left_child,
    ));

    assert!(left.overlaps(&left_child));
    assert!(conflicts(
        AccessKind::Read,
        &left,
        AccessKind::StateChange,
        &left_child,
    ));

    assert!(!left.overlaps(&right));
    assert!(!conflicts(
        AccessKind::StateChange,
        &left,
        AccessKind::StateChange,
        &right,
    ));

    assert!(!left.overlaps(&foreign_left));
    assert!(!conflicts(
        AccessKind::StateChange,
        &left,
        AccessKind::StateChange,
        &foreign_left,
    ));
}
