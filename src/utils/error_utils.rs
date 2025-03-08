use std::error::Error;

/// Convert any error type into a boxed dynamic Error with additional context
pub fn into_boxed_error<E: Error + 'static>(e: E, context: &str) -> Box<dyn Error> {
    Box::<dyn Error>::from(format!("{}: {}", context, e))
}

/// Add context to a Result type, converting the error into a Box<dyn Error>
pub fn with_context<T, E: Error + 'static>(
    result: Result<T, E>,
    context: &str,
) -> Result<T, Box<dyn Error>> {
    result.map_err(|e| into_boxed_error(e, context))
}

/// Create a new boxed error from a string message
pub fn new_error(message: &str) -> Box<dyn Error> {
    Box::<dyn Error>::from(message.to_string())
}
