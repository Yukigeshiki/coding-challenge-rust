pub use get_animal_fact::*;
pub use health_check::*;

mod get_animal_fact;
pub mod health_check;

/// Implements the Debug trait for a DTO.
#[macro_export]
macro_rules! impl_json_display {
    ($model:ty) => {
        impl Display for $model {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{}",
                    serde_json::to_string(self).unwrap_or("Not available".into())
                )
            }
        }
    };
}
