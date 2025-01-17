use super::{CodegenTest, ExpectedCHeader, ExpectedRustTokens, ExpectedSwiftCode};
use proc_macro2::TokenStream;
use quote::quote;

/// Verify that we generate the correct to_ffi_repr() and to_rust_repr() implementations for an
/// enum where none of the variants contain any data.
mod generates_enum_to_and_from_ffi_conversions_no_data {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    Variant1,
                    Variant2,
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[derive(Copy, Clone)]
            pub enum SomeEnum {
                Variant1,
                Variant2
            }

            #[repr(C)]
            #[doc(hidden)]
            pub enum __swift_bridge__SomeEnum {
                Variant1,
                Variant2
            }

            impl swift_bridge::SharedEnum for SomeEnum {
                type FfiRepr = __swift_bridge__SomeEnum;
            }

            impl SomeEnum {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_ffi_repr(self) -> __swift_bridge__SomeEnum {
                    match self {
                        SomeEnum::Variant1 => __swift_bridge__SomeEnum::Variant1,
                        SomeEnum::Variant2 => __swift_bridge__SomeEnum::Variant2
                    }
                }
            }

            impl __swift_bridge__SomeEnum {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> SomeEnum {
                    match self {
                        __swift_bridge__SomeEnum::Variant1 => SomeEnum::Variant1,
                        __swift_bridge__SomeEnum::Variant2 => SomeEnum::Variant2
                    }
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public enum SomeEnum {
    case Variant1
    case Variant2
}
extension SomeEnum {
    func intoFfiRepr() -> __swift_bridge__$SomeEnum {
        switch self {
            case SomeEnum.Variant1:
                return __swift_bridge__$SomeEnum(tag: __swift_bridge__$SomeEnum$Variant1)
            case SomeEnum.Variant2:
                return __swift_bridge__$SomeEnum(tag: __swift_bridge__$SomeEnum$Variant2)
        }
    }
}
extension __swift_bridge__$SomeEnum {
    func intoSwiftRepr() -> SomeEnum {
        switch self.tag {
            case __swift_bridge__$SomeEnum$Variant1:
                return SomeEnum.Variant1
            case __swift_bridge__$SomeEnum$Variant2:
                return SomeEnum.Variant2
            default:
                fatalError("Unreachable")
        }
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
#include <stdbool.h>
typedef enum __swift_bridge__$SomeEnumTag { __swift_bridge__$SomeEnum$Variant1, __swift_bridge__$SomeEnum$Variant2, } __swift_bridge__$SomeEnumTag;
typedef struct __swift_bridge__$SomeEnum { __swift_bridge__$SomeEnumTag tag; } __swift_bridge__$SomeEnum;
typedef struct __swift_bridge__$Option$SomeEnum { bool is_some; __swift_bridge__$SomeEnum val; } __swift_bridge__$Option$SomeEnum;
"#,
        )
    }

    #[test]
    fn generates_enum_to_and_from_ffi_conversions_no_data() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate the correct code for a function that has an enum as an argument and
/// returns an enum.
mod using_enum_in_extern_rust_fn {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    Variant1,
                    Variant2,
                }

