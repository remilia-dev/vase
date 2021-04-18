// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use smallvec::SmallVec;

use crate::{
    c::{
        ast::{
            Decl,
            DeclIndex,
            Expr,
            ScopeId,
            SourceFile,
            Storage,
            StorageKind,
        },
        Keyword,
        TravelIndex,
        TravelRange,
    },
    math::NonMaxU32,
    util::{
        create_intos,
        CachedString,
        Conversions,
        RedeclMap,
        RedeclMapIndex,
        Vec32,
    },
};

#[derive(Clone, Debug)]
pub struct Type {
    pub root: TypeRoot,
    pub root_index: Option<TravelIndex>,
    pub name: Option<CachedString>,
    pub storage: Storage,
    pub segments: Vec32<TypeSegment>,
    pub base_segments: NonMaxU32,
}

impl Type {
    pub fn new(default: StorageKind) -> Self {
        Type {
            root: TypeRoot::AutoInt,
            root_index: None,
            name: None,
            storage: Storage::new(default),
            segments: Vec32::new(),
            base_segments: 0.into(),
        }
    }

    pub fn new_enum(name: CachedString) -> Self {
        Type {
            root: TypeRoot::EnumValue,
            root_index: None,
            name: Some(name),
            storage: Storage::new(StorageKind::Declared),
            segments: Vec32::new(),
            base_segments: 0.into(),
        }
    }

    pub fn is_implicit(&self) -> bool {
        matches!(self.root, TypeRoot::AutoInt)
    }

    pub fn add_modifier(&mut self, modifier: Keyword, index: TravelIndex) {
        self.segments.push(ModifierSegment::new(modifier, index).into())
    }

    pub fn clone_base(&self) -> Self {
        let mut segments = self.segments.clone();
        segments.truncate(self.base_segments);
        Type {
            root: self.root.clone(),
            root_index: self.root_index,
            name: None,
            storage: self.storage.clone(),
            segments,
            base_segments: self.base_segments,
        }
    }

    pub fn try_set_base_type(&mut self, base: Keyword, index: TravelIndex) -> bool {
        if !matches!(self.root, TypeRoot::AutoInt) {
            return false;
        }

        if let Ok(base) = base.try_into() {
            self.root_index = Some(index);
            self.root = base;
            true
        } else {
            false
        }
    }

