use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Test that the application properly handles Ctrl+C signals
///
/// This test:
/// 1. Launches the podman-compose-mgr process
/// 2. Waits until a prompt appears in the output
/// 3. Sends a SIGINT (Ctrl+C) signal
/// 4. Verifies the process exits within a reasonable time
#[test]
fn test_ctrl_c_handling() {
    // Build the application first to make sure it's available
    let build_status = Command::new("cargo")
        .args(["build"])
        .status()
        .expect("Failed to run cargo build");

    assert!(build_status.success(), "Failed to build the application");

    // Path to the built binary
    let main_binary = "target/debug/podman-compose-mgr";

    // Start the application with test parameters
    let mut child = Command::new(main_binary)
        .args(["--path", "tests/test1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start application");

    let pid = child.id();
    println!("Started application with PID: {pid}");

    // Create a stdout reader to monitor for prompts
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);

    // Wait for the application to reach a prompt
    let mut prompt_found = false;
    for line in reader.lines() {
        let Ok(line) = line else { break };

        println!("Output: {line}");

        // Check if we've reached a prompt
        if line.contains("p/N/d/b/s/?") {
            prompt_found = true;
            break;
        }

        // Check if the process has already exited
        if let Ok(Some(status)) = child.try_wait() {
            panic!("Process exited unexpectedly with status: {status}");
        }
    }

    assert!(prompt_found, "Application didn't output an expected prompt");
    println!("Prompt found, sending SIGINT...");

    // Send SIGINT to the process
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;

        let pid_i32 =
            i32::try_from(pid).expect("PID exceeds i32 range, which is highly unlikely");
        kill(Pid::from_raw(pid_i32), Signal::SIGINT)
            .expect("Failed to send SIGINT to process");

        println!("Sent SIGINT signal to application");
    }

    #[cfg(windows)]
    {
        // On Windows, we use the Win32 API to send Ctrl+C
        use windows::Win32::System::Console::{CTRL_C_EVENT, GenerateConsoleCtrlEvent};
        unsafe {
            GenerateConsoleCtrlEvent(CTRL_C_EVENT, pid).expect("Failed to send Ctrl+C event");
        }

        println!("Sent Ctrl+C event to application");
    }

    // Wait up to 5 seconds for the process to exit
    let mut exit_timeout = 50; // 50 * 100ms = 5 seconds
    let mut status = None;

    while exit_timeout > 0 {
        match child.try_wait() {
            Ok(Some(s)) => {
                status = Some(s);
                break;
            }
            Ok(None) => {
                // Process still running, wait a bit
                thread::sleep(Duration::from_millis(100));
                exit_timeout -= 1;
            }
            Err(e) => {
                panic!("Error checking process status: {e}");
            }
        }
    }

    // If the process is still running, kill it and fail the test
    if status.is_none() {
        child.kill().expect("Failed to kill process");
        panic!("Process didn't terminate after SIGINT within timeout");
    }

    println!("Process exited with status: {status:?}");

    // We validate that the application exited, but we don't check the exit code
    // as SIGINT normally results in a non-zero exit code on Unix (130)
    // Note: Our implementation uses std::process::exit(0) which always returns
    // a successful (0) exit code, which is why we see a 0 status here.
    assert!(status.is_some(), "Process should have exited after SIGINT");

    println!("Test successful: Application responded correctly to Ctrl+C signal");
}
