//  CODEGEN.rs
//    by Tim Müller
//
//  Description:
//!   Provides helper macros for doing impls on complicated bounds efficiently
//!   and coherently.
//


/***** LIBRARY *****/
/// Counts the number of identifiers in the given list.
macro_rules! len_ident_list {
    ($ident:ident $(, $rem:ident)*) => {$crate::codegen::len_ident_list!($($rem),*) + 1};
    () => {
        0
    };
}
pub(crate) use len_ident_list;



/// Implements standard ops (and some for serde) for types that have complicated bounds. In
/// particular, they do some `P` generic that is `ToOwned`.
macro_rules! impl_struct_with_custom_derive {
    /* Private implementors */
    (impl Clone $(, $trait:ident)* for $name:ident { $($field:ident),* }) => {
        impl<P: ?Sized + ToOwned> Clone for $name<P>
        where
            P::Owned: Clone,
        {
            #[inline]
            fn clone(&self) -> Self { Self { $($field: self.$field.clone(),)* } }
        }

        // Don't forget to continue to implement any others
        $crate::codegen::impl_struct_with_custom_derive!(impl $($trait),* for $name { $($field),* });
    };
    (impl Debug $(, $trait:ident)* for $name:ident { $($field:ident),* }) => {
        impl<P: ?Sized + ToOwned> std::fmt::Debug for $name<P>
        where
            P::Owned: std::fmt::Debug,
        {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let mut fmt = f.debug_struct(std::any::type_name::<Self>());
                $(fmt.field(stringify!($field), &self.$field);)*
                fmt.finish()
            }
        }

        // Don't forget to continue to implement any others
        $crate::codegen::impl_struct_with_custom_derive!(impl $($trait),* for $name { $($field),* });
    };
    (impl Deserialize $(, $trait:ident)* for $name:ident { $($field:ident),* }) => {
        #[cfg(feature = "serde")]
        impl<'de, P: ?Sized + ToOwned> serde::Deserialize<'de> for $name<P>
        where
            P::Owned: serde::Deserialize<'de>,
        {
            #[inline]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                /// <https://serde.rs/deserialize-struct.html>
                struct Visitor<P: ?Sized>(std::marker::PhantomData<P>);
                impl<'de, P: ?Sized + ToOwned> serde::de::Visitor<'de> for Visitor<P>
                where
                    P::Owned: serde::Deserialize<'de>,
                {
                    type Value = $name<P>;

                    #[inline]
                    fn expecting(&self, f: &mut Formatter) -> FResult { write!(f, "an {}", stringify!($name).to_lowercase()) }

                    #[inline]
                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        #[derive(serde::Deserialize)]
                        #[serde(field_identifier)]
                        enum Key {
                            $(#[allow(non_camel_case_types)] $field,)*
                        }

                        $(let mut $field: Option<_> = None;)*
                        while let Some(next) = map.next_key()? {
                            match next {
                                $(
                                    Key::$field => {
                                        if $field.is_some() {
                                            return Err(<A::Error as serde::de::Error>::custom(concat!("Duplicate field '", stringify!($field), "'")));
                                        }
                                        $field = Some(map.next_value()?);
                                    },
                                )*
                            }
                        }
                        Ok($name {
                            $(
                                $field: match $field {
                                    Some(res) => res,
                                    None => return Err(<A::Error as serde::de::Error>::custom(concat!("Missing field '", stringify!($field), "'"))),
                                },
                            )*
                        })
                    }
                }

                deserializer.deserialize_map(Visitor(std::marker::PhantomData::<P>))
            }
        }

        // Don't forget to continue to implement any others
        $crate::codegen::impl_struct_with_custom_derive!(impl $($trait),* for $name { $($field),* });
    };
    (impl Serialize $(, $trait:ident)* for $name:ident { $($field:ident),* }) => {
        #[cfg(feature = "serde")]
        impl<P: ?Sized + ToOwned> serde::Serialize for $name<P>
        where
            P::Owned: serde::Serialize,
        {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeMap as _;

                let mut map = serializer.serialize_map(Some($crate::codegen::len_ident_list!($($field),*)))?;
                $(map.serialize_entry(stringify!($field), &self.$field)?;)*
                map.end()
            }
        }

        // Don't forget to continue to implement any others
        $crate::codegen::impl_struct_with_custom_derive!(impl $($trait),* for $name { $($field),* });
    };
    (impl for $name:ident { $($field:ident),* }) => { /* Done implementing */ };



    /* Public API */
    (
        #[derive($($trait:ident),*)]
        $(#[$($attrs:tt)*])*
        $vis:vis struct $name:ident<P: ?Sized + ToOwned> {
            $(
                $(#[$($field_attrs:tt)*])*
                $field_vis:vis $field:ident: $field_ty:ty
            ),*
            $(,)?
        }
    ) => {
        $(#[$($attrs)*])*
        $vis struct $name<P: ?Sized + ToOwned> {
            $(
                $(#[$($field_attrs)*])*
                $field_vis $field: $field_ty
            ),*
        }

        $crate::codegen::impl_struct_with_custom_derive!(impl $($trait),* for $name { $($field),* });
    };
}
pub(crate) use impl_struct_with_custom_derive;




/// Implements standard ops (and some for serde) for sumtypes that have complicated bounds. In
/// particular, they do some `P` generic that is `ToOwned`.
macro_rules! impl_enum_with_custom_derive {
    /* Private implementors */
    (impl Clone $(, $trait:ident)* for $name:ident { $($variant:ident { $($field:ident),* }),* }) => {
        impl<'a, P: ?Sized + ToOwned> Clone for $name<'a, P>
        where
            P::Owned: Clone,
        {
            #[inline]
            fn clone(&self) -> Self {
                match self {
                    $(Self::$variant { $($field),* } => { Self::$variant { $($field: $field.clone(),)* } },)*
                }
            }
        }

        // Don't forget to continue to implement any others
        $crate::codegen::impl_enum_with_custom_derive!(impl $($trait),* for $name { $($variant { $($field),* }),* });
    };
    (impl Debug $(, $trait:ident)* for $name:ident { $($variant:ident { $($field:ident),* }),* }) => {
        impl<'a, P: ?Sized + ToOwned> std::fmt::Debug for $name<'a, P>
        where
            P::Owned: std::fmt::Debug,
        {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $(Self::$variant { $($field),* } => {
                        let mut fmt = f.debug_struct(&format!("{}::{}", std::any::type_name::<Self>(), stringify!($variant)));
                        $(fmt.field(stringify!($field), $field);)*
                        fmt.finish()
                    },)*
                }
            }
        }

        // Don't forget to continue to implement any others
        $crate::codegen::impl_enum_with_custom_derive!(impl $($trait),* for $name { $($variant { $($field),* }),* });
    };
    (impl Deserialize $(, $trait:ident)* for $name:ident { $($variant:ident { $($field:ident),* }),* }) => {
        #[cfg(feature = "serde")]
        impl<'de, P: ?Sized + ToOwned> serde::Deserialize<'de> for $name<'de, P>
        where
            P::Owned: serde::Deserialize<'de>,
        {
            #[inline]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                /// <https://serde.rs/deserialize-struct.html>
                struct Visitor<P: ?Sized>(std::marker::PhantomData<P>);
                impl<'de, P: ?Sized + ToOwned> serde::de::Visitor<'de> for Visitor<P>
                where
                    P::Owned: serde::Deserialize<'de>,
                {
                    type Value = $name<'de, P>;

                    #[inline]
                    fn expecting(&self, f: &mut Formatter) -> FResult { write!(f, "an {}", stringify!($name).to_lowercase()) }

                    #[inline]
                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde::de::MapAccess<'de>,
                    {
                        // First, deserialize the kind-tag
                        let Some(key) = map.next_key::<&str>()? else {
                            return Err(<A::Error as serde::de::Error>::custom("Expected 'kind'-tag first"));
                        };
                        if key != "kind" { return Err(<A::Error as serde::de::Error>::custom("Expected 'kind'-tag first")); }
                        match map.next_value()? {
                            $(stringify!($variant) => {
                                #[derive(serde::Deserialize)]
                                #[serde(field_identifier)]
                                enum Key {
                                    $(#[allow(non_camel_case_types)] $field,)*
                                }

                                $(let mut $field: Option<_> = None;)*
                                while let Some(next) = map.next_key()? {
                                    match next {
                                        $(
                                            Key::$field => {
                                                if $field.is_some() {
                                                    return Err(<A::Error as serde::de::Error>::custom(concat!("Duplicate field '", stringify!($field), "'")));
                                                }
                                                $field = Some(map.next_value()?);
                                            },
                                        )*
                                    }
                                }
                                Ok($name::$variant {
                                    $(
                                        $field: match $field {
                                            Some(res) => res,
                                            None => return Err(<A::Error as serde::de::Error>::custom(concat!("Missing field '", stringify!($field), "'"))),
                                        },
                                    )*
                                })
                            },)*
                            other => return Err(<A::Error as serde::de::Error>::custom(&format!("Unexpected kind '{other}'"))),
                        }
                    }
                }

                deserializer.deserialize_map(Visitor(std::marker::PhantomData::<P>))
            }
        }

        // Don't forget to continue to implement any others
        $crate::codegen::impl_enum_with_custom_derive!(impl $($trait),* for $name { $($variant { $($field),* }),* });
    };
    (impl Serialize $(, $trait:ident)* for $name:ident { $($variant:ident { $($field:ident),* }),* }) => {
        #[cfg(feature = "serde")]
        impl<'a, P: ?Sized + ToOwned> serde::Serialize for $name<'a, P>
        where
            P::Owned: serde::Serialize,
        {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeMap as _;

                match self {
                    $(
                        Self::$variant { $($field),* } => {
                            let mut map = serializer.serialize_map(Some($crate::codegen::len_ident_list!($($field),*)))?;
                            map.serialize_entry("kind", stringify!($variant))?;
                            $(map.serialize_entry(stringify!($field), $field)?;)*
                            map.end()
                        },
                    )*
                }
            }
        }

        // Don't forget to continue to implement any others
        $crate::codegen::impl_enum_with_custom_derive!(impl $($trait),* for $name { $($variant { $($field),* }),* });
    };
    (impl for $name:ident { $($variant:ident { $($field:ident),* }),* }) => { /* Done implementing */ };



    /* Public API */
    (
        #[derive($($trait:ident),*)]
        $(#[$($attrs:tt)*])*
        $vis:vis enum $name:ident<'a, P: ToOwned> {
            $(
                $(#[$($variant_attrs:tt)*])*
                $variant:ident {
                    $(
                        $(#[$($field_attrs:tt)*])*
                        $field_vis:vis $field:ident: $field_ty:ty
                    ),*
                    $(,)?
                }
            ),*
            $(,)?
        }
    ) => {
        $(#[$($attrs)*])*
        $vis enum $name<'a, P: ?Sized + ToOwned> {
            $(
                $(#[$($variant_attrs)*])*
                $variant {
                    $(
                        $(#[$($field_attrs)*])*
                        $field_vis $field: $field_ty
                    ),*
                }
            ),*
        }

        $crate::codegen::impl_enum_with_custom_derive!(impl $($trait),* for $name { $($variant { $($field),* }),* });
    };
}
pub(crate) use impl_enum_with_custom_derive;