    pub fn get_func_scope_id(&self) -> Option<ScopeId> {
        if let TypeSegment::Func(ref func) = *self.segments.last()? {
            Some(func.scope_id)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeRoot {
    AutoInt,
    Bool,
    Char,
    Int,
    Float,
    Double,
    Void,
    Decimal32,
    Decimal64,
    Decimal128,
    Type(DeclIndex),
    Typedef(DeclIndex),
    /// Represents that the type is part of an enum.
    /// This type should only show up in Type.field_decls.
    EnumValue,
    /// Represents an enum literal that has been inlined into the scope.
    EnumForward(DeclIndex),
}

impl std::convert::TryFrom<Keyword> for TypeRoot {
    type Error = ();

    fn try_from(value: Keyword) -> Result<Self, Self::Error> {
        match value {
            Keyword::Bool => Ok(TypeRoot::Bool),
            Keyword::Char => Ok(TypeRoot::Char),
            Keyword::Int => Ok(TypeRoot::Int),
            Keyword::Float => Ok(TypeRoot::Float),
            Keyword::Double => Ok(TypeRoot::Double),
            Keyword::Void => Ok(TypeRoot::Void),
            Keyword::Decimal32 => Ok(TypeRoot::Decimal32),
            Keyword::Decimal64 => Ok(TypeRoot::Decimal64),
            Keyword::Decimal128 => Ok(TypeRoot::Decimal128),
            keyword if keyword.is_base_type() => unimplemented!(),
            _ => Err(()),
        }
    }
}

#[create_intos]
#[derive(Clone, Debug)]
pub enum TypeSegment {
    Pointer(PointerSegment),
    Array(ArraySegment),
    Func(FuncSegment),
    Modifier(ModifierSegment),
}

#[derive(Clone, Debug)]
pub enum ModifierSegment {
    Const(TravelIndex),
    Inline(TravelIndex),
    Long(TravelIndex),
    Short(TravelIndex),
    Signed(TravelIndex),
    Unsigned(TravelIndex),
    Volatile(TravelIndex),
    Alignas(Box<Expr>),
    Atomic(TravelIndex),
    Complex(TravelIndex),
    Imaginary(TravelIndex),
    NoReturn(TravelIndex),
    ThreadLocal(TravelIndex),
}
impl ModifierSegment {
    pub fn new(keyword: Keyword, index: TravelIndex) -> ModifierSegment {
        match keyword {
            Keyword::Const => ModifierSegment::Const(index),
            Keyword::Inline => ModifierSegment::Inline(index),
            Keyword::Long => ModifierSegment::Long(index),
            Keyword::Short => ModifierSegment::Short(index),
            Keyword::Signed => ModifierSegment::Signed(index),
            Keyword::Unsigned => ModifierSegment::Unsigned(index),
            Keyword::Volatile => ModifierSegment::Volatile(index),
            Keyword::Complex => ModifierSegment::Complex(index),
            Keyword::Imaginary => ModifierSegment::Imaginary(index),
            Keyword::Noreturn => ModifierSegment::NoReturn(index),
            Keyword::ThreadLocal => ModifierSegment::ThreadLocal(index),
            keyword if keyword.is_type_modifier() => unimplemented!(),
            _ => panic!("Only type modifier keywords should be passed to add_modifier"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PointerSegment(pub TravelIndex);

#[derive(Clone, Debug)]
pub struct ArraySegment {
    pub range: TravelRange,
    pub const_: Option<TravelIndex>,
    pub restrict: Option<TravelIndex>,
    pub static_: Option<TravelIndex>,
    pub kind: ArrayKind,
}

#[create_intos]
#[derive(Clone, Debug)]
pub enum ArrayKind {
    Empty,
    Expr(Box<Expr>),
    Star(TravelIndex),
}

#[derive(Clone, Debug)]
pub struct FuncSegment {
    pub range: TravelRange,
    pub scope_id: ScopeId,
    pub vararg_index: Option<TravelIndex>,
}

impl FuncSegment {
    pub fn has_vararg(&self) -> bool {
        self.vararg_index.is_some()
    }
}

#[derive(Clone, Debug)]
pub struct TypeDeclTag {
    pub range: TravelRange,
    pub kind: TypeDeclKind,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TypeDeclKind {
    Enum,
    Struct,
    Union,
}

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub name: Option<CachedString>,
    pub tags: Vec<TypeDeclTag>,
    pub body: Option<TypeDeclBody>,
}

impl TypeDecl {
    pub fn new(tag: TypeDeclTag, name: Option<CachedString>) -> Self {
        Self { name, tags: vec![tag], body: None }
    }

    pub fn incomplete(&self) -> bool {
        self.body.is_none()
    }
}

#[derive(Clone, Debug)]
pub struct TypeDeclBody {
    pub range: TravelRange,
    pub kind: TypeDeclKind,
    pub fields: RedeclMap<CachedString, TypeDeclField>,
}

impl TypeDeclBody {
    pub fn new(kind: TypeDeclKind) -> Self {
        Self {
            range: 0.into()..0.into(),
            kind,
            fields: RedeclMap::new(),
        }
    }

    pub fn add_decls(&mut self, file: &SourceFile, decls: SmallVec<[Decl; 1]>) {
        for decl in decls.into_iter() {
            let decl_name = decl.type_.name.clone();
            match decl.type_.root {
                TypeRoot::Type(inner_index) if decl.type_.name.is_none() => {
                    let indirect_index = self.fields.add(decl_name, decl.into());
                    let inner_type = file.get_type_decl(inner_index);
                    if let Some(ref body) = inner_type.body {
                        self.add_forwards(body, indirect_index);
                    }
                },
                _ => {
                    self.fields.add(decl_name, decl.into());
                },
            }
        }
    }

    fn add_forwards(&mut self, inner: &TypeDeclBody, to: RedeclMapIndex) {
        if inner.kind == TypeDeclKind::Enum {
            return;
        }

        for field in inner.fields.keys() {
            self.fields.add_keyed(field.clone(), to.into());
        }
    }
}

#[create_intos]
#[derive(Clone, Debug)]
pub enum TypeDeclField {
    Direct(Decl),
    Indirect(RedeclMapIndex),
}
