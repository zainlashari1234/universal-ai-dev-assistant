import axios, { AxiosInstance } from 'axios';

export interface CompletionRequest {
    prompt: string;
    language?: string;
    model?: string;
    provider?: string;
    max_tokens?: number;
    temperature?: number;
    system_prompt?: string;
}

export interface CompletionResponse {
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

export interface AnalysisRequest {
    code: string;
    language: string;
    analysis_type: string;
    context?: string;
}

export interface AnalysisResponse {
    analysis_type: string;
    findings: Array<{
        severity: string;
        category: string;
        title: string;
        description: string;
        line_number?: number;
        column?: number;
        code_snippet?: string;
        fix_suggestion?: string;
    }>;
    summary: string;
    confidence_score: number;
    suggestions: Array<{
        title: string;
        description: string;
        code_example?: string;
        impact: string;
        effort: string;
    }>;
}

export interface CodeActionRequest {
    code: string;
    language: string;
    action: string;
    instructions?: string;
    target_language?: string;
}

export interface CodeActionResponse {
    action: string;
    result: string;
    success: boolean;
}

export interface HealthResponse {
    status: string;
    version: string;
    providers: Record<string, {
        provider_type: string;
        is_available: boolean;
        response_time_ms?: number;
        error_message?: string;
        models_available: string[];
    }>;
    features: string[];
}

export class UAIDAClient {
    private client: AxiosInstance;

    constructor(baseURL: string) {
        this.client = axios.create({
            baseURL,
            timeout: 30000,
            headers: {
                'Content-Type': 'application/json',
            },
        });

        // Add request interceptor for logging
        this.client.interceptors.request.use(
            (config) => {
                console.log(`🚀 UAIDA Request: ${config.method?.toUpperCase()} ${config.url}`);
                return config;
            },
            (error) => {
                console.error('❌ UAIDA Request Error:', error);
                return Promise.reject(error);
            }
        );

        // Add response interceptor for logging
        this.client.interceptors.response.use(
            (response) => {
                console.log(`✅ UAIDA Response: ${response.status} ${response.config.url}`);
                return response;
            },
            (error) => {
                console.error('❌ UAIDA Response Error:', error.response?.status, error.message);
                return Promise.reject(error);
            }
        );
    }

    async health(): Promise<HealthResponse> {
        const response = await this.client.get('/health');
        return response.data;
    }

    async complete(request: CompletionRequest): Promise<CompletionResponse> {
        const response = await this.client.post('/api/v1/complete', request);
        return response.data;
    }

    async analyze(request: AnalysisRequest): Promise<AnalysisResponse> {
        const response = await this.client.post('/api/v1/analyze', request);
        return response.data;
    }

    async codeAction(request: CodeActionRequest): Promise<CodeActionResponse> {
        const response = await this.client.post('/api/v1/code/action', request);
        return response.data;
    }

    async providers(): Promise<any> {
        const response = await this.client.get('/api/v1/providers');
        return response.data;
    }

    async models(): Promise<any> {
        const response = await this.client.get('/api/v1/models');
        return response.data;
    }

    async metrics(): Promise<any> {
        const response = await this.client.get('/api/v1/metrics');
        return response.data;
    }
}