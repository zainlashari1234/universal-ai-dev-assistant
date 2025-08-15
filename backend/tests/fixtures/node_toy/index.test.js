/**
 * Test file for the Node.js fixture.
 */

const { fibonacci, isPrime, factorial, buggyDivision, fetchData } = require('./index');

describe('Math Functions', () => {
    test('fibonacci should return correct values', () => {
        expect(fibonacci(0)).toBe(0);
        expect(fibonacci(1)).toBe(1);
        expect(fibonacci(10)).toBe(55);
    });

    test('isPrime should correctly identify primes', () => {
        expect(isPrime(2)).toBe(true);
        expect(isPrime(17)).toBe(true);
        expect(isPrime(4)).toBe(false);
        expect(isPrime(1)).toBe(false);
    });

    test('factorial should return correct values', () => {
        expect(factorial(0)).toBe(1);
        expect(factorial(1)).toBe(1);
        expect(factorial(5)).toBe(120);
    });

    test('buggyDivision should work with valid inputs', () => {
        expect(buggyDivision(10, 2)).toBe(5);
    });

    test('buggyDivision should return Infinity with zero divisor', () => {
        expect(buggyDivision(10, 0)).toBe(Infinity);
    });
});

describe('Async Functions', () => {
    test('fetchData should return expected data', async () => {
        const result = await fetchData('https://api.example.com');
        expect(result.data).toBe('Data from https://api.example.com');
    });
});