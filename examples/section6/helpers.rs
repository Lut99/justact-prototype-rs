//  HELPERS.rs
//    by Tim Müller
//
//  Description:
//!   Defines helpers for the examples.
//


/***** LIBRARY *****/
/// Helper macro for conveniently creating Slick ground atoms.
macro_rules! ground_atom {
    // Single identifiers
    ($ident:ident) => {
        ::slick::GroundAtom::Constant(::slick::text::Text::from_str(::std::stringify!($ident)))
    };
    ($ident:literal) => {
        ::slick::GroundAtom::Constant(::slick::text::Text::from_str($ident))
    };
    // Tuples
    (( $($nested:tt)* )) => {
        ::slick::GroundAtom::Tuple(vec![$($crate::helpers::ground_atom!($nested)),*])
    };
    // Sequences
    ($($toplevel:tt)*) => {
        ::slick::GroundAtom::Tuple(vec![$($crate::helpers::ground_atom!($toplevel)),*])
    };
}
pub(crate) use ground_atom;
