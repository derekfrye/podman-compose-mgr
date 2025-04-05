use crate::read_interactive_input::StdinHelper;

/// Test implementation that returns a predefined response
pub struct TestStdinHelper {
    pub response: String,
}

impl StdinHelper for TestStdinHelper {
    fn read_line(&self) -> String {
        self.response.clone()
    }
}
