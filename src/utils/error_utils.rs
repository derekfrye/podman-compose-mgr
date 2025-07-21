use std::error::Error;
use std::fmt;

/// Convert any error type into a boxed dynamic Error with additional context
pub fn into_boxed_error<E: Error + 'static>(e: E, context: &str) -> Box<dyn Error> {
    Box::<dyn Error>::from(format!("{context}: {e}"))
}

/// Add context to a Result type, converting the error into a Box<dyn Error>
///
/// # Arguments
///
/// * `result` - The result to add context to
/// * `context` - Context string to add to the error
///
/// # Returns
///
/// * `Result<T, Box<dyn Error>>` - The result with context added to any error
///
/// # Errors
///
/// Returns an error if the input result is an error, with added context.
pub fn with_context<T, E: Error + 'static>(
    result: Result<T, E>,
    context: &str,
) -> Result<T, Box<dyn Error>> {
    result.map_err(|e| into_boxed_error(e, context))
}

/// Create a new boxed error from a string message
#[must_use]
pub fn new_error(message: &str) -> Box<dyn Error> {
    Box::<dyn Error>::from(message.to_string())
}

/// A simple error type that wraps a string message
#[derive(Debug)]
pub struct ErrorFromStr(pub String);

impl fmt::Display for ErrorFromStr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for ErrorFromStr {}
