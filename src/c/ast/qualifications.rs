// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        Keyword,
        TravelIndex,
    },
    util::Conversions,
};

#[derive(Clone, Debug)]
pub struct Storage {
    pub kind_index: Option<TravelIndex>,
    pub kind: StorageKind,
}

impl Storage {
    pub fn new(default: StorageKind) -> Self {
        Self { kind_index: None, kind: default }
    }

    pub fn is_implicit(&self) -> bool {
        self.kind_index.is_none()
    }

    pub fn try_set(&mut self, keyword: Keyword, index: TravelIndex) -> bool {
        if self.kind_index.is_some() {
            false
        } else if let Ok(kind) = keyword.try_into() {
            self.kind_index = Some(index);
            self.kind = kind;
            true
        } else {
            false
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StorageKind {
    Declared,
    Auto,
    Static,
    Extern,
    Register,
    Typedef,
}

impl std::convert::TryFrom<Keyword> for StorageKind {
    type Error = ();

    fn try_from(value: Keyword) -> Result<Self, Self::Error> {
        match value {
            Keyword::Auto => Ok(StorageKind::Auto),
            Keyword::Static => Ok(StorageKind::Static),
            Keyword::Extern => Ok(StorageKind::Typedef),
            Keyword::Register => Ok(StorageKind::Register),
            Keyword::Typedef => Ok(StorageKind::Typedef),
            keyword if keyword.is_storage_class() => unimplemented!(),
            _ => Err(()),
        }
    }
}
