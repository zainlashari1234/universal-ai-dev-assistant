#!/usr/bin/env python3
"""
Sample Python project to demonstrate UAIDA capabilities.

This file contains various code patterns that UAIDA can help with:
- Code completion
- Security analysis
- Performance optimization
- Documentation generation
- Test generation
"""

import os
import json
import hashlib
import subprocess
from typing import List, Dict, Optional, Union
from dataclasses import dataclass
from datetime import datetime


@dataclass
class User:
    """User data model."""
    id: int
    username: str
    email: str
    created_at: datetime
    is_active: bool = True


class UserManager:
    """Manages user operations with various code patterns for UAIDA to analyze."""
    
    def __init__(self, database_url: str):
        self.database_url = database_url
        self.users: Dict[int, User] = {}
        
    def create_user(self, username: str, email: str) -> User:
        """Create a new user."""
        # UAIDA can suggest improvements here
        user_id = len(self.users) + 1
        user = User(
            id=user_id,
            username=username,
            email=email,
            created_at=datetime.now()
        )
        self.users[user_id] = user
        return user
    
    def get_user(self, user_id: int) -> Optional[User]:
        """Get user by ID."""
        return self.users.get(user_id)
    
    def authenticate_user(self, username: str, password: str) -> bool:
        """
        Authenticate user - SECURITY ISSUE: This has vulnerabilities
        that UAIDA should detect.
        """
        # Security issue: Hardcoded password (UAIDA should flag this)
        admin_password = "admin123"
        
        # Security issue: Using eval (UAIDA should flag this)
        if username == "admin":
            return eval(f"'{password}' == '{admin_password}'")
        
        # Security issue: SQL injection vulnerability (simulated)
        query = f"SELECT * FROM users WHERE username = '{username}' AND password = '{password}'"
        
        return False
    
    def hash_password(self, password: str) -> str:
        """Hash password - SECURITY ISSUE: Weak hashing."""
        # Security issue: Using MD5 (UAIDA should suggest better alternatives)
        return hashlib.md5(password.encode()).hexdigest()
    
    def execute_command(self, command: str) -> str:
        """
        Execute system command - SECURITY ISSUE: Command injection vulnerability.
        """
        # Security issue: Shell injection (UAIDA should flag this)
        result = subprocess.run(command, shell=True, capture_output=True, text=True)
        return result.stdout
    
    def process_user_data(self, data: str) -> Dict:
        """
        Process user data - SECURITY ISSUE: Unsafe deserialization.
        """
        # Security issue: Using eval for JSON (UAIDA should suggest json.loads)
        return eval(data)
    
    def inefficient_search(self, users: List[User], target_email: str) -> Optional[User]:
        """
        Inefficient search algorithm - PERFORMANCE ISSUE.
        UAIDA should suggest optimization.
        """
        # Performance issue: O(n) search when dict lookup would be O(1)
        for user in users:
            if user.email == target_email:
                return user
        return None
    
    def generate_report(self) -> str:
        """Generate user report - UAIDA can help with documentation."""
        # UAIDA can suggest adding type hints and better documentation
        report = "User Report\n"
        report += "=" * 20 + "\n"
        
        for user_id, user in self.users.items():
            report += f"ID: {user_id}, Username: {user.username}, Email: {user.email}\n"
        
        return report


def fibonacci(n):
    """
    Calculate Fibonacci number - UAIDA can suggest optimizations.
    Missing type hints and inefficient implementation.
    """
    # Performance issue: Recursive implementation without memoization
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)


def process_files(directory: str) -> List[str]:
    """
    Process files in directory - UAIDA can suggest improvements.
    """
    files = []
    # UAIDA might suggest using pathlib or os.walk for better practices
    for filename in os.listdir(directory):
        if filename.endswith('.txt'):
            files.append(filename)
    return files


class DatabaseConnection:
    """Database connection class with potential issues."""
    
    def __init__(self, host: str, port: int, username: str, password: str):
        # Security issue: Storing password in plain text
        self.host = host
        self.port = port
        self.username = username
        self.password = password  # UAIDA should flag this
        
    def connect(self):
        """Connect to database."""
        # Missing error handling - UAIDA can suggest try/catch
        connection_string = f"postgresql://{self.username}:{self.password}@{self.host}:{self.port}"
        print(f"Connecting to: {connection_string}")  # Security issue: Logging credentials


def main():
    """Main function demonstrating various patterns."""
    # UAIDA can help complete this function
    manager = UserManager("postgresql://localhost:5432/users")
    
    # Create some users
    user1 = manager.create_user("alice", "alice@example.com")
    user2 = manager.create_user("bob", "bob@example.com")
    
    # Generate report
    report = manager.generate_report()
    print(report)
    
    # Calculate Fibonacci (inefficient)
    result = fibonacci(10)
    print(f"Fibonacci(10) = {result}")


if __name__ == "__main__":
    main()


# TODO: UAIDA can help with:
# 1. Adding proper error handling
# 2. Improving security (removing hardcoded secrets, fixing SQL injection)
# 3. Performance optimization (memoization, better algorithms)
# 4. Adding comprehensive type hints
# 5. Generating unit tests
# 6. Adding proper logging
# 7. Code refactoring and cleanup