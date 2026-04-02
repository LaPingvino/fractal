use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use ruma::{
    api::client::space::SpaceHierarchyRoomsChunk,
    OwnedRoomId, RoomId,
};

#[cfg(test)]
mod tests;

/// Cached suggested rooms for a space.
#[derive(Debug)]
struct SuggestedCache {
    rooms: Vec<SpaceHierarchyRoomsChunk>,
    fetched_at: Instant,
}

/// Internal graph data.
#[derive(Debug, Default)]
struct SpaceGraph {
    /// space_id -> set of child room/space IDs.
    children: HashMap<OwnedRoomId, HashSet<OwnedRoomId>>,
    /// room_id -> set of parent space IDs.
    parents: HashMap<OwnedRoomId, HashSet<OwnedRoomId>>,
    /// Cached recursive notification counts per space.
    recursive_counts: HashMap<OwnedRoomId, u64>,
    /// Suggested rooms per space, with fetch timestamp.
    suggested: HashMap<OwnedRoomId, SuggestedCache>,
}

/// Central, authoritative store for the entire space hierarchy.
///
/// All parent-child edges live here. `Room` GObjects delegate to this
/// structure rather than maintaining their own `RefCell<HashSet>`.
///
/// Lives on the main thread (GTK is single-threaded), so a plain
/// `RefCell` suffices for interior mutability.
#[derive(Debug, Default)]
pub(crate) struct SpaceHierarchy {
    inner: RefCell<SpaceGraph>,
    /// Monotonically increasing revision counter, bumped on every
    /// structural change. Consumers (filters, UI) can compare their
    /// last-seen revision to decide whether to re-query.
    revision: Cell<u64>,
    /// Whether a filter re-evaluation is already scheduled.
    /// Used to coalesce multiple relationship loads into one notification.
    filter_notify_pending: Cell<bool>,
}

