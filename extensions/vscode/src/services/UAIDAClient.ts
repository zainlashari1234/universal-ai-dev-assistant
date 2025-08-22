// VS Code Extension UAIDA Client

export interface CompletionRequest {
  prompt: string;
  language: string;
  max_tokens?: number;
  temperature?: number;
  provider?: string;
}

export interface CompletionResponse {
  suggestions: Array<{
    text: string;
    confidence: number;
    provider: string;
    metadata?: {
      tokens_used?: number;
      processing_time_ms?: number;
    };
  }>;
  request_id: string;
}

export interface AnalysisRequest {
  code: string;
  language: string;
  analysis_types?: string[];
}

export interface AnalysisResponse {
  security_issues: Array<{
    type: string;
    severity: string;
    message: string;
    line?: number;
  }>;
  performance_suggestions: Array<{
    type: string;
    message: string;
    line?: number;
  }>;
  code_quality: {
    score: number;
    issues: Array<{
      type: string;
      message: string;
      line?: number;
    }>;
  };
}

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  metadata?: {
    provider?: string;
    tokens_used?: number;
  };
}

export class UAIDAClient {
  private baseUrl: string;
  private apiKey?: string;

  constructor(baseUrl: string = 'http://localhost:8080', apiKey?: string) {
    this.baseUrl = baseUrl;
    this.apiKey = apiKey;
  }

  private async makeRequest(endpoint: string, options: RequestInit = {}): Promise<Response> {
    const url = `${this.baseUrl}${endpoint}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...((options.headers as Record<string, string>) || {}),
    };

    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    const response = await fetch(url, {
      ...options,
      headers,
    });

    return response;
  }

  async getCompletion(request: CompletionRequest): Promise<CompletionResponse> {
    const response = await this.makeRequest('/api/v1/complete', {
      method: 'POST',
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Completion request failed: ${response.statusText}`);
    }

    return response.json();
  }

  async analyzeCode(request: AnalysisRequest): Promise<AnalysisResponse> {
    const response = await this.makeRequest('/api/v1/analyze', {
      method: 'POST',
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Analysis request failed: ${response.statusText}`);
    }

    return response.json();
  }

  async sendChatMessage(message: string, context?: string): Promise<ChatMessage> {
    const response = await this.makeRequest('/api/v1/chat', {
      method: 'POST',
      body: JSON.stringify({
        message,
        context,
      }),
    });

    if (!response.ok) {
      throw new Error(`Chat request failed: ${response.statusText}`);
    }

    const data = await response.json();
    return {
      role: 'assistant',
      content: data.response,
      timestamp: new Date(),
      metadata: data.metadata,
    };
  }

  async searchCode(query: string, filters?: {
    language?: string;
    file_path?: string;
    limit?: number;
  }): Promise<Array<{
    file_path: string;
    line_number: number;
    content: string;
    score: number;
  }>> {
    const response = await this.makeRequest('/api/v1/search', {
      method: 'POST',
      body: JSON.stringify({
        query,
        ...filters,
      }),
    });

    if (!response.ok) {
      throw new Error(`Search request failed: ${response.statusText}`);
    }

    const data = await response.json();
    return data.results;
  }

  async getHealth(): Promise<{
    status: string;
    version: string;
    uptime: number;
  }> {
    const response = await this.makeRequest('/health');

    if (!response.ok) {
      throw new Error(`Health check failed: ${response.statusText}`);
    }

    return response.json();
  }
}