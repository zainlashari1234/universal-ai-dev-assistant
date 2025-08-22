// Global type definitions for Universal AI Development Assistant

declare global {
  interface RequestInit {
    method?: string;
    headers?: Record<string, string> | Headers;
    body?: string | FormData | Blob | ArrayBuffer;
    mode?: RequestMode;
    credentials?: RequestCredentials;
    cache?: RequestCache;
    redirect?: RequestRedirect;
    referrer?: string;
    referrerPolicy?: ReferrerPolicy;
    integrity?: string;
    keepalive?: boolean;
    signal?: AbortSignal;
  }

  interface Window {
    UAIDAClient?: any;
  }
}

export {};

// VS Code Extension Types
export interface VSCodeCommand {
  command: string;
  title: string;
  category?: string;
  icon?: string;
}

export interface TreeItemData {
  label: string;
  id: string;
  children?: TreeItemData[];
  contextValue?: string;
  iconPath?: string;
}

// Chat Types
export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: Date;
  metadata?: {
    provider?: string;
    tokens_used?: number;
    processing_time_ms?: number;
  };
}

// Analysis Types
export interface CodeAnalysis {
  security_issues: SecurityIssue[];
  performance_suggestions: PerformanceSuggestion[];
  code_quality: CodeQuality;
}

export interface SecurityIssue {
  type: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  message: string;
  line?: number;
  column?: number;
  fix_suggestion?: string;
}

export interface PerformanceSuggestion {
  type: string;
  message: string;
  line?: number;
  column?: number;
  impact: 'low' | 'medium' | 'high';
}

export interface CodeQuality {
  score: number;
  issues: QualityIssue[];
  metrics: {
    complexity: number;
    maintainability: number;
    readability: number;
  };
}

export interface QualityIssue {
  type: string;
  message: string;
  line?: number;
  column?: number;
  severity: 'info' | 'warning' | 'error';
}