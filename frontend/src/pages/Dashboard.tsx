import React, { useState, useEffect } from 'react';
import { 
  CheckCircleIcon, 
  XCircleIcon, 
  ClockIcon,
  ChartBarIcon,
  CodeBracketIcon,
  CpuChipIcon
} from '@heroicons/react/24/outline';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

interface ServerStatus {
  status: string;
  version: string;
  ai_model_loaded: boolean;
  supported_languages: string[];
}

interface Stats {
  totalCompletions: number;
  avgResponseTime: number;
  successRate: number;
  activeUsers: number;
}

export const Dashboard: React.FC = () => {
  const [serverStatus, setServerStatus] = useState<ServerStatus | null>(null);
  const [stats] = useState<Stats>({
    totalCompletions: 1234,
    avgResponseTime: 85,
    successRate: 97.5,
    activeUsers: 42
  });
  const [loading, setLoading] = useState(true);

  const performanceData = [
    { time: '00:00', responseTime: 120 },
    { time: '04:00', responseTime: 95 },
    { time: '08:00', responseTime: 110 },
    { time: '12:00', responseTime: 85 },
    { time: '16:00', responseTime: 90 },
    { time: '20:00', responseTime: 75 },
  ];

  useEffect(() => {
    fetchServerStatus();
  }, []);

  const fetchServerStatus = async () => {
    try {
      const response = await fetch('/health');
      const data = await response.json();
      setServerStatus(data);
    } catch (error) {
      console.error('Failed to fetch server status:', error);
    } finally {
      setLoading(false);
    }
  };

  const StatusCard: React.FC<{ title: string; value: string | number; icon: React.ElementType; color: string }> = 
    ({ title, value, icon: Icon, color }) => (
    <div className="bg-white rounded-lg shadow p-6">
      <div className="flex items-center">
        <div className={`flex-shrink-0 p-3 rounded-md ${color}`}>
          <Icon className="h-6 w-6 text-white" />
        </div>
        <div className="ml-4">
          <p className="text-sm font-medium text-gray-600">{title}</p>
          <p className="text-2xl font-semibold text-gray-900">{value}</p>
        </div>
      </div>
    </div>
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
        <button
          onClick={fetchServerStatus}
          className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 transition-colors"
        >
          Refresh Status
        </button>
      </div>

      {/* Server Status */}
      <div className="bg-white rounded-lg shadow p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Server Status</h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="flex items-center space-x-3">
            {serverStatus?.status === 'healthy' ? (
              <CheckCircleIcon className="h-6 w-6 text-green-500" />
            ) : (
              <XCircleIcon className="h-6 w-6 text-red-500" />
            )}
            <div>
              <p className="text-sm text-gray-600">Status</p>
              <p className="font-medium">{serverStatus?.status || 'Unknown'}</p>
            </div>
          </div>
          
          <div className="flex items-center space-x-3">
            <CpuChipIcon className="h-6 w-6 text-blue-500" />
            <div>
              <p className="text-sm text-gray-600">AI Model</p>
              <p className="font-medium">
                {serverStatus?.ai_model_loaded ? 'Loaded' : 'Not Loaded'}
              </p>
            </div>
          </div>
          
          <div className="flex items-center space-x-3">
            <CodeBracketIcon className="h-6 w-6 text-purple-500" />
            <div>
              <p className="text-sm text-gray-600">Version</p>
              <p className="font-medium">{serverStatus?.version || 'Unknown'}</p>
            </div>
          </div>
        </div>
      </div>

      {/* Statistics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatusCard
          title="Total Completions"
          value={stats.totalCompletions.toLocaleString()}
          icon={CodeBracketIcon}
          color="bg-blue-500"
        />
        <StatusCard
          title="Avg Response Time"
          value={`${stats.avgResponseTime}ms`}
          icon={ClockIcon}
          color="bg-green-500"
        />
        <StatusCard
          title="Success Rate"
          value={`${stats.successRate}%`}
          icon={CheckCircleIcon}
          color="bg-purple-500"
        />
        <StatusCard
          title="Active Users"
          value={stats.activeUsers}
          icon={ChartBarIcon}
          color="bg-orange-500"
        />
      </div>

      {/* Performance Chart */}
      <div className="bg-white rounded-lg shadow p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Response Time (24h)</h2>
        <div className="h-64">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={performanceData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="time" />
              <YAxis />
              <Tooltip />
              <Line 
                type="monotone" 
                dataKey="responseTime" 
                stroke="#3B82F6" 
                strokeWidth={2}
                dot={{ fill: '#3B82F6' }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Supported Languages */}
      <div className="bg-white rounded-lg shadow p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Supported Languages</h2>
        <div className="flex flex-wrap gap-2">
          {serverStatus?.supported_languages?.map((language) => (
            <span
              key={language}
              className="bg-blue-100 text-blue-800 px-3 py-1 rounded-full text-sm font-medium"
            >
              {language}
            </span>
          )) || (
            <p className="text-gray-500">Loading languages...</p>
          )}
        </div>
      </div>
    </div>
  );
};