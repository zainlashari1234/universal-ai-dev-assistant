/**
 * Simple Node.js test fixture for sandbox testing.
 * Contains basic functions for completion and analysis testing.
 */

function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

function isPrime(num) {
    if (num < 2) return false;
    for (let i = 2; i <= Math.sqrt(num); i++) {
        if (num % i === 0) return false;
    }
    return true;
}

function factorial(n) {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

// Intentional bug for testing
function buggyDivision(a, b) {
    return a / b; // Should check for b === 0
}

function incompleteFunction() {
    // TODO: Complete this function
}

// Async function for testing
async function fetchData(url) {
    // Simulate API call
    return new Promise((resolve) => {
        setTimeout(() => {
            resolve({ data: `Data from ${url}` });
        }, 100);
    });
}

module.exports = {
    fibonacci,
    isPrime,
    factorial,
    buggyDivision,
    incompleteFunction,
    fetchData
};

if (require.main === module) {
    console.log(`Fibonacci(10): ${fibonacci(10)}`);
    console.log(`Is 17 prime? ${isPrime(17)}`);
    console.log(`Factorial(5): ${factorial(5)}`);
}