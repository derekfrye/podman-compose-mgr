use std::io::Write;
use reedline::{DefaultPrompt, Reedline, Signal};

/// Type for reading and processing user input
pub struct ReadValResult {
    pub user_entered_val: Option<String>,
    pub was_interrupted: bool,
}

/// For dependency injection in tests - PrintFunction type alias
/// Using trait object allows both regular functions and closures that capture environment
pub type PrintFunction<'a> = Box<dyn Fn(&str) + 'a>;

#[derive(Debug, PartialEq, Clone)]
pub enum GrammarType {
    Verbiage,
    UserChoice,
    Image,
    DockerComposePath,
    ContainerName,
    FileName,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GrammarFragment {
    pub original_val_for_prompt: Option<String>,
    pub shortened_val_for_prompt: Option<String>,
    pub pos: u8,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub grammar_type: GrammarType,
    pub display_at_all: bool,
    pub can_shorten: bool,
}

impl Default for GrammarFragment {
    fn default() -> Self {
        GrammarFragment {
            original_val_for_prompt: None,
            shortened_val_for_prompt: None,
            pos: 0,
            prefix: None,
            suffix: Some(" ".to_string()),
            grammar_type: GrammarType::Verbiage,
            can_shorten: false,
            display_at_all: true,
        }
    }
}

/// Trait for handling stdin operations, makes testing easier
pub trait StdinHelper {
    /// Read a line of input, possibly from stdin or a test double
    fn read_line(&self) -> String;
}

/// Default implementation that reads from actual stdin
pub struct DefaultStdinHelper;

impl StdinHelper for DefaultStdinHelper {
    fn read_line(&self) -> String {
        // flush stdout so prompt for sure displays
        std::io::stdout().flush().unwrap();
        
        // Initialize reedline editor
        let mut editor = Reedline::create();
        
        match editor.read_line(&DefaultPrompt) {
            Ok(Signal::Success(buffer)) => buffer,
            Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => {
                // Handle Ctrl+C/Ctrl+D by printing a message and exiting
                println!("\nOperation cancelled by user");
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("Error reading line: {}", err);
                String::new()
            }
        }
    }
}

/// Wrapper type for StdinHelper with static dispatch
pub enum StdinHelperWrapper {
    Default(DefaultStdinHelper),
    Test(crate::testing::stdin_helpers::TestStdinHelper),
}

impl StdinHelperWrapper {
    pub fn read_line(&self) -> String {
        match self {
            StdinHelperWrapper::Default(helper) => helper.read_line(),
            StdinHelperWrapper::Test(helper) => helper.read_line(),
        }
    }
}

impl Default for StdinHelperWrapper {
    fn default() -> Self {
        StdinHelperWrapper::Default(DefaultStdinHelper)
    }
}

