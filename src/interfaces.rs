use crate::read_interactive_input::{GrammarFragment, ReadValResult};
use crate::secrets::models::SetSecretResponse;
use crate::secrets::b2_storage::B2UploadResult;
use crate::secrets::file_details::FileDetails;
use mockall::automock;
use std::path::Path;

/// Interface for command-related functions to facilitate testing
#[automock]
pub trait CommandHelper {
    fn exec_cmd(&self, cmd: &str, args: Vec<String>) -> Result<(), Box<dyn std::error::Error>>;
    fn pull_base_image(&self, dockerfile: &Path) -> Result<(), Box<dyn std::error::Error>>;
    fn get_terminal_display_width(&self, specify_size: Option<usize>) -> usize;
    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool;
}

/// Interface for read_val-related functions to facilitate testing
#[automock]
pub trait ReadInteractiveInputHelper {
    /// Read a value from the command line
    ///
    /// # Arguments
    /// * `grammars` - The grammar fragments to display in the prompt
    /// * `size` - Optional override for terminal width
    ///
    /// # Returns
    /// ReadValResult containing the user's input
    fn read_val_from_cmd_line_and_proceed(
        &self,
        grammars: &mut [GrammarFragment],
        size: Option<usize>,
    ) -> ReadValResult;
}

/// Default implementation of CommandHelper that uses the actual functions
pub struct DefaultCommandHelper;

impl CommandHelper for DefaultCommandHelper {
    fn exec_cmd(&self, cmd: &str, args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        // Convert Vec<String> to Vec<&str> for compatibility with existing function
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        crate::helpers::cmd_helper_fns::exec_cmd(cmd, &args_ref)?;
        Ok(())
    }

    fn pull_base_image(&self, dockerfile: &Path) -> Result<(), Box<dyn std::error::Error>> {
        crate::helpers::cmd_helper_fns::pull_base_image(dockerfile)
    }

    fn get_terminal_display_width(&self, specify_size: Option<usize>) -> usize {
        crate::helpers::cmd_helper_fns::get_terminal_display_width(specify_size)
    }

    fn file_exists_and_readable(&self, file: &std::path::Path) -> bool {
        crate::helpers::cmd_helper_fns::file_exists_and_readable(file)
    }
}

/// Default implementation of ReadValHelper that uses the actual function
pub struct DefaultReadInteractiveInputHelper;

impl ReadInteractiveInputHelper for DefaultReadInteractiveInputHelper {
    fn read_val_from_cmd_line_and_proceed(
        &self,
        grammars: &mut [GrammarFragment],
        size: Option<usize>,
    ) -> ReadValResult {
        // Use the default command helper for terminal width
        let cmd_helper = DefaultCommandHelper;
        crate::read_interactive_input::read_val_from_cmd_line_and_proceed_with_deps(
            grammars,
            &cmd_helper,
            Box::new(crate::read_interactive_input::default_print),
            size,
            None, // Use default stdin behavior
        )
    }
}

/// Interface for Azure KeyVault operations to facilitate testing
#[automock]
pub trait AzureKeyVaultClient {
    /// Sets a secret value in Azure KeyVault
    fn set_secret_value(
        &self,
        secret_name: &str,
        secret_value: &str,
    ) -> Result<SetSecretResponse, Box<dyn std::error::Error>>;

    /// Gets a secret value from Azure KeyVault
    fn get_secret_value(
        &self,
        secret_name: &str,
    ) -> Result<SetSecretResponse, Box<dyn std::error::Error>>;
}

/// Default implementation that uses the actual Azure KeyVault client
pub struct DefaultAzureKeyVaultClient {
    // The actual KeyvaultClient from the Azure SDK
    client: azure_security_keyvault::KeyvaultClient,
}

impl DefaultAzureKeyVaultClient {
    pub fn new(client: azure_security_keyvault::KeyvaultClient) -> Self {
        Self { client }
    }
}

impl AzureKeyVaultClient for DefaultAzureKeyVaultClient {
    fn set_secret_value(
        &self,
        secret_name: &str,
        secret_value: &str,
    ) -> Result<SetSecretResponse, Box<dyn std::error::Error>> {
        // Create a runtime for the async functions
        let rt = tokio::runtime::Runtime::new()?;

        // Call the actual implementation
        rt.block_on(crate::secrets::azure::set_secret_value(
            secret_name,
            &self.client,
            secret_value,
        ))
    }

    fn get_secret_value(
        &self,
        secret_name: &str,
    ) -> Result<SetSecretResponse, Box<dyn std::error::Error>> {
        // Create a runtime for the async functions
        let rt = tokio::runtime::Runtime::new()?;

        // Call the actual implementation
        rt.block_on(crate::secrets::azure::get_secret_value(
            secret_name,
            &self.client,
        ))
    }
}

/// Interface for B2 storage operations to facilitate testing
#[automock]
pub trait B2StorageClient {
    /// Creates a client from Args
    fn from_args(args: &crate::args::Args) -> Result<Self, Box<dyn std::error::Error>> where Self: Sized;
    
    /// Uploads a file to B2 storage
    fn upload_file_with_details(&self, file_details: &FileDetails) -> Result<B2UploadResult, Box<dyn std::error::Error>>;
}

/// Default implementation that uses the actual B2 client
pub struct DefaultB2StorageClient {
    // The real B2Client 
    client: crate::secrets::b2_storage::B2Client,
}

impl DefaultB2StorageClient {
    pub fn new(client: crate::secrets::b2_storage::B2Client) -> Self {
        Self { client }
    }
}

impl B2StorageClient for DefaultB2StorageClient {
    fn from_args(args: &crate::args::Args) -> Result<Self, Box<dyn std::error::Error>> {
        let client = crate::secrets::b2_storage::B2Client::from_args(args)?;
        Ok(Self::new(client))
    }
    
    fn upload_file_with_details(&self, file_details: &FileDetails) -> Result<B2UploadResult, Box<dyn std::error::Error>> {
        self.client.upload_file_with_details(file_details)
    }
}
