#![allow(clippy::too_many_lines)]

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
    let h = SpaceHierarchy::default();
    assert!(h.is_orphaned(&id("!room:example.com")));
    assert!(h.children_of(&id("!space:example.com")).is_empty());
    assert!(h.parents_of(&id("!room:example.com")).is_empty());
}

#[test]
fn add_edge_creates_bidirectional_relationship() {
    let h = SpaceHierarchy::default();
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
    let h = SpaceHierarchy::default();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    assert!(h.add_edge(&space, &room));
    assert!(!h.add_edge(&space, &room));
}

#[test]
fn remove_edge() {
    let h = SpaceHierarchy::default();
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
    let h = SpaceHierarchy::default();
    let space = id("!space:example.com");
    let room = id("!room:example.com");

    assert!(!h.remove_edge(&space, &room));
}

// ---------------------------------------------------------------
// Multiple parents
// ---------------------------------------------------------------

#[test]
fn room_with_multiple_parents() {
    let h = SpaceHierarchy::default();
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
// Cycle detection
// ---------------------------------------------------------------

#[test]
fn propagate_delta_handles_cycle() {
    let h = SpaceHierarchy::default();
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
// Notification count cache
// ---------------------------------------------------------------

#[test]
fn recursive_count_cache() {
    let h = SpaceHierarchy::default();
    let space = id("!space:example.com");

    assert_eq!(h.cached_recursive_count(&space), None);

    h.set_recursive_count(&space, 42);
    assert_eq!(h.cached_recursive_count(&space), Some(42));
}

#[test]
fn add_edge_invalidates_ancestor_counts() {
    let h = SpaceHierarchy::default();
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
    let h = SpaceHierarchy::default();
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
    let h = SpaceHierarchy::default();
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
    let h = SpaceHierarchy::default();
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
    let h = SpaceHierarchy::default();
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

    let h = SpaceHierarchy::default();
    let space = id("!space:example.com");

    assert!(h.suggested_rooms(&space, Duration::from_secs(300)).is_none());

    h.set_suggested_rooms(&space, vec![]);
    assert!(h.suggested_rooms(&space, Duration::from_secs(300)).is_some());

    // A zero TTL should always miss.
    assert!(h.suggested_rooms(&space, Duration::ZERO).is_none());
}
