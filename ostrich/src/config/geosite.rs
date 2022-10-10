// This file is generated by rust-protobuf 3.2.0. Do not edit
// .proto file is parsed by protoc 3.19.4
// @generated

// https://github.com/rust-lang/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![allow(unused_attributes)]
#![cfg_attr(rustfmt, rustfmt::skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unused_results)]
#![allow(unused_mut)]

//! Generated file from `geosite.proto`

/// Generated files are compatible only with the same version
/// of protobuf runtime.
const _PROTOBUF_VERSION_CHECK: () = ::protobuf::VERSION_3_2_0;

#[derive(PartialEq,Clone,Default,Debug)]
// @@protoc_insertion_point(message:Domain)
pub struct Domain {
    // message fields
    // @@protoc_insertion_point(field:Domain.type)
    pub type_: ::protobuf::EnumOrUnknown<domain::Type>,
    // @@protoc_insertion_point(field:Domain.value)
    pub value: ::std::string::String,
    // @@protoc_insertion_point(field:Domain.attribute)
    pub attribute: ::std::vec::Vec<domain::Attribute>,
    // special fields
    // @@protoc_insertion_point(special_field:Domain.special_fields)
    pub special_fields: ::protobuf::SpecialFields,
}

impl<'a> ::std::default::Default for &'a Domain {
    fn default() -> &'a Domain {
        <Domain as ::protobuf::Message>::default_instance()
    }
}

impl Domain {
    pub fn new() -> Domain {
        ::std::default::Default::default()
    }

    fn generated_message_descriptor_data() -> ::protobuf::reflect::GeneratedMessageDescriptorData {
        let mut fields = ::std::vec::Vec::with_capacity(3);
        let mut oneofs = ::std::vec::Vec::with_capacity(0);
        fields.push(::protobuf::reflect::rt::v2::make_simpler_field_accessor::<_, _>(
            "type",
            |m: &Domain| { &m.type_ },
            |m: &mut Domain| { &mut m.type_ },
        ));
        fields.push(::protobuf::reflect::rt::v2::make_simpler_field_accessor::<_, _>(
            "value",
            |m: &Domain| { &m.value },
            |m: &mut Domain| { &mut m.value },
        ));
        fields.push(::protobuf::reflect::rt::v2::make_vec_simpler_accessor::<_, _>(
            "attribute",
            |m: &Domain| { &m.attribute },
            |m: &mut Domain| { &mut m.attribute },
        ));
        ::protobuf::reflect::GeneratedMessageDescriptorData::new_2::<Domain>(
            "Domain",
            fields,
            oneofs,
        )
    }
}