impl SpaceHierarchy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current revision number.
    pub fn revision(&self) -> u64 {
        self.revision.get()
    }

    /// Whether a filter notification is pending.
    pub fn filter_notify_pending(&self) -> bool {
        self.filter_notify_pending.get()
    }

    /// Set the filter notification pending flag.
    pub fn set_filter_notify_pending(&self, pending: bool) {
        self.filter_notify_pending.set(pending);
    }

    // ---------------------------------------------------------------
    // Structural mutations (all bump revision)
    // ---------------------------------------------------------------

    /// Add a parent-child edge. Returns `true` if the edge was new.
    pub fn add_edge(&self, parent: &RoomId, child: &RoomId) -> bool {
        let mut g = self.inner.borrow_mut();
        let p = g.children.entry(parent.to_owned()).or_default().insert(child.to_owned());
        let c = g.parents.entry(child.to_owned()).or_default().insert(parent.to_owned());
        let changed = p || c;
        if changed {
            Self::invalidate_counts_up(&mut g, parent);
            drop(g);
            self.bump_revision();
        }
        changed
    }

    /// Remove a parent-child edge. Returns `true` if it existed.
    pub fn remove_edge(&self, parent: &RoomId, child: &RoomId) -> bool {
        let mut g = self.inner.borrow_mut();
        let mut changed = false;
        if let Some(set) = g.children.get_mut(parent) {
            changed |= set.remove(child);
            if set.is_empty() {
                g.children.remove(parent);
            }
        }
        if let Some(set) = g.parents.get_mut(child) {
            changed |= set.remove(parent);
            if set.is_empty() {
                g.parents.remove(child);
            }
        }
        if changed {
            Self::invalidate_counts_up(&mut g, parent);
            drop(g);
            self.bump_revision();
        }
        changed
    }

    /// Batch-set all children for a space, replacing any previous set.
    /// More efficient than N individual `add_edge` calls.
    pub fn set_children(&self, space_id: &RoomId, children: HashSet<OwnedRoomId>) {
        let mut g = self.inner.borrow_mut();

        // Remove old reverse edges.
        let old_children: Vec<_> = g
            .children
            .get(space_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        for old_child in &old_children {
            if let Some(parents) = g.parents.get_mut(old_child) {
                parents.remove(space_id);
            }
        }

        // Insert new reverse edges.
        for child in &children {
            g.parents.entry(child.clone()).or_default().insert(space_id.to_owned());
        }

        g.children.insert(space_id.to_owned(), children);
        Self::invalidate_counts_up(&mut g, space_id);
        drop(g);
        self.bump_revision();
    }

    /// Remove all edges involving a room (e.g. when it is forgotten).
    pub fn remove_room(&self, room_id: &RoomId) {
        let mut g = self.inner.borrow_mut();
        let mut changed = false;

        if let Some(parent_set) = g.parents.remove(room_id) {
            for parent in &parent_set {
                if let Some(children) = g.children.get_mut(parent) {
                    children.remove(room_id);
                }
            }
            changed = true;
        }

        if let Some(child_set) = g.children.remove(room_id) {
            for child in &child_set {
                if let Some(parents) = g.parents.get_mut(child) {
                    parents.remove(room_id);
                }
            }
            changed = true;
        }

        g.recursive_counts.remove(room_id);
        g.suggested.remove(room_id);

        if changed {
            drop(g);
            self.bump_revision();
        }
    }

    // ---------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------

    /// Is `room_id` a direct child of `space_id`?
    pub fn is_in_space(&self, room_id: &RoomId, space_id: &RoomId) -> bool {
        self.inner
            .borrow()
            .children
            .get(space_id)
            .is_some_and(|set| set.contains(room_id))
    }

    /// Does this room have zero parents?
    pub fn is_orphaned(&self, room_id: &RoomId) -> bool {
        self.inner
            .borrow()
            .parents
            .get(room_id)
            .map_or(true, |set| set.is_empty())
    }

    /// Direct children of a space.
    pub fn children_of(&self, space_id: &RoomId) -> HashSet<OwnedRoomId> {
        self.inner
            .borrow()
            .children
            .get(space_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Direct parents of a room.
    pub fn parents_of(&self, room_id: &RoomId) -> HashSet<OwnedRoomId> {
        self.inner
            .borrow()
            .parents
            .get(room_id)
            .cloned()
            .unwrap_or_default()
    }

    // ---------------------------------------------------------------
    // Notification counts
    // ---------------------------------------------------------------

    /// Get the cached recursive notification count, or `None` if stale.
    pub fn cached_recursive_count(&self, space_id: &RoomId) -> Option<u64> {
        self.inner.borrow().recursive_counts.get(space_id).copied()
    }

    /// Store a freshly computed recursive count.
    pub fn set_recursive_count(&self, space_id: &RoomId, count: u64) {
        self.inner
            .borrow_mut()
            .recursive_counts
            .insert(space_id.to_owned(), count);
    }

    /// Apply an additive delta to cached counts, propagating up through
    /// ancestors. Returns the IDs of spaces whose counts changed.
    pub fn propagate_delta(&self, room_id: &RoomId, delta: i64) -> Vec<OwnedRoomId> {
        if delta == 0 {
            return Vec::new();
        }

        let mut changed = Vec::new();
        let mut g = self.inner.borrow_mut();
        let mut visited = HashSet::new();
        let mut stack: Vec<OwnedRoomId> = g
            .parents
            .get(room_id)
            .into_iter()
            .flat_map(|s| s.iter().cloned())
            .collect();

        while let Some(ancestor) = stack.pop() {
            if !visited.insert(ancestor.clone()) {
                continue;
            }
            if let Some(count) = g.recursive_counts.get_mut(&ancestor) {
                *count = (*count as i64 + delta).max(0) as u64;
                changed.push(ancestor.clone());
            }
            if let Some(grandparents) = g.parents.get(&*ancestor) {
                stack.extend(grandparents.iter().cloned());
            }
        }

        changed
    }

    // ---------------------------------------------------------------
    // Suggested rooms cache
    // ---------------------------------------------------------------

    /// Get cached suggested rooms if younger than `max_age`.
    pub fn suggested_rooms(
        &self,
        space_id: &RoomId,
        max_age: Duration,
    ) -> Option<Vec<SpaceHierarchyRoomsChunk>> {
        let g = self.inner.borrow();
        g.suggested.get(space_id).and_then(|cache| {
            if cache.fetched_at.elapsed() < max_age {
                Some(cache.rooms.clone())
            } else {
                None
            }
        })
    }

    /// Store fetched suggested rooms.
    pub fn set_suggested_rooms(
        &self,
        space_id: &RoomId,
        rooms: Vec<SpaceHierarchyRoomsChunk>,
    ) {
        self.inner.borrow_mut().suggested.insert(
            space_id.to_owned(),
            SuggestedCache {
                rooms,
                fetched_at: Instant::now(),
            },
        );
    }

    // ---------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------

    fn bump_revision(&self) {
        self.revision.set(self.revision.get() + 1);
    }

    /// Invalidate cached recursive counts for `start` and all ancestors.
    fn invalidate_counts_up(g: &mut SpaceGraph, start: &RoomId) {
        let mut stack = vec![start.to_owned()];
        let mut visited = HashSet::new();
        while let Some(id) = stack.pop() {
            if !visited.insert(id.clone()) {
                continue;
            }
            g.recursive_counts.remove(&id);
            if let Some(parents) = g.parents.get(&*id) {
                stack.extend(parents.iter().cloned());
            }
        }
    }
}
