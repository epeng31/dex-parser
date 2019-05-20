use std::clone::Clone;

use scroll::{Pread, Uleb128};

use crate::cache::Ref;
use crate::encoded_item::EncodedItemArrayCtx;
use crate::error::Error;
use crate::field::EncodedFieldArray;
use crate::field::Field;
use crate::jtype::Type;
use crate::method::EncodedMethodArray;
use crate::method::Method;
use crate::source::Source;
use crate::string::JString;
use crate::uint;
use scroll::ctx;

pub type ClassId = uint;
// TODO: define an enum for this
pub type AccessFlags = uint;

#[allow(unused)]
#[derive(Debug)]
pub struct Class {
    pub(crate) id: ClassId,
    pub(crate) access_flags: AccessFlags,
    pub(crate) super_class: ClassId,
    pub(crate) interfaces: Option<Vec<Type>>,
    pub(crate) jtype: Type,
    pub(crate) source_file: Option<Ref<JString>>,
    pub(crate) static_fields: Option<Vec<Field>>,
    pub(crate) instance_fields: Option<Vec<Field>>,
    pub(crate) direct_methods: Option<Vec<Method>>,
    pub(crate) virtual_methods: Option<Vec<Method>>,
}

impl Class {
    pub(crate) fn try_from_dex<T: AsRef<[u8]>>(
        dex: &super::Dex<T>,
        class_def: &ClassDefItem,
    ) -> super::Result<Self> {
        let data_off = class_def.class_data_off;

        let (static_fields, instance_fields, direct_methods, virtual_methods) =
            match dex.get_class_data(data_off)? {
                Some(c) => {
                    let ec = |encoded_field| dex.get_field(&encoded_field);
                    let ef = |encoded_method| dex.get_method(&encoded_method);
                    (
                        try_into_item!(c.static_fields, ec),
                        try_into_item!(c.instance_fields, ec),
                        try_into_item!(c.direct_methods, ef),
                        try_into_item!(c.virtual_methods, ef),
                    )
                }
                None => (None, None, None, None),
            };
        Ok(Class {
            id: class_def.class_idx,
            jtype: dex.get_type(class_def.class_idx)?,
            super_class: class_def.superclass_idx,
            interfaces: dex.get_interfaces(class_def.interfaces_off)?,
            access_flags: class_def.access_flags,
            source_file: dex.get_source_file(class_def.source_file_idx)?,
            static_fields,
            instance_fields,
            direct_methods,
            virtual_methods,
        })
    }

    pub fn get_type(&self) -> Type {
        self.jtype.clone()
    }
}

pub(crate) struct ClassDataItem {
    static_fields: Option<EncodedFieldArray>,
    instance_fields: Option<EncodedFieldArray>,
    direct_methods: Option<EncodedMethodArray>,
    virtual_methods: Option<EncodedMethodArray>,
}

impl<'a, S> ctx::TryFromCtx<'a, &super::Dex<S>> for ClassDataItem
where
    S: AsRef<[u8]>,
{
    type Error = crate::error::Error;
    type Size = usize;

    fn try_from_ctx(source: &'a [u8], dex: &super::Dex<S>) -> super::Result<(Self, Self::Size)> {
        let offset = &mut 0;
        let static_field_size = Uleb128::read(source, offset)?;
        let instance_field_size = Uleb128::read(source, offset)?;
        let direct_methods_size = Uleb128::read(source, offset)?;
        let virtual_methods_size = Uleb128::read(source, offset)?;

        Ok((
            ClassDataItem {
                static_fields: encoded_array!(source, dex, offset, static_field_size),
                instance_fields: encoded_array!(source, dex, offset, instance_field_size),
                direct_methods: encoded_array!(source, dex, offset, direct_methods_size),
                virtual_methods: encoded_array!(source, dex, offset, virtual_methods_size),
            },
            *offset,
        ))
    }
}

#[derive(Copy, Clone, Debug, Pread)]
pub(crate) struct ClassDefItem {
    pub(crate) class_idx: uint,
    pub(crate) access_flags: uint,
    pub(crate) superclass_idx: uint,
    pub(crate) interfaces_off: uint,
    pub(crate) source_file_idx: uint,
    pub(crate) annotations_off: uint,
    pub(crate) class_data_off: uint,
    pub(crate) static_values_off: uint,
}

pub(crate) struct ClassDefItemIter<T> {
    source: Source<T>,
    offset: usize,
    len: uint,
    endian: super::Endian,
}

impl<T> ClassDefItemIter<T> {
    pub(crate) fn new(source: Source<T>, offset: uint, len: uint, endian: super::Endian) -> Self {
        Self {
            source,
            offset: offset as usize,
            len,
            endian,
        }
    }
}

impl<T: AsRef<[u8]>> Iterator for ClassDefItemIter<T> {
    type Item = super::Result<ClassDefItem>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }
        let class_item: super::Result<ClassDefItem> = self
            .source
            .as_ref()
            .gread_with(&mut self.offset, self.endian)
            .map_err(Error::from);
        self.len -= 1;
        Some(class_item)
    }
}
