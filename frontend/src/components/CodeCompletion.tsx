import React, { useState, useEffect } from 'react';
import { UAIDAClient } from '../services/UAIDAClient';

interface CompletionSuggestion {
  text: string;
  confidence: number;
  provider: string;
  metadata?: {
    tokens_used?: number;
    processing_time_ms?: number;
  };
}

interface CodeCompletionProps {
  code: string;
  language: string;
  onSuggestionSelect: (suggestion: CompletionSuggestion) => void;
}

export const CodeCompletion: React.FC<CodeCompletionProps> = ({
  code,
  language,
  onSuggestionSelect,
}) => {
  const [suggestions, setSuggestions] = useState<CompletionSuggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [client] = useState(() => new UAIDAClient());

  const handleSuggestionSelect = (index: number, suggestion: CompletionSuggestion) => {
    onSuggestionSelect(suggestion);
  };

  const fetchCompletions = async () => {
    if (!code.trim()) return;

    setLoading(true);
    try {
      const response = await client.getCompletion({
        prompt: code,
        language,
        max_tokens: 100,
        temperature: 0.7,
      });
      setSuggestions(response.suggestions);
    } catch (error) {
      console.error('Failed to fetch completions:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    const debounceTimer = setTimeout(() => {
      fetchCompletions();
    }, 500);

    return () => clearTimeout(debounceTimer);
  }, [code, language]);

  return (
    <div className="code-completion">
      <h3>AI Suggestions</h3>
      {loading && <div className="loading">Generating suggestions...</div>}
      {suggestions.length > 0 && (
        <div className="suggestions">
          {suggestions.map((suggestion, index) => (
            <div
              key={index}
              className="suggestion"
              onClick={() => handleSuggestionSelect(index, suggestion)}
            >
              <div className="suggestion-text">{suggestion.text}</div>
              <div className="suggestion-meta">
                <span className="provider">{suggestion.provider}</span>
                <span className="confidence">{Math.round(suggestion.confidence * 100)}%</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};