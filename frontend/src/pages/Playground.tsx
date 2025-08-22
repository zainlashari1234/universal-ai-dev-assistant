import React, { useState, useEffect } from 'react';
import { 
  PlayIcon, 
  StopIcon, 
  DocumentDuplicateIcon,
  AdjustmentsHorizontalIcon,
  ChartBarIcon,
  CpuChipIcon,
  BoltIcon
} from '@heroicons/react/24/outline';
import { StreamingCompletion } from '../components/StreamingCompletion';

const Playground: React.FC = () => {
  const [prompt, setPrompt] = useState('');
  const [systemPrompt, setSystemPrompt] = useState('');
  const [provider, setProvider] = useState('openrouter');
  const [model, setModel] = useState('gpt-4o-mini');
  const [language, setLanguage] = useState('javascript');
  const [maxTokens, setMaxTokens] = useState(1000);
  const [temperature, setTemperature] = useState(0.7);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState('');
  const [error, setError] = useState('');
  const [useStreaming, setUseStreaming] = useState(true);
  const [streamingResult, setStreamingResult] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);

  const providers = [
    { id: 'openrouter', name: 'OpenRouter', models: ['gpt-4o', 'gpt-4o-mini', 'claude-3-sonnet', 'llama-3.1-70b'] },
    { id: 'openai', name: 'OpenAI', models: ['gpt-4o', 'gpt-4o-mini', 'gpt-3.5-turbo'] },
    { id: 'anthropic', name: 'Anthropic', models: ['claude-3-sonnet', 'claude-3-haiku'] },
    { id: 'google', name: 'Google', models: ['gemini-pro', 'gemini-flash'] },
  ];

  const languages = [
    'javascript', 'typescript', 'python', 'rust', 'go', 'java', 'cpp', 'csharp', 'php', 'ruby'
  ];

  const currentProvider = providers.find(p => p.id === provider);

  const handleSubmit = async () => {
    if (!prompt.trim() || useStreaming) return;

    setLoading(true);
    setError('');
    setResult('');

    try {
      const response = await fetch('/api/completion', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${localStorage.getItem('access_token')}`,
        },
        body: JSON.stringify({
          prompt,
          model,
          provider,
          language,
          max_tokens: maxTokens,
          temperature,
          system_prompt: systemPrompt || undefined,
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to generate completion');
      }

      const data = await response.json();
      setResult(data.response?.content || data.response || 'No response received');
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleStreamingComplete = (result: string, metrics: any) => {
    setStreamingResult(result);
    console.log('Streaming completed with metrics:', metrics);
  };

  const handleStreamingError = (error: string) => {
    setError(error);
  };

  return (
    <div className="max-w-6xl mx-auto space-y-8">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">AI Playground</h1>
        <p className="mt-2 text-gray-600">
          Test and experiment with different AI models and providers in real-time.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Input Panel */}
        <div className="lg:col-span-2 space-y-6">
          {/* Provider and Model Selection */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <h3 className="text-lg font-medium text-gray-900 mb-4">Model Configuration</h3>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  AI Provider
                </label>
                <select
                  value={provider}
                  onChange={(e) => setProvider(e.target.value)}
                  className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  {providers.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Model
                </label>
                <select
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  className="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  {currentProvider?.models.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </select>
              </div>

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
                    <option key={lang} value={lang}>
                      {lang.charAt(0).toUpperCase() + lang.slice(1)}
                    </option>
                  ))}
                </select>
              </div>

              <div className="flex items-center space-x-2">
                <input
                  id="streaming-toggle"
                  type="checkbox"
                  checked={useStreaming}
                  onChange={(e) => setUseStreaming(e.target.checked)}
                  className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label htmlFor="streaming-toggle" className="flex items-center space-x-1 text-sm text-gray-700">
                  <BoltIcon className="h-4 w-4" />
                  <span>Real-time Streaming</span>
                </label>
              </div>
            </div>

            {/* Advanced Settings */}
            <div className="mt-4">
              <button
                onClick={() => setShowAdvanced(!showAdvanced)}
                className="flex items-center space-x-2 text-sm text-gray-600 hover:text-gray-800"
              >
                <AdjustmentsHorizontalIcon className="h-4 w-4" />
                <span>Advanced Settings</span>
              </button>

              {showAdvanced && (
                <div className="mt-4 grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Max Tokens: {maxTokens}
                    </label>
                    <input
                      type="range"
                      min="100"
                      max="4000"
                      step="100"
                      value={maxTokens}
                      onChange={(e) => setMaxTokens(parseInt(e.target.value))}
                      className="w-full"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Temperature: {temperature}
                    </label>
                    <input
                      type="range"
                      min="0"
                      max="2"
                      step="0.1"
                      value={temperature}
                      onChange={(e) => setTemperature(parseFloat(e.target.value))}
                      className="w-full"
                    />
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* System Prompt */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              System Prompt (Optional)
            </label>
            <textarea
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
              placeholder="You are a helpful AI assistant..."
              className="w-full h-24 p-3 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          {/* Main Prompt */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Prompt
            </label>
            <textarea
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="Write a Python function to calculate fibonacci numbers..."
              className="w-full h-32 p-3 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            />

            <div className="flex items-center justify-between mt-4">
              <div className="text-sm text-gray-500">
                {prompt.length} characters
              </div>

              {!useStreaming && (
                <button
                  onClick={handleSubmit}
                  disabled={loading || !prompt.trim()}
                  className="flex items-center space-x-2 bg-blue-600 text-white px-6 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {loading ? (
                    <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                  ) : (
                    <PlayIcon className="h-4 w-4" />
                  )}
                  <span>{loading ? 'Processing...' : 'Generate'}</span>
                </button>
              )}
            </div>
          </div>
        </div>

        {/* Results Panel */}
        <div className="space-y-6">
          {/* Provider Info */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <h3 className="text-lg font-medium text-gray-900 mb-4">Current Setup</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-600">Provider:</span>
                <span className="font-medium">{currentProvider?.name}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Model:</span>
                <span className="font-medium">{model}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Language:</span>
                <span className="font-medium">{language}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-600">Mode:</span>
                <span className="font-medium flex items-center space-x-1">
                  {useStreaming ? (
                    <>
                      <BoltIcon className="h-3 w-3" />
                      <span>Streaming</span>
                    </>
                  ) : (
                    <span>Standard</span>
                  )}
                </span>
              </div>
            </div>
          </div>

          {/* Quick Examples */}
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <h3 className="text-lg font-medium text-gray-900 mb-4">Quick Examples</h3>
            <div className="space-y-2">
              {[
                'Write a React component for a todo list',
                'Create a Python script to analyze CSV data',
                'Generate a REST API endpoint in Node.js',
                'Write unit tests for a calculator function',
              ].map((example, index) => (
                <button
                  key={index}
                  onClick={() => setPrompt(example)}
                  className="w-full text-left text-sm text-gray-600 hover:text-gray-800 hover:bg-gray-50 p-2 rounded"
                >
                  {example}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Results Section */}
      <div className="space-y-6">
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
            {error}
          </div>
        )}

        {/* Streaming Component */}
        {useStreaming && prompt.trim() && (
          <StreamingCompletion
            prompt={prompt}
            provider={provider}
            model={model}
            language={language}
            maxTokens={maxTokens}
            temperature={temperature}
            systemPrompt={systemPrompt}
            onComplete={handleStreamingComplete}
            onError={handleStreamingError}
          />
        )}

        {/* Traditional Result Display */}
        {!useStreaming && result && (
          <div className="bg-white rounded-lg border border-gray-200 p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-medium text-gray-900">AI Response</h3>
              <button
                onClick={() => navigator.clipboard.writeText(result)}
                className="flex items-center space-x-1 text-sm text-gray-500 hover:text-gray-700"
              >
                <DocumentDuplicateIcon className="h-4 w-4" />
                <span>Copy</span>
              </button>
            </div>
            <div className="w-full h-64 p-3 border border-gray-300 rounded-md bg-gray-50 overflow-y-auto">
              <pre className="whitespace-pre-wrap text-sm">{result}</pre>
            </div>
          </div>
        )}

        {/* Streaming Result Copy Option */}
        {useStreaming && streamingResult && (
          <div className="flex justify-end">
            <button
              onClick={() => navigator.clipboard.writeText(streamingResult)}
              className="flex items-center space-x-1 text-sm text-gray-500 hover:text-gray-700"
            >
              <DocumentDuplicateIcon className="h-4 w-4" />
              <span>Copy Streaming Result</span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default Playground;