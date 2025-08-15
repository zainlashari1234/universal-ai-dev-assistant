import axios, { AxiosInstance } from 'axios';
import * as vscode from 'vscode';

export interface APIResponse<T> {
    success: boolean;
    data: T;
    error?: string;
}

export class UAIDAClient {
    private client: AxiosInstance;
    private baseUrl: string;

    constructor() {
        this.baseUrl = vscode.workspace.getConfiguration('uaida').get('serverUrl', 'http://localhost:8080');
        this.client = axios.create({
            baseURL: this.baseUrl,
            timeout: 30000,
            headers: {
                'Content-Type': 'application/json',
            },
        });

        // Add request interceptor for logging
        this.client.interceptors.request.use(
            (config) => {
                console.log(`UAIDA API Request: ${config.method?.toUpperCase()} ${config.url}`);
                return config;
            },
            (error) => {
                console.error('UAIDA API Request Error:', error);
                return Promise.reject(error);
            }
        );

        // Add response interceptor for error handling
        this.client.interceptors.response.use(
            (response) => {
                console.log(`UAIDA API Response: ${response.status} ${response.config.url}`);
                return response;
            },
            (error) => {
                console.error('UAIDA API Response Error:', error);
                if (error.code === 'ECONNREFUSED') {
                    vscode.window.showErrorMessage(
                        'Cannot connect to UAIDA server. Please ensure the backend is running.',
                        'Open Settings'
                    ).then(selection => {
                        if (selection === 'Open Settings') {
                            vscode.commands.executeCommand('workbench.action.openSettings', 'uaida.serverUrl');
                        }
                    });
                }
                return Promise.reject(error);
            }
        );
    }

    async createPlan(request: any): Promise<APIResponse<any>> {
        try {
            const response = await this.client.post('/api/v1/plan', request);
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    async applyPatch(request: any): Promise<APIResponse<any>> {
        try {
            const response = await this.client.post('/api/v1/patch', request);
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    async runTests(request: any): Promise<APIResponse<any>> {
        try {
            const response = await this.client.post('/api/v1/run-tests', request);
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    async getArtifacts(runId: string): Promise<APIResponse<any>> {
        try {
            const response = await this.client.get(`/api/v1/artifacts/${runId}`);
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    async getRiskReport(patchId: string): Promise<APIResponse<any>> {
        try {
            const response = await this.client.get(`/api/v1/risk-report/${patchId}`);
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    async rollback(request: any): Promise<APIResponse<any>> {
        try {
            const response = await this.client.post('/api/v1/rollback', request);
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    async complete(request: any): Promise<APIResponse<any>> {
        try {
            const response = await this.client.post('/api/v1/complete', request);
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    async analyze(request: any): Promise<APIResponse<any>> {
        try {
            const response = await this.client.post('/api/v1/analyze', request);
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    async healthCheck(): Promise<APIResponse<any>> {
        try {
            const response = await this.client.get('/health');
            return {
                success: true,
                data: response.data
            };
        } catch (error: any) {
            return {
                success: false,
                data: null,
                error: error.response?.data?.error || error.message
            };
        }
    }

    dispose() {
        // Cleanup if needed
    }
}