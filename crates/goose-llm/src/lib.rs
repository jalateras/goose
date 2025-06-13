uniffi::setup_scaffolding!();

mod completion;
pub mod extractors;
pub mod message;
mod model;
mod prompt_template;
pub mod providers;
mod structured_outputs;
pub mod types;
pub mod usage_tracker;

pub use completion::completion;
pub use message::Message;
pub use model::ModelConfig;
pub use structured_outputs::generate_structured_outputs;
