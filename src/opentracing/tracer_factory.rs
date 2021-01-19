use std::rc::Rc;

use eyre::Result;

use super::Tracer;

pub(crate) enum TracerFactoryError {
    /// `configuration_parse_error` occurs when the configuration string used to
    /// construct a tracer does not adhere to the expected format.
    ConfigurationError,
    /// `invalid_configuration_error` occurs if the requested configuration for a
    /// tracer has invalid values.
    InvalidConfiguration,
}

/// TracerFactory constructs tracers from configuration strings.
pub(crate) trait TracerFactory {
    /// Creates a tracer with the requested `configuration`.
    fn make_tracer(&self, configuration: &str) -> Result<Rc<dyn Tracer>>;
}
