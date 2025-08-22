import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import axios from 'axios';

interface User {
  id: string;
  email: string;
  username: string;
  full_name?: string;
  is_active: boolean;
  is_verified: boolean;
  created_at: string;
}

interface AuthTokens {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
}

interface AuthContextType {
  user: User | null;
  tokens: AuthTokens | null;
  login: (email: string, password: string) => Promise<void>;
  register: (email: string, username: string, password: string, fullName?: string) => Promise<void>;
  logout: () => void;
  refreshToken: () => Promise<void>;
  isAuthenticated: boolean;
  loading: boolean;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

// Configure axios defaults
const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:3001';
axios.defaults.baseURL = API_BASE_URL;

// Add request interceptor to include auth token
axios.interceptors.request.use(
  (config) => {
    const tokens = localStorage.getItem('auth_tokens');
    if (tokens) {
      const parsedTokens = JSON.parse(tokens);
      config.headers.Authorization = `Bearer ${parsedTokens.access_token}`;
    }
    return config;
  },
  (error) => {
    return Promise.reject(error);
  }
);

// Add response interceptor to handle token refresh
axios.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config;
    
    if (error.response?.status === 401 && !originalRequest._retry) {
      originalRequest._retry = true;
      
      try {
        const tokens = localStorage.getItem('auth_tokens');
        if (tokens) {
          const parsedTokens = JSON.parse(tokens);
          const response = await axios.post('/auth/refresh', {
            refresh_token: parsedTokens.refresh_token
          });
          
          const newTokens = response.data.tokens;
          localStorage.setItem('auth_tokens', JSON.stringify(newTokens));
          
          // Retry original request with new token
          originalRequest.headers.Authorization = `Bearer ${newTokens.access_token}`;
          return axios(originalRequest);
        }
      } catch (refreshError) {
        // Refresh failed, logout user
        localStorage.removeItem('auth_tokens');
        localStorage.removeItem('user');
        window.location.href = '/login';
      }
    }
    
    return Promise.reject(error);
  }
);

interface AuthProviderProps {
  children: ReactNode;
}

export const AuthProvider: React.FC<AuthProviderProps> = ({ children }) => {
  const [user, setUser] = useState<User | null>(null);
  const [tokens, setTokens] = useState<AuthTokens | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Check for stored auth data on app start
    const storedTokens = localStorage.getItem('auth_tokens');
    const storedUser = localStorage.getItem('user');
    
    if (storedTokens && storedUser) {
      setTokens(JSON.parse(storedTokens));
      setUser(JSON.parse(storedUser));
    }
    
    setLoading(false);
  }, []);

  const login = async (email: string, password: string): Promise<void> => {
    try {
      const response = await axios.post('/auth/login', {
        email,
        password
      });

      const { user: userData, tokens: tokenData } = response.data;
      
      setUser(userData);
      setTokens(tokenData);
      
      localStorage.setItem('auth_tokens', JSON.stringify(tokenData));
      localStorage.setItem('user', JSON.stringify(userData));
    } catch (error: any) {
      throw new Error(error.response?.data?.error || 'Login failed');
    }
  };

  const register = async (
    email: string, 
    username: string, 
    password: string, 
    fullName?: string
  ): Promise<void> => {
    try {
      const response = await axios.post('/auth/register', {
        email,
        username,
        password,
        full_name: fullName
      });

      // After successful registration, automatically log in
      await login(email, password);
    } catch (error: any) {
      throw new Error(error.response?.data?.error || 'Registration failed');
    }
  };

  const logout = async (): Promise<void> => {
    try {
      if (tokens) {
        await axios.post('/auth/logout');
      }
    } catch (error) {
      console.error('Logout error:', error);
    } finally {
      setUser(null);
      setTokens(null);
      localStorage.removeItem('auth_tokens');
      localStorage.removeItem('user');
    }
  };

  const refreshToken = async (): Promise<void> => {
    try {
      if (!tokens?.refresh_token) {
        throw new Error('No refresh token available');
      }

      const response = await axios.post('/auth/refresh', {
        refresh_token: tokens.refresh_token
      });

      const newTokens = response.data.tokens;
      setTokens(newTokens);
      localStorage.setItem('auth_tokens', JSON.stringify(newTokens));
    } catch (error) {
      console.error('Token refresh failed:', error);
      logout();
    }
  };

  const value: AuthContextType = {
    user,
    tokens,
    login,
    register,
    logout,
    refreshToken,
    isAuthenticated: !!user && !!tokens,
    loading
  };

  return (
    <AuthContext.Provider value={value}>
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = (): AuthContextType => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};