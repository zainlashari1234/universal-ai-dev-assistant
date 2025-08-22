// Test setup for Universal AI Development Assistant Frontend
// Simple setup without complex dependencies

export const mockUAIDAClient = {
  getCompletion: () => Promise.resolve({ suggestions: [] }),
  analyzeCode: () => Promise.resolve({ security_issues: [], performance_suggestions: [], code_quality: { score: 100, issues: [] } }),
  sendChatMessage: () => Promise.resolve({ role: 'assistant', content: 'Test response', timestamp: new Date() }),
  searchCode: () => Promise.resolve([]),
  getProviders: () => Promise.resolve([]),
  getHealth: () => Promise.resolve({ status: 'healthy', version: '1.0.0', uptime: 0 }),
};

// Basic test utilities
if (typeof global !== 'undefined') {
  (global as any).ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  };

  (global as any).matchMedia = (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => {},
  });
}