                extern "Rust" {
                    fn some_function(arg: SomeEnum) -> SomeEnum;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            pub extern "C" fn __swift_bridge__some_function(arg: __swift_bridge__SomeEnum) -> __swift_bridge__SomeEnum {
                super::some_function(arg.into_rust_repr()).into_ffi_repr()
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
func some_function(_ arg: SomeEnum) -> SomeEnum {
    __swift_bridge__$some_function(arg.intoFfiRepr()).intoSwiftRepr()
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
struct __swift_bridge__$SomeEnum __swift_bridge__$some_function(struct __swift_bridge__$SomeEnum arg);
    "#,
        )
    }

    #[test]
    fn using_enum_in_extern_rust_fn() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we can use `Option<Enum>` as Rust function arg and return type.
mod extern_rust_option_enum {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    Variant1,
                    Variant2,
                }

                extern "Rust" {
                    fn some_function(arg: Option<SomeEnum>) -> Option<SomeEnum>;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::ContainsMany(vec![
            quote! {
                #[repr(C)]
                #[doc(hidden)]
                pub struct __swift_bridge__Option_SomeEnum {
                    is_some: bool,
                    val: std::mem::MaybeUninit<__swift_bridge__SomeEnum>,
                }

                impl __swift_bridge__Option_SomeEnum {
                    #[doc(hidden)]
                    #[inline(always)]
                    pub fn into_rust_repr(self) -> Option<SomeEnum> {
                        if self.is_some {
                            Some(unsafe { self.val.assume_init().into_rust_repr() })
                        } else {
                            None
                        }
                    }

                    #[doc(hidden)]
                    #[inline(always)]
                    pub fn from_rust_repr(val: Option<SomeEnum>) -> __swift_bridge__Option_SomeEnum {
                        if let Some(val) = val {
                            __swift_bridge__Option_SomeEnum {
                                is_some: true,
                                val: std::mem::MaybeUninit::new(val.into_ffi_repr())
                            }
                        } else {
                            __swift_bridge__Option_SomeEnum {
                                is_some: false,
                                val: std::mem::MaybeUninit::uninit()
                            }
                        }
                    }
                }
            },
            quote! {
                pub extern "C" fn __swift_bridge__some_function(arg: __swift_bridge__Option_SomeEnum) -> __swift_bridge__Option_SomeEnum {
                    __swift_bridge__Option_SomeEnum::from_rust_repr(super::some_function(arg.into_rust_repr()))
                }
            },
        ])
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsManyAfterTrim(vec![
            r#"
extension __swift_bridge__$Option$SomeEnum {
    @inline(__always)
    func intoSwiftRepr() -> Optional<SomeEnum> {
        if self.is_some {
            return self.val.intoSwiftRepr()
        } else {
            return nil
        }
    }

    @inline(__always)
    static func fromSwiftRepr(_ val: Optional<SomeEnum>) -> __swift_bridge__$Option$SomeEnum {
        if let v = val {
            return __swift_bridge__$Option$SomeEnum(is_some: true, val: v.intoFfiRepr())
        } else {
            return __swift_bridge__$Option$SomeEnum(is_some: false, val: __swift_bridge__$SomeEnum())
        }
    }
}
"#,
            r#"
func some_function(_ arg: Optional<SomeEnum>) -> Optional<SomeEnum> {
    __swift_bridge__$some_function(__swift_bridge__$Option$SomeEnum.fromSwiftRepr(arg)).intoSwiftRepr()
}
"#,
        ])
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsManyAfterTrim(vec![
            r#"
#include <stdbool.h>
typedef enum __swift_bridge__$SomeEnumTag { __swift_bridge__$SomeEnum$Variant1, __swift_bridge__$SomeEnum$Variant2, } __swift_bridge__$SomeEnumTag;
typedef struct __swift_bridge__$SomeEnum { __swift_bridge__$SomeEnumTag tag; } __swift_bridge__$SomeEnum;
typedef struct __swift_bridge__$Option$SomeEnum { bool is_some; __swift_bridge__$SomeEnum val; } __swift_bridge__$Option$SomeEnum;
"#,
            r#"
struct __swift_bridge__$Option$SomeEnum __swift_bridge__$some_function(struct __swift_bridge__$Option$SomeEnum arg);
    "#,
        ])
    }

    #[test]
    fn option_enum() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that the original name of the enum is not present in any of the generated Swift
/// code when we use the `swift_name` attribute..
/// Related: crates/swift-integration-tests/src/enum_attributes/swift_name.rs
mod shared_enum_swift_name_attribute {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                #[swift_bridge(swift_name = "EnumRename")]
                enum EnumName {
                    Variant
                }


                extern "Rust" {
                    fn extern_rust_enum_rename(arg: EnumName) -> EnumName;
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::SkipTest
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::DoesNotContainAfterTrim("EnumName")
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::DoesNotContainAfterTrim("EnumName")
    }

    #[test]
    fn shared_enum_swift_name_attribute() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate an enum type that has a variant with one unnamed field and one with no fields.
mod generates_enum_to_and_from_ffi_conversions_one_unnamed_data_and_no_fields {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    Variant1(i32),
                    Variant2,
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[derive ()]
            pub enum SomeEnum {
                Variant1(i32),
                Variant2
            }

            #[repr(C)]
            #[doc(hidden)]
            pub enum __swift_bridge__SomeEnum {
                Variant1(i32),
                Variant2
            }

            impl swift_bridge::SharedEnum for SomeEnum {
                type FfiRepr = __swift_bridge__SomeEnum;
            }

            impl SomeEnum {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_ffi_repr(self) -> __swift_bridge__SomeEnum {
                    match self {
                        SomeEnum::Variant1(value_0) => __swift_bridge__SomeEnum::Variant1(value_0),
                        SomeEnum::Variant2 => __swift_bridge__SomeEnum::Variant2
                    }
                }
            }

            impl __swift_bridge__SomeEnum {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> SomeEnum {
                    match self {
                        __swift_bridge__SomeEnum::Variant1(value_0) => SomeEnum::Variant1(value_0),
                        __swift_bridge__SomeEnum::Variant2 => SomeEnum::Variant2
                    }
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public enum SomeEnum {
    case Variant1(Int32)
    case Variant2
}
extension SomeEnum {
    func intoFfiRepr() -> __swift_bridge__$SomeEnum {
        switch self {
            case SomeEnum.Variant1(let value_0):
                return __swift_bridge__$SomeEnum(tag: __swift_bridge__$SomeEnum$Variant1, payload: __swift_bridge__$SomeEnumFields(Variant1: __swift_bridge__$SomeEnum$FieldOfVariant1(_0: value_0)))
            case SomeEnum.Variant2:
                return {var val = __swift_bridge__$SomeEnum(); val.tag = __swift_bridge__$SomeEnum$Variant2; return val }()
        }
    }
}
extension __swift_bridge__$SomeEnum {
    func intoSwiftRepr() -> SomeEnum {
        switch self.tag {
            case __swift_bridge__$SomeEnum$Variant1:
                return SomeEnum.Variant1(self.payload.Variant1._0)
            case __swift_bridge__$SomeEnum$Variant2:
                return SomeEnum.Variant2
            default:
                fatalError("Unreachable")
        }
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
#include <stdint.h>
#include <stdbool.h>
typedef struct __swift_bridge__$SomeEnum$FieldOfVariant1 {int32_t _0;} __swift_bridge__$SomeEnum$FieldOfVariant1;

union __swift_bridge__$SomeEnumFields { __swift_bridge__$SomeEnum$FieldOfVariant1 Variant1;};
typedef enum __swift_bridge__$SomeEnumTag { __swift_bridge__$SomeEnum$Variant1, __swift_bridge__$SomeEnum$Variant2, } __swift_bridge__$SomeEnumTag;
typedef struct __swift_bridge__$SomeEnum { __swift_bridge__$SomeEnumTag tag; union __swift_bridge__$SomeEnumFields payload;} __swift_bridge__$SomeEnum;
typedef struct __swift_bridge__$Option$SomeEnum { bool is_some; __swift_bridge__$SomeEnum val; } __swift_bridge__$Option$SomeEnum;
"#,
        )
    }

    #[test]
    fn generates_enum_to_and_from_ffi_conversions_one_unnamed_data_and_no_fields() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}

/// Verify that we generate an enum type that has a variant with one unnamed field and one with two unnamed fields.
mod generates_enum_to_and_from_ffi_conversions_unnamed_data_and_two_unnamed_data {
    use super::*;

    fn bridge_module_tokens() -> TokenStream {
        quote! {
            #[swift_bridge::bridge]
            mod ffi {
                enum SomeEnum {
                    A(i32, u32),
                    B(String),
                }
            }
        }
    }

    fn expected_rust_tokens() -> ExpectedRustTokens {
        ExpectedRustTokens::Contains(quote! {
            #[derive ()]
            pub enum SomeEnum {
                A(i32, u32),
                B(String)
            }

            #[repr(C)]
            #[doc(hidden)]
            pub enum __swift_bridge__SomeEnum {
                A(i32, u32),
                B(*mut swift_bridge::string::RustString)
            }

            impl swift_bridge::SharedEnum for SomeEnum {
                type FfiRepr = __swift_bridge__SomeEnum;
            }

            impl SomeEnum {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_ffi_repr(self) -> __swift_bridge__SomeEnum {
                    match self {
                        SomeEnum::A(value_0, value_1) => __swift_bridge__SomeEnum::A(value_0, value_1),
                        SomeEnum::B(value_0) => __swift_bridge__SomeEnum::B(swift_bridge::string::RustString(value_0).box_into_raw())
                    }
                }
            }

            impl __swift_bridge__SomeEnum {
                #[doc(hidden)]
                #[inline(always)]
                pub fn into_rust_repr(self) -> SomeEnum {
                    match self {
                        __swift_bridge__SomeEnum::A(value_0, value_1) => SomeEnum::A(value_0, value_1),
                        __swift_bridge__SomeEnum::B(value_0) => SomeEnum::B(unsafe { Box::from_raw(value_0).0 })
                    }
                }
            }
        })
    }

    fn expected_swift_code() -> ExpectedSwiftCode {
        ExpectedSwiftCode::ContainsAfterTrim(
            r#"
public enum SomeEnum {
    case A(Int32, UInt32)
    case B(RustString)
}
extension SomeEnum {
    func intoFfiRepr() -> __swift_bridge__$SomeEnum {
        switch self {
            case SomeEnum.A(let value_0, let value_1):
                return __swift_bridge__$SomeEnum(tag: __swift_bridge__$SomeEnum$A, payload: __swift_bridge__$SomeEnumFields(A: __swift_bridge__$SomeEnum$FieldOfA(_0: value_0, _1: value_1)))
            case SomeEnum.B(let value_0):
                return __swift_bridge__$SomeEnum(tag: __swift_bridge__$SomeEnum$B, payload: __swift_bridge__$SomeEnumFields(B: __swift_bridge__$SomeEnum$FieldOfB(_0: { let rustString = value_0.intoRustString(); rustString.isOwned = false; return rustString.ptr }())))
        }
    }
}
extension __swift_bridge__$SomeEnum {
    func intoSwiftRepr() -> SomeEnum {
        switch self.tag {
            case __swift_bridge__$SomeEnum$A:
                return SomeEnum.A(self.payload.A._0, self.payload.A._1)
            case __swift_bridge__$SomeEnum$B:
                return SomeEnum.B(RustString(ptr: self.payload.B._0))
            default:
                fatalError("Unreachable")
        }
    }
}
"#,
        )
    }

    fn expected_c_header() -> ExpectedCHeader {
        ExpectedCHeader::ContainsAfterTrim(
            r#"
#include <stdint.h>
#include <stdbool.h>
typedef struct __swift_bridge__$SomeEnum$FieldOfA {int32_t _0; uint32_t _1;} __swift_bridge__$SomeEnum$FieldOfA;
typedef struct __swift_bridge__$SomeEnum$FieldOfB {void* _0;} __swift_bridge__$SomeEnum$FieldOfB;

union __swift_bridge__$SomeEnumFields { __swift_bridge__$SomeEnum$FieldOfA A; __swift_bridge__$SomeEnum$FieldOfB B;};
typedef enum __swift_bridge__$SomeEnumTag { __swift_bridge__$SomeEnum$A, __swift_bridge__$SomeEnum$B, } __swift_bridge__$SomeEnumTag;
typedef struct __swift_bridge__$SomeEnum { __swift_bridge__$SomeEnumTag tag; union __swift_bridge__$SomeEnumFields payload;} __swift_bridge__$SomeEnum;
typedef struct __swift_bridge__$Option$SomeEnum { bool is_some; __swift_bridge__$SomeEnum val; } __swift_bridge__$Option$SomeEnum;
"#,
        )
    }

    #[test]
    fn generates_enum_to_and_from_ffi_conversions_unnamed_data_and_two_unnamed_data() {
        CodegenTest {
            bridge_module: bridge_module_tokens().into(),
            expected_rust_tokens: expected_rust_tokens(),
            expected_swift_code: expected_swift_code(),
            expected_c_header: expected_c_header(),
        }
        .test();
    }
}
