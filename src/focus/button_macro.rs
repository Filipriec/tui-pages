// src/focus/button_macro.rs

#[macro_export]
macro_rules! define_buttons {
    ($name:ident { $($variant:ident),* $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(usize)]
        pub enum $name {
            $($variant,)*
        }

        impl $name {
            pub const COUNT: usize = <[()]>::len(&[$($crate::define_buttons!(@unit $variant)),*]);

            pub fn from_index(i: usize) -> Option<Self> {
                match i {
                    $(x if x == Self::$variant as usize => Some(Self::$variant),)*
                    _ => None,
                }
            }

            pub fn index(self) -> usize {
                self as usize
            }
        }
    };

    (@unit $variant:ident) => {
        ()
    };
}
