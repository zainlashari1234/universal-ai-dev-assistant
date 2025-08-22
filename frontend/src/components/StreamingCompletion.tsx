import React, { useState, useEffect, useRef } from 'react';
import { 
  PlayIcon, 
  StopIcon, 
  ClockIcon, 
  CurrencyDollarIcon,
  ChartBarIcon,
  ExclamationTriangleIcon,
  CheckCircleIcon
} from '@heroicons/react/24/outline';

interface StreamEvent {
  type: 'start' | 'chunk' | 'progress' | 'metadata' | 'complete' | 'error';
  stream_id: string;
  content?: string;
  percentage?: number;
  tokens_generated?: number;
  estimated_total?: number;
  provider_latency?: number;
  cost_estimate?: number;
  quality_score?: number;
  total_tokens?: number;
  total_cost?: number;
  completion_time?: number;
  quality_metrics?: QualityMetrics;
  error?: string;
  retry_after?: number;
}

interface QualityMetrics {
  coherence_score: number;
  relevance_score: number;
  code_quality_score?: number;
  security_score?: number;
}

interface StreamingCompletionProps {
  prompt: string;
  provider?: string;
  model?: string;
  language?: string;
  maxTokens?: number;
  temperature?: number;
  systemPrompt?: string;
  onComplete?: (result: string, metrics: QualityMetrics) => void;
  onError?: (error: string) => void;
}

