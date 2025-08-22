use anyhow::Result;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn, error};
use uuid::Uuid;

use super::{CommandExecutionResult, SafetyLevel, TerminalContext};

pub struct ShellExecutor {
    shell_path: String,
    timeout: Duration,
}

impl ShellExecutor {
    pub fn new() -> Self {
        let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        
        Self {
            shell_path,
            timeout: Duration::from_secs(300), // 5 dakika timeout
        }
    }

    pub async fn execute_command(
        &self,
        command: &str,
        context: &TerminalContext,
        safety_level: SafetyLevel,
    ) -> Result<CommandExecutionResult> {
        // Güvenlik kontrolü
        if matches!(safety_level, SafetyLevel::Blocked) {
            return Ok(CommandExecutionResult {
                command: command.to_string(),
                output: String::new(),
                error: Some("Komut güvenlik nedeniyle engellendi".to_string()),
                exit_code: 1,
                execution_time_ms: 0,
            });
        }

        let start_time = Instant::now();
        
        info!("Executing command: {}", command);

        let mut cmd = Command::new(&self.shell_path);
        cmd.arg("-c")
           .arg(command)
           .current_dir(&context.current_directory)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Environment variables'ları ayarla
        for (key, value) in &context.environment_vars {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;

        // Timeout ile çalıştır
        let result = tokio::time::timeout(self.timeout, async {
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let stdout_reader = BufReader::new(stdout);
            let stderr_reader = BufReader::new(stderr);

            let mut stdout_lines = stdout_reader.lines();
            let mut stderr_lines = stderr_reader.lines();

            let mut output = String::new();
            let mut error_output = String::new();

            // Stdout ve stderr'ı paralel olarak oku
            loop {
                tokio::select! {
                    line = stdout_lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                output.push_str(&line);
                                output.push('\n');
                            }
                            Ok(None) => break,
                            Err(e) => {
                                error!("Error reading stdout: {}", e);
                                break;
                            }
                        }
                    }
                    line = stderr_lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                error_output.push_str(&line);
                                error_output.push('\n');
                            }
                            Ok(None) => {},
                            Err(e) => {
                                error!("Error reading stderr: {}", e);
                            }
                        }
                    }
                }
            }

            let status = child.wait().await?;
            let exit_code = status.code().unwrap_or(-1);

            Ok::<_, anyhow::Error>((output, error_output, exit_code))
        }).await;

        let execution_time = start_time.elapsed();

        match result {
            Ok(Ok((output, error_output, exit_code))) => {
                let final_error = if error_output.is_empty() { None } else { Some(error_output) };
                
                Ok(CommandExecutionResult {
                    command: command.to_string(),
                    output,
                    error: final_error,
                    exit_code,
                    execution_time_ms: execution_time.as_millis() as u64,
                })
            }
            Ok(Err(e)) => {
                error!("Command execution error: {}", e);
                Ok(CommandExecutionResult {
                    command: command.to_string(),
                    output: String::new(),
                    error: Some(format!("Execution error: {}", e)),
                    exit_code: -1,
                    execution_time_ms: execution_time.as_millis() as u64,
                })
            }
            Err(_) => {
                // Timeout
                warn!("Command timed out: {}", command);
                let _ = child.kill().await;
                
                Ok(CommandExecutionResult {
                    command: command.to_string(),
                    output: String::new(),
                    error: Some("Komut zaman aşımına uğradı".to_string()),
                    exit_code: 124, // timeout exit code
                    execution_time_ms: self.timeout.as_millis() as u64,
                })
            }
        }
    }

    pub async fn execute_interactive_command(
        &self,
        command: &str,
        context: &TerminalContext,
    ) -> Result<InteractiveSession> {
        info!("Starting interactive command: {}", command);

        let mut cmd = Command::new(&self.shell_path);
        cmd.arg("-c")
           .arg(command)
           .current_dir(&context.current_directory)
           .stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Environment variables'ları ayarla
        for (key, value) in &context.environment_vars {
            cmd.env(key, value);
        }

        let child = cmd.spawn()?;

        Ok(InteractiveSession {
            child,
            session_id: Uuid::new_v4(),
            command: command.to_string(),
            start_time: Instant::now(),
        })
    }

    pub fn validate_command(&self, command: &str) -> CommandValidation {
        let command = command.trim();
        
        if command.is_empty() {
            return CommandValidation {
                is_valid: false,
                error: Some("Boş komut".to_string()),
                suggestions: vec!["Bir komut yazın".to_string()],
            };
        }

        // Temel syntax kontrolü
        if command.chars().filter(|&c| c == '"').count() % 2 != 0 {
            return CommandValidation {
                is_valid: false,
                error: Some("Eşleşmeyen tırnak işareti".to_string()),
                suggestions: vec!["Tırnak işaretlerini kontrol edin".to_string()],
            };
        }

        if command.chars().filter(|&c| c == '(').count() != command.chars().filter(|&c| c == ')').count() {
            return CommandValidation {
                is_valid: false,
                error: Some("Eşleşmeyen parantez".to_string()),
                suggestions: vec!["Parantezleri kontrol edin".to_string()],
            };
        }

        // Pipe kontrolü
        if command.contains("||") && !command.contains("&&") {
            return CommandValidation {
                is_valid: true,
                error: None,
                suggestions: vec!["|| operatörü kullanıyorsunuz, hata durumunda çalışacak".to_string()],
            };
        }

        CommandValidation {
            is_valid: true,
            error: None,
            suggestions: vec![],
        }
    }

    pub async fn get_command_completion(
        &self,
        partial_command: &str,
        context: &TerminalContext,
    ) -> Result<Vec<String>> {
        // Bash completion kullanarak önerileri al
        let completion_command = format!(
            r#"
            compgen -c {} 2>/dev/null | head -20
            "#,
            partial_command
        );

        let result = self.execute_command(
            &completion_command,
            context,
            SafetyLevel::Safe,
        ).await?;

        if result.exit_code == 0 {
            let completions: Vec<String> = result.output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string())
                .collect();
            
            Ok(completions)
        } else {
            Ok(vec![])
        }
    }
}

pub struct InteractiveSession {
    pub child: tokio::process::Child,
    pub session_id: Uuid,
    pub command: String,
    pub start_time: Instant,
}

impl InteractiveSession {
    pub async fn send_input(&mut self, input: &str) -> Result<()> {
        if let Some(stdin) = self.child.stdin.as_mut() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(input.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }
        Ok(())
    }

    pub async fn read_output(&mut self) -> Result<String> {
        if let Some(stdout) = self.child.stdout.as_mut() {
            let mut reader = BufReader::new(stdout);
            let mut output = String::new();
            
            // Timeout ile oku
            match tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut output)).await {
                Ok(Ok(_)) => Ok(output),
                Ok(Err(e)) => Err(e.into()),
                Err(_) => Ok(String::new()), // Timeout
            }
        } else {
            Ok(String::new())
        }
    }

    pub async fn terminate(mut self) -> Result<CommandExecutionResult> {
        let _ = self.child.kill().await;
        let status = self.child.wait().await?;
        let execution_time = self.start_time.elapsed();

        Ok(CommandExecutionResult {
            command: self.command,
            output: "Interactive session terminated".to_string(),
            error: None,
            exit_code: status.code().unwrap_or(-1),
            execution_time_ms: execution_time.as_millis() as u64,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CommandValidation {
    pub is_valid: bool,
    pub error: Option<String>,
    pub suggestions: Vec<String>,
}

impl Default for ShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}