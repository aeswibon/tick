use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::api::types::Ticket;

/// Read tickets; recovers from poisoned lock instead of panicking.
pub fn read_tickets<'a>(lock: &'a RwLock<Vec<Ticket>>) -> RwLockReadGuard<'a, Vec<Ticket>> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

pub fn write_tickets<'a>(lock: &'a RwLock<Vec<Ticket>>) -> RwLockWriteGuard<'a, Vec<Ticket>> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}
