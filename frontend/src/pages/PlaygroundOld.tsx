import React, { useState, useEffect } from 'react';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';

interface CompletionRequest {
  prompt: string;
  language?: string;
  model?: string;
  provider?: string;
  max_tokens?: number;
  temperature?: number;
}

interface CompletionResponse {
  id: string;
  text: string;
  model: string;
  provider: string;
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
    cost_usd?: number;
  };
}

const Playground: React.FC = () => {
  const [code, setCode] = useState('// Enter your code here\nfunction fibonacci(n) {\n  \n}');
  const [language, setLanguage] = useState('javascript');
  const [provider, setProvider] = useState('auto');
  const [model, setModel] = useState('');
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(1000);
  const [result, setResult] = useState<CompletionResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const languages = [
    { value: 'javascript', label: 'JavaScript' },
    { value: 'typescript', label: 'TypeScript' },
    { value: 'python', label: 'Python' },
    { value: 'rust', label: 'Rust' },
    { value: 'go', label: 'Go' },
    { value: 'java', label: 'Java' },
    { value: 'cpp', label: 'C++' },
    { value: 'c', label: 'C' },
  ];

  const providers = [
    { value: 'auto', label: 'Auto (Best Available)' },
    { value: 'openrouter', label: 'OpenRouter' },
    { value: 'openai', label: 'OpenAI' },
    { value: 'anthropic', label: 'Anthropic' },
    { value: 'google', label: 'Google' },
    { value: 'groq', label: 'Groq' },
    { value: 'ollama', label: 'Ollama' },
  ];

  const handleComplete = async () => {
    setLoading(true);
    setError(null);

    try {
      const request: CompletionRequest = {
        prompt: code,
        language,
        provider: provider === 'auto' ? undefined : provider,
        model: model || undefined,
        max_tokens: maxTokens,
        temperature,
      };

      const response = await fetch('/api/v1/complete', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data: CompletionResponse = await response.json();
      setResult(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred');
    } finally {
      setLoading(false);
    }
  };

  const handleInsertCompletion = () => {
    if (result) {
      setCode(code + '\n' + result.text);
      setResult(null);
    }
  };

  return (
    <div className="min-h-screen bg-gray-50 py-8">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-gray-900">AI Code Playground</h1>
          <p className="mt-2 text-gray-600">
            Experiment with AI-powered code completion using multiple providers
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Input Section */}
          <div className="space-y-6">
            <div className="bg-white rounded-lg shadow p-6">
              <h2 className="text-xl font-semibold mb-4">Code Input</h2>
              
              {/* Settings */}
              <div className="grid grid-cols-2 gap-4 mb-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Language
                  </label>
                  <select
                    value={language}
                    onChange={(e) => setLanguage(e.target.value)}
                    className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  >
                    {languages.map((lang) => (
                      <option key={lang.value} value={lang.value}>
                        {lang.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Provider
                  </label>
                  <select
                    value={provider}
                    onChange={(e) => setProvider(e.target.value)}
                    className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  >
                    {providers.map((prov) => (
                      <option key={prov.value} value={prov.value}>
                        {prov.label}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4 mb-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Model (optional)
                  </label>
                  <input
                    type="text"
                    value={model}
                    onChange={(e) => setModel(e.target.value)}
                    placeholder="e.g., gpt-4o"
                    className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Temperature ({temperature})
                  </label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    value={temperature}
                    onChange={(e) => setTemperature(parseFloat(e.target.value))}
                    className="w-full"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Max Tokens
                  </label>
                  <input
                    type="number"
                    value={maxTokens}
                    onChange={(e) => setMaxTokens(parseInt(e.target.value))}
                    min="1"
                    max="4000"
                    className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  />
                </div>
              </div>

              {/* Code Editor */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Code
                </label>
                <textarea
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                  rows={15}
                  className="w-full border border-gray-300 rounded-md px-3 py-2 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="Enter your code here..."
                />
              </div>

              {/* Action Buttons */}
              <div className="flex space-x-4 mt-4">
                <button
                  onClick={handleComplete}
                  disabled={loading || !code.trim()}
                  className="flex-1 bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {loading ? 'Generating...' : '🤖 Complete Code'}
                </button>
                
                <button
                  onClick={() => setCode('')}
                  className="px-4 py-2 border border-gray-300 rounded-md hover:bg-gray-50"
                >
                  Clear
                </button>
              </div>
            </div>
          </div>

          {/* Output Section */}
          <div className="space-y-6">
            {error && (
              <div className="bg-red-50 border border-red-200 rounded-lg p-4">
                <div className="flex">
                  <div className="flex-shrink-0">
                    <span className="text-red-400">❌</span>
                  </div>
                  <div className="ml-3">
                    <h3 className="text-sm font-medium text-red-800">Error</h3>
                    <div className="mt-2 text-sm text-red-700">{error}</div>
                  </div>
                </div>
              </div>
            )}

            {result && (
              <div className="bg-white rounded-lg shadow p-6">
                <div className="flex justify-between items-center mb-4">
                  <h2 className="text-xl font-semibold">AI Completion</h2>
                  <button
                    onClick={handleInsertCompletion}
                    className="bg-green-600 text-white px-3 py-1 rounded text-sm hover:bg-green-700"
                  >
                    Insert into Code
                  </button>
                </div>

                {/* Completion Info */}
                <div className="grid grid-cols-2 gap-4 mb-4 text-sm">
                  <div>
                    <span className="font-medium">Provider:</span> {result.provider}
                  </div>
                  <div>
                    <span className="font-medium">Model:</span> {result.model}
                  </div>
                  {result.usage && (
                    <>
                      <div>
                        <span className="font-medium">Tokens:</span> {result.usage.total_tokens}
                      </div>
                      {result.usage.cost_usd && (
                        <div>
                          <span className="font-medium">Cost:</span> ${result.usage.cost_usd.toFixed(6)}
                        </div>
                      )}
                    </>
                  )}
                </div>

                {/* Completion Result */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    Generated Code
                  </label>
                  <div className="border border-gray-300 rounded-md overflow-hidden">
                    <SyntaxHighlighter
                      language={language}
                      style={vscDarkPlus}
                      customStyle={{
                        margin: 0,
                        fontSize: '14px',
                      }}
                    >
                      {result.text}
                    </SyntaxHighlighter>
                  </div>
                </div>
              </div>
            )}

            {/* Help Section */}
            <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
              <h3 className="text-lg font-medium text-blue-900 mb-2">💡 Tips</h3>
              <ul className="text-sm text-blue-800 space-y-1">
                <li>• Start with incomplete functions or classes for best results</li>
                <li>• Use descriptive comments to guide the AI</li>
                <li>• Try different providers to compare results</li>
                <li>• Adjust temperature for more creative (1.0) or focused (0.1) outputs</li>
                <li>• Use specific models for specialized tasks</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default Playground;