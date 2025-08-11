import axios, { AxiosInstance } from 'axios';
import * as vscode from 'vscode';

export interface CompletionRequest {
    code: string;
    language: string;
    cursor_position: number;
    context?: string;
}

export interface CompletionResponse {
    suggestions: string[];
    confidence: number;
    processing_time_ms: number;
}

export interface HealthResponse {
    status: string;
    version: string;
    ai_model_loaded: boolean;
    supported_languages: string[];
}

export class UAIDAClient {
    private client: AxiosInstance;
    private serverUrl: string;

    constructor() {
        this.updateConfiguration();
        this.client = axios.create({
            timeout: 10000,
            headers: {
                'Content-Type': 'application/json'
            }
        });
    }

    updateConfiguration() {
        const config = vscode.workspace.getConfiguration('uaida');
        this.serverUrl = config.get('serverUrl', 'http://127.0.0.1:8080');
        
        if (this.client) {
            this.client.defaults.baseURL = this.serverUrl;
        }
    }

    async checkHealth(): Promise<boolean> {
        try {
            const response = await this.client.get<HealthResponse>('/health');
            return response.data.status === 'healthy';
        } catch (error) {
            console.error('Health check failed:', error);
            return false;
        }
    }

    async getCompletion(request: CompletionRequest): Promise<string[]> {
        try {
            const response = await this.client.post<CompletionResponse>('/api/v1/complete', request);
            return response.data.suggestions;
        } catch (error) {
            console.error('Completion request failed:', error);
            throw new Error(`Failed to get completion: ${error}`);
        }
    }

    async analyzeCode(request: CompletionRequest): Promise<any> {
        try {
            const response = await this.client.post('/api/v1/analyze', request);
            return response.data;
        } catch (error) {
            console.error('Analysis request failed:', error);
            throw new Error(`Failed to analyze code: ${error}`);
        }
    }

    async getServerInfo(): Promise<HealthResponse | null> {
        try {
            const response = await this.client.get<HealthResponse>('/health');
            return response.data;
        } catch (error) {
            console.error('Failed to get server info:', error);
            return null;
        }
    }

    dispose() {
        // Cleanup if needed
    }
}