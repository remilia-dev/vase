// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::collections::{
    HashMap,
    VecDeque,
};

use crate::{
    c::traveler::{
        Frame,
        MacroKind,
    },
    util::{
        CachedString,
        FileId,
    },
};

/// A snapshot of [Traveler](super::Traveler)'s progress in a token stack.
///
/// It can be loaded at any point to bring the traveler back to the save point.
/// However, loading a state from a different traveler (or a re-used traveler) may
/// inevitably cause panics.
#[derive(Clone, Debug)]
pub struct TravelerState {
    pub(super) frames: VecDeque<Frame>,
    pub(super) macros: HashMap<CachedString, MacroKind>,
    pub(super) dependencies: Vec<FileId>,
    pub(super) index: u32,
    pub(super) should_chain_skip: bool,
}
