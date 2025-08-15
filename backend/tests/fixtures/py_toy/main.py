#!/usr/bin/env python3
"""
Simple Python test fixture for sandbox testing.
Contains basic functions for completion and analysis testing.
"""

def fibonacci(n):
    """Calculate the nth Fibonacci number."""
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

def is_prime(num):
    """Check if a number is prime."""
    if num < 2:
        return False
    for i in range(2, int(num ** 0.5) + 1):
        if num % i == 0:
            return False
    return True

def factorial(n):
    """Calculate factorial of n."""
    if n <= 1:
        return 1
    return n * factorial(n - 1)

# Intentional bug for testing
def buggy_division(a, b):
    """Divide a by b - contains intentional bug."""
    return a / b  # Should check for b == 0

def incomplete_function():
    """This function is incomplete for completion testing."""
    # TODO: Complete this function
    pass

if __name__ == "__main__":
    print(f"Fibonacci(10): {fibonacci(10)}")
    print(f"Is 17 prime? {is_prime(17)}")
    print(f"Factorial(5): {factorial(5)}")