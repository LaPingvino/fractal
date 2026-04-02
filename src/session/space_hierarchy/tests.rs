#![allow(clippy::too_many_lines)]

use std::collections::HashSet;

use ruma::OwnedRoomId;

use super::SpaceHierarchy;

fn id(s: &str) -> OwnedRoomId {
    OwnedRoomId::try_from(s).unwrap()
}

// ---------------------------------------------------------------
// Basic edge operations
// ---------------------------------------------------------------

#[test]
fn empty_hierarchy() {
    let h = SpaceHierarchy::new();
    assert_eq!(h.revision(), 0);
    assert!(h.is_orphaned(&id("!room:example.com")));
    assert!(h.children_of(&id("!space:example.com")).is_empty());
    assert!(h.parents_of(&id("!room:example.com")).is_empty());
}

#[test]
fn add_edge_creates_bidirectional_relationship() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    assert!(h.add_edge(&space, &room));
    assert!(h.is_in_space(&room, &space));
    assert!(!h.is_orphaned(&room));
    assert!(h.children_of(&space).contains(&room));
    assert!(h.parents_of(&room).contains(&space));
}

#[test]
fn add_edge_returns_false_on_duplicate() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    assert!(h.add_edge(&space, &room));
    let rev = h.revision();
    assert!(!h.add_edge(&space, &room));
    // Revision should not bump on duplicate.
    assert_eq!(h.revision(), rev);
}

#[test]
fn remove_edge() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    h.add_edge(&space, &room);
    assert!(h.remove_edge(&space, &room));
    assert!(!h.is_in_space(&room, &space));
    assert!(h.is_orphaned(&room));
    assert!(h.children_of(&space).is_empty());
}

#[test]
fn remove_nonexistent_edge_returns_false() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    let rev = h.revision();
    assert!(!h.remove_edge(&space, &room));
    assert_eq!(h.revision(), rev);
}

// ---------------------------------------------------------------
// Multiple parents
// ---------------------------------------------------------------

#[test]
fn room_with_multiple_parents() {
    let h = SpaceHierarchy::new();
    let space_a = id("!a:example.com");
    let space_b = id("!b:example.com");
    let room = id("!room:example.com");

    h.add_edge(&space_a, &room);
    h.add_edge(&space_b, &room);

    assert!(h.is_in_space(&room, &space_a));
    assert!(h.is_in_space(&room, &space_b));
    assert!(!h.is_orphaned(&room));

    let parents = h.parents_of(&room);
    assert_eq!(parents.len(), 2);
    assert!(parents.contains(&space_a));
    assert!(parents.contains(&space_b));
}

// ---------------------------------------------------------------
// Batch operations
// ---------------------------------------------------------------

#[test]
fn set_children_replaces_previous() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room_a = id("!a:example.com");
    let room_b = id("!b:example.com");
    let room_c = id("!c:example.com");

    // Initial children.
    h.set_children(&space, HashSet::from([room_a.clone(), room_b.clone()]));
    assert!(h.is_in_space(&room_a, &space));
    assert!(h.is_in_space(&room_b, &space));
    assert!(!h.is_in_space(&room_c, &space));

    // Replace: room_a removed, room_c added, room_b stays.
    h.set_children(&space, HashSet::from([room_b.clone(), room_c.clone()]));
    assert!(!h.is_in_space(&room_a, &space));
    assert!(h.is_in_space(&room_b, &space));
    assert!(h.is_in_space(&room_c, &space));

    // room_a should be orphaned now (no other parents).
    assert!(h.is_orphaned(&room_a));
}

#[test]
fn set_children_updates_reverse_edges() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    h.set_children(&space, HashSet::from([room.clone()]));
    assert!(h.parents_of(&room).contains(&space));

    // Replace with empty set.
    h.set_children(&space, HashSet::new());
    assert!(h.is_orphaned(&room));
}

#[test]
fn set_children_single_revision_bump() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let rooms: HashSet<_> = (0..100)
        .map(|i| id(&format!("!room{i}:example.com")))
        .collect();

    let rev = h.revision();
    h.set_children(&space, rooms);
    // Only one revision bump for the whole batch.
    assert_eq!(h.revision(), rev + 1);
}

// ---------------------------------------------------------------
// Remove room
// ---------------------------------------------------------------

#[test]
fn remove_room_cleans_all_edges() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");
    let nested_space = id("!nested:example.com");

    // room is child of space, nested_space is child of room.
    h.add_edge(&space, &room);
    h.add_edge(&room, &nested_space);

    h.remove_room(&room);

    assert!(!h.is_in_space(&room, &space));
    assert!(h.children_of(&space).is_empty());
    assert!(h.is_orphaned(&nested_space));
    assert!(h.children_of(&room).is_empty());
    assert!(h.parents_of(&room).is_empty());
}

// ---------------------------------------------------------------
// Cycle detection
// ---------------------------------------------------------------

#[test]
fn transitive_children_handles_cycle() {
    let h = SpaceHierarchy::new();
    let space_a = id("!a:example.com");
    let space_b = id("!b:example.com");
    let room = id("!room:example.com");

    // A -> B -> A (cycle), B -> room
    h.add_edge(&space_a, &space_b);
    h.add_edge(&space_b, &space_a);
    h.add_edge(&space_b, &room);

    let children = h.transitive_children_of(&space_a);
    assert!(children.contains(&space_b));
    assert!(children.contains(&room));
    // space_a itself should not be in its own transitive children,
    // but the cycle shouldn't cause a hang.
    assert!(children.contains(&space_a));
}

