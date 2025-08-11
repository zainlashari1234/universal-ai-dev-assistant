/*!
 * Sample Rust project to demonstrate UAIDA capabilities.
 * 
 * This file contains various code patterns that UAIDA can help with:
 * - Code completion
 * - Performance optimization
 * - Error handling improvements
 * - Documentation generation
 * - Test generation
 */

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub is_active: bool,
}

#[derive(Debug)]
pub enum UserError {
    NotFound,
    InvalidEmail,
    DatabaseError(String),
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::NotFound => write!(f, "User not found"),
            UserError::InvalidEmail => write!(f, "Invalid email format"),
            UserError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for UserError {}

pub struct UserManager {
    users: HashMap<u64, User>,
    next_id: u64,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }

    /// Create a new user - UAIDA can suggest improvements
    pub fn create_user(&mut self, username: String, email: String) -> Result<User, UserError> {
        // UAIDA might suggest email validation here
        if !email.contains('@') {
            return Err(UserError::InvalidEmail);
        }

        let user = User {
            id: self.next_id,
            username,
            email,
            is_active: true,
        };

        self.users.insert(self.next_id, user.clone());
        self.next_id += 1;
        Ok(user)
    }

    /// Get user by ID
    pub fn get_user(&self, id: u64) -> Result<&User, UserError> {
        self.users.get(&id).ok_or(UserError::NotFound)
    }

    /// Inefficient search function - PERFORMANCE ISSUE
    pub fn find_user_by_email(&self, email: &str) -> Option<&User> {
        // Performance issue: O(n) search when we could maintain an email index
        for user in self.users.values() {
            if user.email == email {
                return Some(user);
            }
        }
        None
    }

    /// Inefficient user statistics - UAIDA can suggest optimizations
    pub fn calculate_stats(&self) -> UserStats {
        let total_users = self.users.len();
        
        // Performance issue: Multiple iterations over the same data
        let active_users = self.users.values().filter(|u| u.is_active).count();
        let inactive_users = self.users.values().filter(|u| !u.is_active).count();
        
        // Performance issue: Collecting into Vec when we just need the count
        let usernames: Vec<String> = self.users.values().map(|u| u.username.clone()).collect();
        let avg_username_length = usernames.iter().map(|s| s.len()).sum::<usize>() as f64 / usernames.len() as f64;

        UserStats {
            total_users,
            active_users,
            inactive_users,
            avg_username_length,
        }
    }
}

#[derive(Debug)]
pub struct UserStats {
    pub total_users: usize,
    pub active_users: usize,
    pub inactive_users: usize,
    pub avg_username_length: f64,
}

/// File operations with poor error handling
pub struct FileManager;

impl FileManager {
    /// Read file with poor error handling - UAIDA can suggest improvements
    pub fn read_user_data(filename: &str) -> String {
        // Error handling issue: Using unwrap() instead of proper error handling
        let mut file = File::open(filename).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    }

    /// Write file with blocking I/O - PERFORMANCE ISSUE
    pub fn save_user_data(filename: &str, data: &str) -> io::Result<()> {
        // Performance issue: Synchronous I/O in what could be async context
        let mut file = File::create(filename)?;
        file.write_all(data.as_bytes())?;
        file.sync_all()?; // Performance: Unnecessary sync for every write
        Ok(())
    }

    /// Process multiple files inefficiently
    pub fn process_files(filenames: Vec<&str>) -> Vec<String> {
        let mut results = Vec::new();
        
        // Performance issue: Sequential processing instead of parallel
        for filename in filenames {
            // Performance issue: Blocking sleep in loop
            thread::sleep(Duration::from_millis(100));
            
            match File::open(filename) {
                Ok(mut file) => {
                    let mut content = String::new();
                    if file.read_to_string(&mut content).is_ok() {
                        results.push(content);
                    }
                }
                Err(_) => {
                    // Poor error handling: Silently ignoring errors
                    results.push(String::new());
                }
            }
        }
        
        results
    }
}

/// Mathematical functions with performance issues
pub mod math {
    /// Fibonacci with no memoization - PERFORMANCE ISSUE
    pub fn fibonacci(n: u64) -> u64 {
        // Performance issue: Exponential time complexity
        match n {
            0 => 0,
            1 => 1,
            _ => fibonacci(n - 1) + fibonacci(n - 2),
        }
    }

    /// Prime checking with inefficient algorithm
    pub fn is_prime(n: u64) -> bool {
        if n < 2 {
            return false;
        }
        
        // Performance issue: Checking all numbers instead of up to sqrt(n)
        for i in 2..n {
            if n % i == 0 {
                return false;
            }
        }
        true
    }

    /// Sum calculation that could be optimized
    pub fn sum_of_squares(numbers: &[i32]) -> i64 {
        // Performance issue: Could use iterator methods more efficiently
        let mut sum = 0i64;
        for &num in numbers {
            sum += (num as i64) * (num as i64);
        }
        sum
    }
}

/// Network operations with poor error handling
pub mod network {
    use std::net::TcpStream;
    use std::io::Write;

    /// Connect to server with poor error handling
    pub fn connect_to_server(address: &str) -> TcpStream {
        // Error handling issue: Using unwrap() for network operations
        TcpStream::connect(address).unwrap()
    }

    /// Send data without proper error handling
    pub fn send_data(stream: &mut TcpStream, data: &[u8]) {
        // Error handling issue: Ignoring write errors
        let _ = stream.write_all(data);
    }
}

/// Configuration management with hardcoded values
pub struct Config {
    pub database_url: String,
    pub api_key: String,
    pub max_connections: u32,
}

impl Config {
    /// Load configuration with hardcoded values - SECURITY ISSUE
    pub fn load() -> Self {
        // Security issue: Hardcoded credentials (UAIDA should flag this)
        Self {
            database_url: "postgresql://admin:password123@localhost:5432/mydb".to_string(),
            api_key: "sk-1234567890abcdef".to_string(),
            max_connections: 100,
        }
    }
}

/// Main application logic
pub fn run_application() -> Result<(), Box<dyn std::error::Error>> {
    // UAIDA can help complete and improve this function
    let mut user_manager = UserManager::new();
    
    // Create some test users
    let user1 = user_manager.create_user("alice".to_string(), "alice@example.com".to_string())?;
    let user2 = user_manager.create_user("bob".to_string(), "bob@example.com".to_string())?;
    
    println!("Created users: {:?}, {:?}", user1, user2);
    
    // Calculate statistics
    let stats = user_manager.calculate_stats();
    println!("User statistics: {:?}", stats);
    
    // Test mathematical functions
    let fib_result = math::fibonacci(10);
    println!("Fibonacci(10) = {}", fib_result);
    
    // Load configuration
    let config = Config::load();
    println!("Loaded config with {} max connections", config.max_connections);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let mut manager = UserManager::new();
        let user = manager.create_user("test".to_string(), "test@example.com".to_string());
        assert!(user.is_ok());
    }

    #[test]
    fn test_invalid_email() {
        let mut manager = UserManager::new();
        let user = manager.create_user("test".to_string(), "invalid-email".to_string());
        assert!(user.is_err());
    }

    // UAIDA can generate more comprehensive tests
}

// TODO: UAIDA can help with:
// 1. Performance optimization (memoization, parallel processing, efficient algorithms)
// 2. Better error handling (removing unwrap(), proper error propagation)
// 3. Security improvements (removing hardcoded credentials)
// 4. Adding comprehensive documentation
// 5. Generating more unit tests
// 6. Async/await implementation for I/O operations
// 7. Memory optimization and lifetime management
// 8. Code refactoring and modern Rust patterns