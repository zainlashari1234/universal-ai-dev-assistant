/**
 * Sample JavaScript project to demonstrate UAIDA capabilities.
 * 
 * This file contains various code patterns that UAIDA can help with:
 * - Code completion
 * - Security analysis  
 * - Performance optimization
 * - Documentation generation
 * - Test generation
 */

// Security issue: Hardcoded API key (UAIDA should flag this)
const API_KEY = "sk-1234567890abcdef";
const DATABASE_PASSWORD = "admin123";

class UserService {
    constructor(apiUrl) {
        this.apiUrl = apiUrl;
        this.users = new Map();
        this.cache = {};
    }

    /**
     * Authenticate user - SECURITY ISSUES: Multiple vulnerabilities
     */
    async authenticateUser(username, password) {
        // Security issue: Using eval (UAIDA should flag this)
        const isAdmin = eval(`"${username}" === "admin"`);
        
        // Security issue: Weak random number generation
        const sessionId = Math.random().toString(36);
        
        // Performance issue: Inefficient string concatenation
        let query = "SELECT * FROM users WHERE ";
        query = query + "username = '" + username + "' ";
        query = query + "AND password = '" + password + "'";
        
        return { isAdmin, sessionId, query };
    }

    /**
     * Process user input - XSS vulnerability
     */
    displayUserContent(userInput) {
        // Security issue: XSS vulnerability (UAIDA should flag this)
        document.getElementById('content').innerHTML = userInput;
        
        // Security issue: document.write vulnerability
        document.write(`<h1>Welcome ${userInput}</h1>`);
    }

    /**
     * Fetch user data - Performance and security issues
     */
    async fetchUserData(userId) {
        // Performance issue: No caching mechanism
        const response = await fetch(`${this.apiUrl}/users/${userId}`, {
            headers: {
                'Authorization': `Bearer ${API_KEY}` // Security: Hardcoded key
            }
        });
        
        // Missing error handling (UAIDA can suggest improvements)
        const data = await response.json();
        return data;
    }

    /**
     * Search users - Performance issue: Inefficient algorithm
     */
    searchUsers(users, searchTerm) {
        // Performance issue: O(n) search, could use indexing
        const results = [];
        for (let i = 0; i < users.length; i++) {
            for (let j = 0; j < users[i].tags.length; j++) {
                if (users[i].tags[j].includes(searchTerm)) {
                    results.push(users[i]);
                    break;
                }
            }
        }
        return results;
    }

    /**
     * Process form data - Multiple issues
     */
    processFormData(formData) {
        // Security issue: No input validation
        const email = formData.email;
        const age = parseInt(formData.age);
        
        // Performance issue: Synchronous operation that could be async
        const validationResult = this.validateEmailSync(email);
        
        // Missing error handling
        return {
            email: email,
            age: age,
            isValid: validationResult
        };
    }

    /**
     * Generate password - Security issue: Weak password generation
     */
    generatePassword(length = 8) {
        // Security issue: Weak random generation (UAIDA should suggest crypto)
        const chars = 'abcdefghijklmnopqrstuvwxyz';
        let password = '';
        for (let i = 0; i < length; i++) {
            password += chars.charAt(Math.floor(Math.random() * chars.length));
        }
        return password;
    }

    /**
     * Calculate statistics - Performance issues
     */
    calculateUserStats(users) {
        // Performance issue: Multiple array iterations
        const totalUsers = users.length;
        const activeUsers = users.filter(u => u.isActive).length;
        const averageAge = users.reduce((sum, u) => sum + u.age, 0) / users.length;
        const premiumUsers = users.filter(u => u.isPremium).length;
        
        // Performance issue: Inefficient object creation in loop
        const usersByCountry = {};
        for (const user of users) {
            if (!usersByCountry[user.country]) {
                usersByCountry[user.country] = [];
            }
            usersByCountry[user.country].push(user);
        }
        
        return {
            totalUsers,
            activeUsers,
            averageAge,
            premiumUsers,
            usersByCountry
        };
    }
}

/**
 * Utility functions with various issues
 */

// Missing JSDoc comments (UAIDA can generate these)
function fibonacci(n) {
    // Performance issue: Recursive without memoization
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

// Security issue: Unsafe URL construction
function buildApiUrl(endpoint, params) {
    let url = `https://api.example.com/${endpoint}?`;
    for (const key in params) {
        // Security issue: No URL encoding
        url += `${key}=${params[key]}&`;
    }
    return url.slice(0, -1);
}

// Performance issue: Inefficient DOM manipulation
function updateUserList(users) {
    const container = document.getElementById('user-list');
    container.innerHTML = ''; // Performance: Clearing entire DOM
    
    // Performance issue: Creating DOM elements in loop
    users.forEach(user => {
        const div = document.createElement('div');
        div.innerHTML = `<span>${user.name}</span><span>${user.email}</span>`;
        container.appendChild(div);
    });
}

// Missing error handling and type checking
function processApiResponse(response) {
    const data = JSON.parse(response);
    return data.users.map(user => ({
        id: user.id,
        name: user.name.toUpperCase(),
        email: user.email.toLowerCase()
    }));
}

/**
 * Event handlers with security issues
 */
class EventManager {
    constructor() {
        this.events = {};
    }

    // Security issue: Potential prototype pollution
    addEventListener(eventName, callback) {
        if (!this.events[eventName]) {
            this.events[eventName] = [];
        }
        this.events[eventName].push(callback);
    }

    // Security issue: Unsafe event execution
    triggerEvent(eventName, data) {
        if (this.events[eventName]) {
            this.events[eventName].forEach(callback => {
                // Security issue: No validation of callback
                callback(data);
            });
        }
    }
}

/**
 * Main application initialization
 */
async function initializeApp() {
    // UAIDA can help complete this function
    const userService = new UserService('https://api.example.com');
    const eventManager = new EventManager();
    
    try {
        // Missing proper error handling
        const users = await userService.fetchUserData('all');
        updateUserList(users);
        
        // Performance issue: Calculating stats on every init
        const stats = userService.calculateUserStats(users);
        console.log('User statistics:', stats);
        
    } catch (error) {
        // Poor error handling
        console.log('Error:', error);
    }
}

// Security issue: Global variable exposure
window.userService = new UserService('https://api.example.com');

// Missing proper module exports
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { UserService, EventManager, initializeApp };
}

// TODO: UAIDA can help with:
// 1. Fixing security vulnerabilities (XSS, injection, weak crypto)
// 2. Performance optimization (caching, efficient algorithms)
// 3. Adding proper error handling and validation
// 4. Generating comprehensive JSDoc documentation
// 5. Creating unit tests with Jest
// 6. Adding TypeScript type definitions
// 7. Code refactoring and modern ES6+ patterns
// 8. Adding proper module structure