export const StreamingCompletion: React.FC<StreamingCompletionProps> = ({
  prompt,
  provider = 'openrouter',
  model = 'gpt-4o-mini',
  language,
  maxTokens = 1000,
  temperature = 0.7,
  systemPrompt,
  onComplete,
  onError
}) => {
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamedContent, setStreamedContent] = useState('');
  const [progress, setProgress] = useState(0);
  const [tokensGenerated, setTokensGenerated] = useState(0);
  const [estimatedTotal, setEstimatedTotal] = useState<number | null>(null);
  const [currentLatency, setCurrentLatency] = useState<number | null>(null);
  const [costEstimate, setCostEstimate] = useState<number | null>(null);
  const [qualityScore, setQualityScore] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [finalMetrics, setFinalMetrics] = useState<QualityMetrics | null>(null);
  const [streamId, setStreamId] = useState<string | null>(null);
  
  const eventSourceRef = useRef<EventSource | null>(null);
  const contentRef = useRef<HTMLDivElement>(null);

  const startStreaming = async () => {
    if (isStreaming) return;

    setIsStreaming(true);
    setStreamedContent('');
    setProgress(0);
    setTokensGenerated(0);
    setError(null);
    setFinalMetrics(null);

    try {
      // Create streaming request
      const streamingRequest = {
        prompt,
        provider,
        model,
        language,
        max_tokens: maxTokens,
        temperature,
        system_prompt: systemPrompt,
        stream_id: `stream_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
      };

      setStreamId(streamingRequest.stream_id);

      // Start SSE connection
      const eventSource = new EventSource(
        `/api/completion/stream?${new URLSearchParams({
          data: JSON.stringify(streamingRequest)
        })}`
      );

      eventSourceRef.current = eventSource;

      eventSource.onopen = () => {
        console.log('Streaming connection opened');
      };

      eventSource.addEventListener('start', (event) => {
        const data: StreamEvent = JSON.parse(event.data);
        console.log('Stream started:', data);
        setEstimatedTotal(data.estimated_total || null);
      });

      eventSource.addEventListener('chunk', (event) => {
        const data: StreamEvent = JSON.parse(event.data);
        setStreamedContent(prev => prev + (data.content || ''));
        setTokensGenerated(data.tokens_generated || 0);
        
        // Auto-scroll to bottom
        if (contentRef.current) {
          contentRef.current.scrollTop = contentRef.current.scrollHeight;
        }
      });

      eventSource.addEventListener('progress', (event) => {
        const data: StreamEvent = JSON.parse(event.data);
        setProgress(data.percentage || 0);
        setTokensGenerated(data.tokens_generated || 0);
        setEstimatedTotal(data.estimated_total || null);
      });

      eventSource.addEventListener('metadata', (event) => {
        const data: StreamEvent = JSON.parse(event.data);
        setCurrentLatency(data.provider_latency || null);
        setCostEstimate(data.cost_estimate || null);
        setQualityScore(data.quality_score || null);
      });

      eventSource.addEventListener('complete', (event) => {
        const data: StreamEvent = JSON.parse(event.data);
        setFinalMetrics(data.quality_metrics || null);
        setProgress(100);
        setIsStreaming(false);
        
        if (onComplete && data.quality_metrics) {
          onComplete(streamedContent, data.quality_metrics);
        }
        
        eventSource.close();
      });

      eventSource.addEventListener('error', (event) => {
        const data: StreamEvent = JSON.parse(event.data);
        setError(data.error || 'Unknown streaming error');
        setIsStreaming(false);
        
        if (onError) {
          onError(data.error || 'Unknown streaming error');
        }
        
        eventSource.close();
      });

      eventSource.onerror = (event) => {
        console.error('EventSource failed:', event);
        setError('Connection error occurred');
        setIsStreaming(false);
        eventSource.close();
      };

    } catch (err) {
      console.error('Failed to start streaming:', err);
      setError('Failed to start streaming');
      setIsStreaming(false);
    }
  };

  const stopStreaming = () => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }
    setIsStreaming(false);
  };

  useEffect(() => {
    return () => {
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
    };
  }, []);

  const formatCost = (cost: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 4,
      maximumFractionDigits: 6
    }).format(cost);
  };

  const getQualityColor = (score: number) => {
    if (score >= 0.9) return 'text-green-600';
    if (score >= 0.7) return 'text-yellow-600';
    return 'text-red-600';
  };

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-medium text-gray-900">AI Streaming Completion</h3>
        <div className="flex items-center space-x-2">
          {!isStreaming ? (
            <button
              onClick={startStreaming}
              disabled={!prompt.trim()}
              className="flex items-center space-x-2 bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <PlayIcon className="h-4 w-4" />
              <span>Start</span>
            </button>
          ) : (
            <button
              onClick={stopStreaming}
              className="flex items-center space-x-2 bg-red-600 text-white px-4 py-2 rounded-md hover:bg-red-700"
            >
              <StopIcon className="h-4 w-4" />
              <span>Stop</span>
            </button>
          )}
        </div>
      </div>

      {/* Progress Bar */}
      {isStreaming && (
        <div className="mb-4">
          <div className="flex items-center justify-between text-sm text-gray-600 mb-1">
            <span>Progress</span>
            <span>{progress.toFixed(1)}%</span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2">
            <div
              className="bg-blue-600 h-2 rounded-full transition-all duration-300"
              style={{ width: `${progress}%` }}
            ></div>
          </div>
        </div>
      )}

      {/* Real-time Metrics */}
      {isStreaming && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
          <div className="bg-gray-50 p-3 rounded-lg">
            <div className="flex items-center space-x-2">
              <ChartBarIcon className="h-4 w-4 text-gray-500" />
              <span className="text-sm text-gray-600">Tokens</span>
            </div>
            <p className="text-lg font-semibold text-gray-900">
              {tokensGenerated}{estimatedTotal && ` / ${estimatedTotal}`}
            </p>
          </div>

          {currentLatency && (
            <div className="bg-gray-50 p-3 rounded-lg">
              <div className="flex items-center space-x-2">
                <ClockIcon className="h-4 w-4 text-gray-500" />
                <span className="text-sm text-gray-600">Latency</span>
              </div>
              <p className="text-lg font-semibold text-gray-900">{currentLatency}ms</p>
            </div>
          )}

          {costEstimate && (
            <div className="bg-gray-50 p-3 rounded-lg">
              <div className="flex items-center space-x-2">
                <CurrencyDollarIcon className="h-4 w-4 text-gray-500" />
                <span className="text-sm text-gray-600">Cost</span>
              </div>
              <p className="text-lg font-semibold text-gray-900">{formatCost(costEstimate)}</p>
            </div>
          )}

          {qualityScore && (
            <div className="bg-gray-50 p-3 rounded-lg">
              <div className="flex items-center space-x-2">
                <CheckCircleIcon className="h-4 w-4 text-gray-500" />
                <span className="text-sm text-gray-600">Quality</span>
              </div>
              <p className={`text-lg font-semibold ${getQualityColor(qualityScore)}`}>
                {(qualityScore * 100).toFixed(1)}%
              </p>
            </div>
          )}
        </div>
      )}

      {/* Streamed Content */}
      <div className="mb-4">
        <label className="block text-sm font-medium text-gray-700 mb-2">
          AI Response {streamId && <span className="text-xs text-gray-500">({streamId})</span>}
        </label>
        <div
          ref={contentRef}
          className="w-full h-64 p-3 border border-gray-300 rounded-md bg-gray-50 overflow-y-auto font-mono text-sm"
        >
          {streamedContent && (
            <pre className="whitespace-pre-wrap">{streamedContent}</pre>
          )}
          {isStreaming && (
            <span className="inline-block w-2 h-4 bg-blue-600 animate-pulse ml-1"></span>
          )}
          {!streamedContent && !isStreaming && (
            <p className="text-gray-500 italic">AI response will appear here...</p>
          )}
        </div>
      </div>

      {/* Error Display */}
      {error && (
        <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-md">
          <div className="flex items-center space-x-2">
            <ExclamationTriangleIcon className="h-5 w-5 text-red-600" />
            <span className="text-sm font-medium text-red-800">Error</span>
          </div>
          <p className="text-sm text-red-700 mt-1">{error}</p>
        </div>
      )}

      {/* Final Quality Metrics */}
      {finalMetrics && (
        <div className="bg-green-50 border border-green-200 rounded-md p-4">
          <h4 className="text-sm font-medium text-green-800 mb-2">Completion Quality Metrics</h4>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            <div>
              <span className="text-xs text-green-600">Coherence</span>
              <p className={`text-sm font-semibold ${getQualityColor(finalMetrics.coherence_score)}`}>
                {(finalMetrics.coherence_score * 100).toFixed(1)}%
              </p>
            </div>
            <div>
              <span className="text-xs text-green-600">Relevance</span>
              <p className={`text-sm font-semibold ${getQualityColor(finalMetrics.relevance_score)}`}>
                {(finalMetrics.relevance_score * 100).toFixed(1)}%
              </p>
            </div>
            {finalMetrics.code_quality_score && (
              <div>
                <span className="text-xs text-green-600">Code Quality</span>
                <p className={`text-sm font-semibold ${getQualityColor(finalMetrics.code_quality_score)}`}>
                  {(finalMetrics.code_quality_score * 100).toFixed(1)}%
                </p>
              </div>
            )}
            {finalMetrics.security_score && (
              <div>
                <span className="text-xs text-green-600">Security</span>
                <p className={`text-sm font-semibold ${getQualityColor(finalMetrics.security_score)}`}>
                  {(finalMetrics.security_score * 100).toFixed(1)}%
                </p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};