#[test]
fn propagate_delta_handles_cycle() {
    let h = SpaceHierarchy::new();
    let space_a = id("!a:example.com");
    let space_b = id("!b:example.com");

    // Cycle: A -> B -> A
    h.add_edge(&space_a, &space_b);
    h.add_edge(&space_b, &space_a);

    // Seed counts.
    h.set_recursive_count(&space_a, 5);
    h.set_recursive_count(&space_b, 3);

    // Propagate delta from a child of A — should not infinite loop.
    let changed = h.propagate_delta(&space_a, 2);
    // space_b is a parent of space_a, so it should be affected.
    assert!(changed.contains(&space_b));
}

// ---------------------------------------------------------------
// Transitive children
// ---------------------------------------------------------------

#[test]
fn transitive_children_deep_nesting() {
    let h = SpaceHierarchy::new();
    let s1 = id("!s1:example.com");
    let s2 = id("!s2:example.com");
    let s3 = id("!s3:example.com");
    let room = id("!room:example.com");

    // s1 -> s2 -> s3 -> room
    h.add_edge(&s1, &s2);
    h.add_edge(&s2, &s3);
    h.add_edge(&s3, &room);

    let children = h.transitive_children_of(&s1);
    assert!(children.contains(&s2));
    assert!(children.contains(&s3));
    assert!(children.contains(&room));
    assert_eq!(children.len(), 3);
}

#[test]
fn transitive_children_empty_space() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    assert!(h.transitive_children_of(&space).is_empty());
}

// ---------------------------------------------------------------
// Notification count cache
// ---------------------------------------------------------------

#[test]
fn recursive_count_cache() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");

    assert_eq!(h.cached_recursive_count(&space), None);

    h.set_recursive_count(&space, 42);
    assert_eq!(h.cached_recursive_count(&space), Some(42));
}

#[test]
fn add_edge_invalidates_ancestor_counts() {
    let h = SpaceHierarchy::new();
    let s1 = id("!s1:example.com");
    let s2 = id("!s2:example.com");
    let room = id("!room:example.com");

    // s1 -> s2, seed counts.
    h.add_edge(&s1, &s2);
    h.set_recursive_count(&s1, 10);
    h.set_recursive_count(&s2, 5);

    // Adding a child to s2 should invalidate s2 and s1.
    h.add_edge(&s2, &room);
    assert_eq!(h.cached_recursive_count(&s2), None);
    assert_eq!(h.cached_recursive_count(&s1), None);
}

#[test]
fn propagate_delta_updates_counts() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    h.add_edge(&space, &room);
    h.set_recursive_count(&space, 10);

    let changed = h.propagate_delta(&room, 3);
    assert!(changed.contains(&space));
    assert_eq!(h.cached_recursive_count(&space), Some(13));
}

#[test]
fn propagate_delta_clamps_to_zero() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    h.add_edge(&space, &room);
    h.set_recursive_count(&space, 2);

    let changed = h.propagate_delta(&room, -5);
    assert!(changed.contains(&space));
    assert_eq!(h.cached_recursive_count(&space), Some(0));
}

#[test]
fn propagate_delta_zero_is_noop() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    h.add_edge(&space, &room);
    h.set_recursive_count(&space, 10);

    let changed = h.propagate_delta(&room, 0);
    assert!(changed.is_empty());
    assert_eq!(h.cached_recursive_count(&space), Some(10));
}

#[test]
fn propagate_delta_skips_spaces_without_cached_count() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    h.add_edge(&space, &room);
    // Don't set a cached count for space.

    let changed = h.propagate_delta(&room, 5);
    // Space has no cached count, so it shouldn't appear in changed.
    assert!(changed.is_empty());
}

// ---------------------------------------------------------------
// Suggested rooms cache
// ---------------------------------------------------------------

#[test]
fn suggested_rooms_cache_with_ttl() {
    use std::time::Duration;

    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");

    assert!(h.suggested_rooms(&space, Duration::from_secs(300)).is_none());

    h.set_suggested_rooms(&space, vec![]);
    assert!(h.suggested_rooms(&space, Duration::from_secs(300)).is_some());

    // A zero TTL should always miss.
    assert!(h.suggested_rooms(&space, Duration::ZERO).is_none());
}

// ---------------------------------------------------------------
// Revision counter
// ---------------------------------------------------------------

#[test]
fn revision_bumps_on_structural_changes() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    assert_eq!(h.revision(), 0);

    h.add_edge(&space, &room);
    assert_eq!(h.revision(), 1);

    h.remove_edge(&space, &room);
    assert_eq!(h.revision(), 2);

    h.set_children(&space, HashSet::from([room.clone()]));
    assert_eq!(h.revision(), 3);

    h.remove_room(&room);
    assert_eq!(h.revision(), 4);
}

#[test]
fn revision_does_not_bump_on_count_updates() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");

    let rev = h.revision();
    h.set_recursive_count(&space, 42);
    assert_eq!(h.revision(), rev);
}

// ---------------------------------------------------------------
// Orphaned rooms helper
// ---------------------------------------------------------------

#[test]
fn orphaned_rooms() {
    let h = SpaceHierarchy::new();
    let space = id("!space:example.com");
    let room_a = id("!a:example.com");
    let room_b = id("!b:example.com");

    h.add_edge(&space, &room_a);

    let all = vec![space.clone(), room_a.clone(), room_b.clone()];
    let orphans = h.orphaned_rooms(all.iter());

    // space itself has no parent, so it's orphaned.
    // room_b has no parent.
    // room_a has a parent (space).
    assert!(orphans.contains(&space));
    assert!(orphans.contains(&room_b));
    assert!(!orphans.contains(&room_a));
    assert_eq!(orphans.len(), 2);
}
