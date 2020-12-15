pub trait Gradient {
    fn gradient(
        &self,
    ) -> ::std::collections::HashMap<
        ::std::string::String,
        (
            ::std::string::String,
            ::std::string::String,
            ::std::string::String,
        ),
    >;
}

pub use ::gradient_derive::*;
