#!/usr/bin/env python3
"""
Test file for the Python fixture.
"""

import pytest
from main import fibonacci, is_prime, factorial, buggy_division

def test_fibonacci():
    """Test fibonacci function."""
    assert fibonacci(0) == 0
    assert fibonacci(1) == 1
    assert fibonacci(10) == 55

def test_is_prime():
    """Test prime checking function."""
    assert is_prime(2) == True
    assert is_prime(17) == True
    assert is_prime(4) == False
    assert is_prime(1) == False

def test_factorial():
    """Test factorial function."""
    assert factorial(0) == 1
    assert factorial(1) == 1
    assert factorial(5) == 120

def test_buggy_division():
    """Test the buggy division function."""
    assert buggy_division(10, 2) == 5.0
    
    # This should fail due to division by zero
    with pytest.raises(ZeroDivisionError):
        buggy_division(10, 0)

if __name__ == "__main__":
    pytest.main([__file__])