// This file is generated by rust-protobuf 3.0.0-alpha.10. Do not edit
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

//! Generated file from `notification.proto`

/// Generated files are compatible only with the same version
/// of protobuf runtime.
const _PROTOBUF_VERSION_CHECK: () = ::protobuf::VERSION_3_0_0_ALPHA_10;

#[derive(Clone,Copy,PartialEq,Eq,Debug,Hash)]
// @@protoc_insertion_point(enum:StatusNotification)
pub enum StatusNotification {
    // @@protoc_insertion_point(enum_value:StatusNotification.UNKNOWN)
    UNKNOWN = 0,
    // @@protoc_insertion_point(enum_value:StatusNotification.Running)
    Running = 1,
    // @@protoc_insertion_point(enum_value:StatusNotification.Reload)
    Reload = 2,
    // @@protoc_insertion_point(enum_value:StatusNotification.Exit)
    Exit = 3,
}

impl ::protobuf::Enum for StatusNotification {
    const NAME: &'static str = "StatusNotification";

    fn value(&self) -> i32 {
        *self as i32
    }

    fn from_i32(value: i32) -> ::std::option::Option<StatusNotification> {
        match value {
            0 => ::std::option::Option::Some(StatusNotification::UNKNOWN),
            1 => ::std::option::Option::Some(StatusNotification::Running),
            2 => ::std::option::Option::Some(StatusNotification::Reload),
            3 => ::std::option::Option::Some(StatusNotification::Exit),
            _ => ::std::option::Option::None
        }
    }

    const VALUES: &'static [StatusNotification] = &[
        StatusNotification::UNKNOWN,
        StatusNotification::Running,
        StatusNotification::Reload,
        StatusNotification::Exit,
    ];
}

impl ::protobuf::EnumFull for StatusNotification {
    fn enum_descriptor() -> ::protobuf::reflect::EnumDescriptor {
        static descriptor: ::protobuf::rt::Lazy<::protobuf::reflect::EnumDescriptor> = ::protobuf::rt::Lazy::new();
        descriptor.get(|| file_descriptor().enum_by_package_relative_name("StatusNotification").unwrap()).clone()
    }

    fn descriptor(&self) -> ::protobuf::reflect::EnumValueDescriptor {
        let index = *self as usize;
        Self::enum_descriptor().value_by_index(index)
    }
}

impl ::std::default::Default for StatusNotification {
    fn default() -> Self {
        StatusNotification::UNKNOWN
    }
}

impl StatusNotification {
    fn generated_enum_descriptor_data() -> ::protobuf::reflect::GeneratedEnumDescriptorData {
        ::protobuf::reflect::GeneratedEnumDescriptorData::new::<StatusNotification>("StatusNotification")
    }
}

static file_descriptor_proto_data: &'static [u8] = b"\
    \n\x12notification.proto*D\n\x12StatusNotification\x12\x0b\n\x07UNKNOWN\
    \x10\0\x12\x0b\n\x07Running\x10\x01\x12\n\n\x06Reload\x10\x02\x12\x08\n\
    \x04Exit\x10\x03b\x06proto3\
";

/// `FileDescriptorProto` object which was a source for this generated file
pub fn file_descriptor_proto() -> &'static ::protobuf::descriptor::FileDescriptorProto {
    static file_descriptor_proto_lazy: ::protobuf::rt::Lazy<::protobuf::descriptor::FileDescriptorProto> = ::protobuf::rt::Lazy::new();
    file_descriptor_proto_lazy.get(|| {
        ::protobuf::Message::parse_from_bytes(file_descriptor_proto_data).unwrap()
    })
}

/// `FileDescriptor` object which allows dynamic access to files
pub fn file_descriptor() -> ::protobuf::reflect::FileDescriptor {
    static file_descriptor_lazy: ::protobuf::rt::Lazy<::protobuf::reflect::GeneratedFileDescriptor> = ::protobuf::rt::Lazy::new();
    let file_descriptor = file_descriptor_lazy.get(|| {
        let mut deps = ::std::vec::Vec::with_capacity(0);
        let mut messages = ::std::vec::Vec::with_capacity(0);
        let mut enums = ::std::vec::Vec::with_capacity(1);
        enums.push(StatusNotification::generated_enum_descriptor_data());
        ::protobuf::reflect::GeneratedFileDescriptor::new_generated(
            file_descriptor_proto(),
            deps,
            messages,
            enums,
        )
    });
    ::protobuf::reflect::FileDescriptor::new_generated_2(file_descriptor)
}
