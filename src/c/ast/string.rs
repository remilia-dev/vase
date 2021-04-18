// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use smallvec::SmallVec;

use crate::{
    c::{
        StringEnc,
        TravelRange,
    },
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct StringLiteral {
    pub range: TravelRange,
    pub segments: SmallVec<[Arc<Box<str>>; 1]>,
    pub encoding: StringEnc,
    pub has_escapes: bool,
}