impl ::protobuf::Message for Domain {
    const NAME: &'static str = "Domain";

    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::Result<()> {
        while let Some(tag) = is.read_raw_tag_or_eof()? {
            match tag {
                8 => {
                    self.type_ = is.read_enum_or_unknown()?;
                },
                18 => {
                    self.value = is.read_string()?;
                },
                26 => {
                    self.attribute.push(is.read_message()?);
                },
                tag => {
                    ::protobuf::rt::read_unknown_or_skip_group(tag, is, self.special_fields.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u64 {
        let mut my_size = 0;
        if self.type_ != ::protobuf::EnumOrUnknown::new(domain::Type::Plain) {
            my_size += ::protobuf::rt::int32_size(1, self.type_.value());
        }
        if !self.value.is_empty() {
            my_size += ::protobuf::rt::string_size(2, &self.value);
        }
        for value in &self.attribute {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint64_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.special_fields.unknown_fields());
        self.special_fields.cached_size().set(my_size as u32);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::Result<()> {
        if self.type_ != ::protobuf::EnumOrUnknown::new(domain::Type::Plain) {
            os.write_enum(1, ::protobuf::EnumOrUnknown::value(&self.type_))?;
        }
        if !self.value.is_empty() {
            os.write_string(2, &self.value)?;
        }
        for v in &self.attribute {
            ::protobuf::rt::write_message_field_with_cached_size(3, v, os)?;
        };
        os.write_unknown_fields(self.special_fields.unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn special_fields(&self) -> &::protobuf::SpecialFields {
        &self.special_fields
    }

    fn mut_special_fields(&mut self) -> &mut ::protobuf::SpecialFields {
        &mut self.special_fields
    }

    fn new() -> Domain {
        Domain::new()
    }

    fn clear(&mut self) {
        self.type_ = ::protobuf::EnumOrUnknown::new(domain::Type::Plain);
        self.value.clear();
        self.attribute.clear();
        self.special_fields.clear();
    }

    fn default_instance() -> &'static Domain {
        static instance: Domain = Domain {
            type_: ::protobuf::EnumOrUnknown::from_i32(0),
            value: ::std::string::String::new(),
            attribute: ::std::vec::Vec::new(),
            special_fields: ::protobuf::SpecialFields::new(),
        };
        &instance
    }
}

impl ::protobuf::MessageFull for Domain {
    fn descriptor() -> ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::Lazy::new();
        descriptor.get(|| file_descriptor().message_by_package_relative_name("Domain").unwrap()).clone()
    }
}

impl ::std::fmt::Display for Domain {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for Domain {
    type RuntimeType = ::protobuf::reflect::rt::RuntimeTypeMessage<Self>;
}

/// Nested message and enums of message `Domain`
pub mod domain {
    #[derive(PartialEq,Clone,Default,Debug)]
    // @@protoc_insertion_point(message:Domain.Attribute)
    pub struct Attribute {
        // message fields
        // @@protoc_insertion_point(field:Domain.Attribute.key)
        pub key: ::std::string::String,
        // message oneof groups
        pub typed_value: ::std::option::Option<attribute::Typed_value>,
        // special fields
        // @@protoc_insertion_point(special_field:Domain.Attribute.special_fields)
        pub special_fields: ::protobuf::SpecialFields,
    }

    impl<'a> ::std::default::Default for &'a Attribute {
        fn default() -> &'a Attribute {
            <Attribute as ::protobuf::Message>::default_instance()
        }
    }

    impl Attribute {
        pub fn new() -> Attribute {
            ::std::default::Default::default()
        }

        // bool bool_value = 2;

        pub fn bool_value(&self) -> bool {
            match self.typed_value {
                ::std::option::Option::Some(attribute::Typed_value::BoolValue(v)) => v,
                _ => false,
            }
        }

        pub fn clear_bool_value(&mut self) {
            self.typed_value = ::std::option::Option::None;
        }

        pub fn has_bool_value(&self) -> bool {
            match self.typed_value {
                ::std::option::Option::Some(attribute::Typed_value::BoolValue(..)) => true,
                _ => false,
            }
        }

        // Param is passed by value, moved
        pub fn set_bool_value(&mut self, v: bool) {
            self.typed_value = ::std::option::Option::Some(attribute::Typed_value::BoolValue(v))
        }

        // int64 int_value = 3;

        pub fn int_value(&self) -> i64 {
            match self.typed_value {
                ::std::option::Option::Some(attribute::Typed_value::IntValue(v)) => v,
                _ => 0,
            }
        }

        pub fn clear_int_value(&mut self) {
            self.typed_value = ::std::option::Option::None;
        }

        pub fn has_int_value(&self) -> bool {
            match self.typed_value {
                ::std::option::Option::Some(attribute::Typed_value::IntValue(..)) => true,
                _ => false,
            }
        }

        // Param is passed by value, moved
        pub fn set_int_value(&mut self, v: i64) {
            self.typed_value = ::std::option::Option::Some(attribute::Typed_value::IntValue(v))
        }

        pub(in super) fn generated_message_descriptor_data() -> ::protobuf::reflect::GeneratedMessageDescriptorData {
            let mut fields = ::std::vec::Vec::with_capacity(3);
            let mut oneofs = ::std::vec::Vec::with_capacity(1);
            fields.push(::protobuf::reflect::rt::v2::make_simpler_field_accessor::<_, _>(
                "key",
                |m: &Attribute| { &m.key },
                |m: &mut Attribute| { &mut m.key },
            ));
            fields.push(::protobuf::reflect::rt::v2::make_oneof_copy_has_get_set_simpler_accessors::<_, _>(
                "bool_value",
                Attribute::has_bool_value,
                Attribute::bool_value,
                Attribute::set_bool_value,
            ));
            fields.push(::protobuf::reflect::rt::v2::make_oneof_copy_has_get_set_simpler_accessors::<_, _>(
                "int_value",
                Attribute::has_int_value,
                Attribute::int_value,
                Attribute::set_int_value,
            ));
            oneofs.push(attribute::Typed_value::generated_oneof_descriptor_data());
            ::protobuf::reflect::GeneratedMessageDescriptorData::new_2::<Attribute>(
                "Domain.Attribute",
                fields,
                oneofs,
            )
        }
    }

    impl ::protobuf::Message for Attribute {
        const NAME: &'static str = "Attribute";

        fn is_initialized(&self) -> bool {
            true
        }

        fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::Result<()> {
            while let Some(tag) = is.read_raw_tag_or_eof()? {
                match tag {
                    10 => {
                        self.key = is.read_string()?;
                    },
                    16 => {
                        self.typed_value = ::std::option::Option::Some(attribute::Typed_value::BoolValue(is.read_bool()?));
                    },
                    24 => {
                        self.typed_value = ::std::option::Option::Some(attribute::Typed_value::IntValue(is.read_int64()?));
                    },
                    tag => {
                        ::protobuf::rt::read_unknown_or_skip_group(tag, is, self.special_fields.mut_unknown_fields())?;
                    },
                };
            }
            ::std::result::Result::Ok(())
        }

        // Compute sizes of nested messages
        #[allow(unused_variables)]
        fn compute_size(&self) -> u64 {
            let mut my_size = 0;
            if !self.key.is_empty() {
                my_size += ::protobuf::rt::string_size(1, &self.key);
            }
            if let ::std::option::Option::Some(ref v) = self.typed_value {
                match v {
                    &attribute::Typed_value::BoolValue(v) => {
                        my_size += 1 + 1;
                    },
                    &attribute::Typed_value::IntValue(v) => {
                        my_size += ::protobuf::rt::int64_size(3, v);
                    },
                };
            }
            my_size += ::protobuf::rt::unknown_fields_size(self.special_fields.unknown_fields());
            self.special_fields.cached_size().set(my_size as u32);
            my_size
        }

        fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::Result<()> {
            if !self.key.is_empty() {
                os.write_string(1, &self.key)?;
            }
            if let ::std::option::Option::Some(ref v) = self.typed_value {
                match v {
                    &attribute::Typed_value::BoolValue(v) => {
                        os.write_bool(2, v)?;
                    },
                    &attribute::Typed_value::IntValue(v) => {
                        os.write_int64(3, v)?;
                    },
                };
            }
            os.write_unknown_fields(self.special_fields.unknown_fields())?;
            ::std::result::Result::Ok(())
        }

        fn special_fields(&self) -> &::protobuf::SpecialFields {
            &self.special_fields
        }

        fn mut_special_fields(&mut self) -> &mut ::protobuf::SpecialFields {
            &mut self.special_fields
        }

        fn new() -> Attribute {
            Attribute::new()
        }

        fn clear(&mut self) {
            self.key.clear();
            self.typed_value = ::std::option::Option::None;
            self.typed_value = ::std::option::Option::None;
            self.special_fields.clear();
        }

        fn default_instance() -> &'static Attribute {
            static instance: Attribute = Attribute {
                key: ::std::string::String::new(),
                typed_value: ::std::option::Option::None,
                special_fields: ::protobuf::SpecialFields::new(),
            };
            &instance
        }
    }

    impl ::protobuf::MessageFull for Attribute {
        fn descriptor() -> ::protobuf::reflect::MessageDescriptor {
            static descriptor: ::protobuf::rt::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::Lazy::new();
            descriptor.get(|| super::file_descriptor().message_by_package_relative_name("Domain.Attribute").unwrap()).clone()
        }
    }

    impl ::std::fmt::Display for Attribute {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            ::protobuf::text_format::fmt(self, f)
        }
    }

    impl ::protobuf::reflect::ProtobufValue for Attribute {
        type RuntimeType = ::protobuf::reflect::rt::RuntimeTypeMessage<Self>;
    }

    /// Nested message and enums of message `Attribute`
    pub mod attribute {

        #[derive(Clone,PartialEq,Debug)]
        #[non_exhaustive]
        // @@protoc_insertion_point(oneof:Domain.Attribute.typed_value)
        pub enum Typed_value {
            // @@protoc_insertion_point(oneof_field:Domain.Attribute.bool_value)
            BoolValue(bool),
            // @@protoc_insertion_point(oneof_field:Domain.Attribute.int_value)
            IntValue(i64),
        }

        impl ::protobuf::Oneof for Typed_value {
        }

        impl ::protobuf::OneofFull for Typed_value {
            fn descriptor() -> ::protobuf::reflect::OneofDescriptor {
                static descriptor: ::protobuf::rt::Lazy<::protobuf::reflect::OneofDescriptor> = ::protobuf::rt::Lazy::new();
                descriptor.get(|| <super::Attribute as ::protobuf::MessageFull>::descriptor().oneof_by_name("typed_value").unwrap()).clone()
            }
        }

        impl Typed_value {
            pub(in super::super) fn generated_oneof_descriptor_data() -> ::protobuf::reflect::GeneratedOneofDescriptorData {
                ::protobuf::reflect::GeneratedOneofDescriptorData::new::<Typed_value>("typed_value")
            }
        }
    }

    #[derive(Clone,Copy,PartialEq,Eq,Debug,Hash)]
    // @@protoc_insertion_point(enum:Domain.Type)
    pub enum Type {
        // @@protoc_insertion_point(enum_value:Domain.Type.Plain)
        Plain = 0,
        // @@protoc_insertion_point(enum_value:Domain.Type.Regex)
        Regex = 1,
        // @@protoc_insertion_point(enum_value:Domain.Type.Domain)
        Domain = 2,
        // @@protoc_insertion_point(enum_value:Domain.Type.Full)
        Full = 3,
    }

    impl ::protobuf::Enum for Type {
        const NAME: &'static str = "Type";

        fn value(&self) -> i32 {
            *self as i32
        }

        fn from_i32(value: i32) -> ::std::option::Option<Type> {
            match value {
                0 => ::std::option::Option::Some(Type::Plain),
                1 => ::std::option::Option::Some(Type::Regex),
                2 => ::std::option::Option::Some(Type::Domain),
                3 => ::std::option::Option::Some(Type::Full),
                _ => ::std::option::Option::None
            }
        }

        const VALUES: &'static [Type] = &[
            Type::Plain,
            Type::Regex,
            Type::Domain,
            Type::Full,
        ];
    }

    impl ::protobuf::EnumFull for Type {
        fn enum_descriptor() -> ::protobuf::reflect::EnumDescriptor {
            static descriptor: ::protobuf::rt::Lazy<::protobuf::reflect::EnumDescriptor> = ::protobuf::rt::Lazy::new();
            descriptor.get(|| super::file_descriptor().enum_by_package_relative_name("Domain.Type").unwrap()).clone()
        }

        fn descriptor(&self) -> ::protobuf::reflect::EnumValueDescriptor {
            let index = *self as usize;
            Self::enum_descriptor().value_by_index(index)
        }
    }

    impl ::std::default::Default for Type {
        fn default() -> Self {
            Type::Plain
        }
    }

    impl Type {
        pub(in super) fn generated_enum_descriptor_data() -> ::protobuf::reflect::GeneratedEnumDescriptorData {
            ::protobuf::reflect::GeneratedEnumDescriptorData::new::<Type>("Domain.Type")
        }
    }
}

#[derive(PartialEq,Clone,Default,Debug)]
// @@protoc_insertion_point(message:SiteGroup)
pub struct SiteGroup {
    // message fields
    // @@protoc_insertion_point(field:SiteGroup.tag)
    pub tag: ::std::string::String,
    // @@protoc_insertion_point(field:SiteGroup.domain)
    pub domain: ::std::vec::Vec<Domain>,
    // special fields
    // @@protoc_insertion_point(special_field:SiteGroup.special_fields)
    pub special_fields: ::protobuf::SpecialFields,
}

impl<'a> ::std::default::Default for &'a SiteGroup {
    fn default() -> &'a SiteGroup {
        <SiteGroup as ::protobuf::Message>::default_instance()
    }
}

impl SiteGroup {
    pub fn new() -> SiteGroup {
        ::std::default::Default::default()
    }

    fn generated_message_descriptor_data() -> ::protobuf::reflect::GeneratedMessageDescriptorData {
        let mut fields = ::std::vec::Vec::with_capacity(2);
        let mut oneofs = ::std::vec::Vec::with_capacity(0);
        fields.push(::protobuf::reflect::rt::v2::make_simpler_field_accessor::<_, _>(
            "tag",
            |m: &SiteGroup| { &m.tag },
            |m: &mut SiteGroup| { &mut m.tag },
        ));
        fields.push(::protobuf::reflect::rt::v2::make_vec_simpler_accessor::<_, _>(
            "domain",
            |m: &SiteGroup| { &m.domain },
            |m: &mut SiteGroup| { &mut m.domain },
        ));
        ::protobuf::reflect::GeneratedMessageDescriptorData::new_2::<SiteGroup>(
            "SiteGroup",
            fields,
            oneofs,
        )
    }
}

impl ::protobuf::Message for SiteGroup {
    const NAME: &'static str = "SiteGroup";

    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::Result<()> {
        while let Some(tag) = is.read_raw_tag_or_eof()? {
            match tag {
                10 => {
                    self.tag = is.read_string()?;
                },
                18 => {
                    self.domain.push(is.read_message()?);
                },
                tag => {
                    ::protobuf::rt::read_unknown_or_skip_group(tag, is, self.special_fields.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u64 {
        let mut my_size = 0;
        if !self.tag.is_empty() {
            my_size += ::protobuf::rt::string_size(1, &self.tag);
        }
        for value in &self.domain {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint64_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.special_fields.unknown_fields());
        self.special_fields.cached_size().set(my_size as u32);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::Result<()> {
        if !self.tag.is_empty() {
            os.write_string(1, &self.tag)?;
        }
        for v in &self.domain {
            ::protobuf::rt::write_message_field_with_cached_size(2, v, os)?;
        };
        os.write_unknown_fields(self.special_fields.unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn special_fields(&self) -> &::protobuf::SpecialFields {
        &self.special_fields
    }

    fn mut_special_fields(&mut self) -> &mut ::protobuf::SpecialFields {
        &mut self.special_fields
    }

    fn new() -> SiteGroup {
        SiteGroup::new()
    }

    fn clear(&mut self) {
        self.tag.clear();
        self.domain.clear();
        self.special_fields.clear();
    }

    fn default_instance() -> &'static SiteGroup {
        static instance: SiteGroup = SiteGroup {
            tag: ::std::string::String::new(),
            domain: ::std::vec::Vec::new(),
            special_fields: ::protobuf::SpecialFields::new(),
        };
        &instance
    }
}

impl ::protobuf::MessageFull for SiteGroup {
    fn descriptor() -> ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::Lazy::new();
        descriptor.get(|| file_descriptor().message_by_package_relative_name("SiteGroup").unwrap()).clone()
    }
}

impl ::std::fmt::Display for SiteGroup {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for SiteGroup {
    type RuntimeType = ::protobuf::reflect::rt::RuntimeTypeMessage<Self>;
}

#[derive(PartialEq,Clone,Default,Debug)]
// @@protoc_insertion_point(message:SiteGroupList)
pub struct SiteGroupList {
    // message fields
    // @@protoc_insertion_point(field:SiteGroupList.site_group)
    pub site_group: ::std::vec::Vec<SiteGroup>,
    // special fields
    // @@protoc_insertion_point(special_field:SiteGroupList.special_fields)
    pub special_fields: ::protobuf::SpecialFields,
}

impl<'a> ::std::default::Default for &'a SiteGroupList {
    fn default() -> &'a SiteGroupList {
        <SiteGroupList as ::protobuf::Message>::default_instance()
    }
}

impl SiteGroupList {
    pub fn new() -> SiteGroupList {
        ::std::default::Default::default()
    }

    fn generated_message_descriptor_data() -> ::protobuf::reflect::GeneratedMessageDescriptorData {
        let mut fields = ::std::vec::Vec::with_capacity(1);
        let mut oneofs = ::std::vec::Vec::with_capacity(0);
        fields.push(::protobuf::reflect::rt::v2::make_vec_simpler_accessor::<_, _>(
            "site_group",
            |m: &SiteGroupList| { &m.site_group },
            |m: &mut SiteGroupList| { &mut m.site_group },
        ));
        ::protobuf::reflect::GeneratedMessageDescriptorData::new_2::<SiteGroupList>(
            "SiteGroupList",
            fields,
            oneofs,
        )
    }
}

impl ::protobuf::Message for SiteGroupList {
    const NAME: &'static str = "SiteGroupList";

    fn is_initialized(&self) -> bool {
        true
    }

    fn merge_from(&mut self, is: &mut ::protobuf::CodedInputStream<'_>) -> ::protobuf::Result<()> {
        while let Some(tag) = is.read_raw_tag_or_eof()? {
            match tag {
                10 => {
                    self.site_group.push(is.read_message()?);
                },
                tag => {
                    ::protobuf::rt::read_unknown_or_skip_group(tag, is, self.special_fields.mut_unknown_fields())?;
                },
            };
        }
        ::std::result::Result::Ok(())
    }

    // Compute sizes of nested messages
    #[allow(unused_variables)]
    fn compute_size(&self) -> u64 {
        let mut my_size = 0;
        for value in &self.site_group {
            let len = value.compute_size();
            my_size += 1 + ::protobuf::rt::compute_raw_varint64_size(len) + len;
        };
        my_size += ::protobuf::rt::unknown_fields_size(self.special_fields.unknown_fields());
        self.special_fields.cached_size().set(my_size as u32);
        my_size
    }

    fn write_to_with_cached_sizes(&self, os: &mut ::protobuf::CodedOutputStream<'_>) -> ::protobuf::Result<()> {
        for v in &self.site_group {
            ::protobuf::rt::write_message_field_with_cached_size(1, v, os)?;
        };
        os.write_unknown_fields(self.special_fields.unknown_fields())?;
        ::std::result::Result::Ok(())
    }

    fn special_fields(&self) -> &::protobuf::SpecialFields {
        &self.special_fields
    }

    fn mut_special_fields(&mut self) -> &mut ::protobuf::SpecialFields {
        &mut self.special_fields
    }

    fn new() -> SiteGroupList {
        SiteGroupList::new()
    }

    fn clear(&mut self) {
        self.site_group.clear();
        self.special_fields.clear();
    }

    fn default_instance() -> &'static SiteGroupList {
        static instance: SiteGroupList = SiteGroupList {
            site_group: ::std::vec::Vec::new(),
            special_fields: ::protobuf::SpecialFields::new(),
        };
        &instance
    }
}

impl ::protobuf::MessageFull for SiteGroupList {
    fn descriptor() -> ::protobuf::reflect::MessageDescriptor {
        static descriptor: ::protobuf::rt::Lazy<::protobuf::reflect::MessageDescriptor> = ::protobuf::rt::Lazy::new();
        descriptor.get(|| file_descriptor().message_by_package_relative_name("SiteGroupList").unwrap()).clone()
    }
}

impl ::std::fmt::Display for SiteGroupList {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        ::protobuf::text_format::fmt(self, f)
    }
}

impl ::protobuf::reflect::ProtobufValue for SiteGroupList {
    type RuntimeType = ::protobuf::reflect::rt::RuntimeTypeMessage<Self>;
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\rgeosite.proto\"\x93\x02\n\x06Domain\x12\x20\n\x04type\x18\x01\x20\
    \x01(\x0e2\x0c.Domain.TypeR\x04type\x12\x14\n\x05value\x18\x02\x20\x01(\
    \tR\x05value\x12/\n\tattribute\x18\x03\x20\x03(\x0b2\x11.Domain.Attribut\
    eR\tattribute\x1al\n\tAttribute\x12\x10\n\x03key\x18\x01\x20\x01(\tR\x03\
    key\x12\x1f\n\nbool_value\x18\x02\x20\x01(\x08H\0R\tboolValue\x12\x1d\n\
    \tint_value\x18\x03\x20\x01(\x03H\0R\x08intValueB\r\n\x0btyped_value\"2\
    \n\x04Type\x12\t\n\x05Plain\x10\0\x12\t\n\x05Regex\x10\x01\x12\n\n\x06Do\
    main\x10\x02\x12\x08\n\x04Full\x10\x03\">\n\tSiteGroup\x12\x10\n\x03tag\
    \x18\x01\x20\x01(\tR\x03tag\x12\x1f\n\x06domain\x18\x02\x20\x03(\x0b2\
    \x07.DomainR\x06domain\":\n\rSiteGroupList\x12)\n\nsite_group\x18\x01\
    \x20\x03(\x0b2\n.SiteGroupR\tsiteGroupb\x06proto3\
";

/// `FileDescriptorProto` object which was a source for this generated file
fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    static file_descriptor_proto_lazy: ::protobuf::rt::Lazy<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::rt::Lazy::new();
    file_descriptor_proto_lazy.get(|| {
        ::protobuf::Message::parse_from_bytes(file_descriptor_proto_data).unwrap()
    })
}

/// `FileDescriptor` object which allows dynamic access to files
pub fn file_descriptor() -> &'static ::protobuf::reflect::FileDescriptor {
    static generated_file_descriptor_lazy: ::protobuf::rt::Lazy<::protobuf::reflect::GeneratedFileDescriptor> = ::protobuf::rt::Lazy::new();
    static file_descriptor: ::protobuf::rt::Lazy<::protobuf::reflect::FileDescriptor> = ::protobuf::rt::Lazy::new();
    file_descriptor.get(|| {
        let generated_file_descriptor = generated_file_descriptor_lazy.get(|| {
            let mut deps = ::std::vec::Vec::with_capacity(0);
            let mut messages = ::std::vec::Vec::with_capacity(4);
            messages.push(Domain::generated_message_descriptor_data());
            messages.push(SiteGroup::generated_message_descriptor_data());
            messages.push(SiteGroupList::generated_message_descriptor_data());
            messages.push(domain::Attribute::generated_message_descriptor_data());
            let mut enums = ::std::vec::Vec::with_capacity(1);
            enums.push(domain::Type::generated_enum_descriptor_data());
            ::protobuf::reflect::GeneratedFileDescriptor::new_generated(
                file_descriptor_proto(),
                deps,
                messages,
                enums,
            )
        });
        ::protobuf::reflect::FileDescriptor::new_generated_2(generated_file_descriptor)
    })
}
