import React, { useState, useEffect } from 'react';
import { useAuth } from '../contexts/AuthContext';
import { 
  KeyIcon, 
  CogIcon, 
  UserIcon, 
  ShieldCheckIcon,
  PlusIcon,
  TrashIcon,
  EyeIcon,
  EyeSlashIcon
} from '@heroicons/react/24/outline';
import axios from 'axios';

interface ApiKey {
  id: string;
  provider: string;
  key_name: string;
  is_active: boolean;
  last_used_at?: string;
  usage_count: number;
  monthly_limit?: number;
  created_at: string;
}

interface UserPreferences {
  default_provider: string;
  default_model: string;
  max_tokens: number;
  temperature: number;
  auto_save: boolean;
  create_backups: boolean;
  theme: string;
  language: string;
}

const PROVIDERS = [
  { id: 'openrouter', name: 'OpenRouter', description: '100+ models via unified API' },
  { id: 'openai', name: 'OpenAI', description: 'GPT-4o, GPT-4o-mini, GPT-3.5-turbo' },
  { id: 'anthropic', name: 'Anthropic', description: 'Claude 3.5 Sonnet, Claude 3 Haiku' },
  { id: 'google', name: 'Google Gemini', description: 'Gemini Pro, Gemini Flash' },
  { id: 'groq', name: 'Groq', description: 'Ultra-fast Llama 3.1, Mixtral' },
  { id: 'together', name: 'Together AI', description: 'Llama-2-70b, CodeLlama-34b' },
  { id: 'cohere', name: 'Cohere', description: 'Command-R+, Command-R' },
  { id: 'ollama', name: 'Ollama', description: 'Local model execution' }
];

export const Settings: React.FC = () => {
  const { user } = useAuth();
  const [activeTab, setActiveTab] = useState('api-keys');
  const [apiKeys, setApiKeys] = useState<ApiKey[]>([]);
  const [preferences, setPreferences] = useState<UserPreferences>({
    default_provider: 'openrouter',
    default_model: 'gpt-4o-mini',
    max_tokens: 4000,
    temperature: 0.7,
    auto_save: true,
    create_backups: true,
    theme: 'dark',
    language: 'en'
  });
  const [loading, setLoading] = useState(false);
  const [showAddKeyModal, setShowAddKeyModal] = useState(false);
  const [newApiKey, setNewApiKey] = useState({
    provider: '',
    key_name: '',
    api_key: '',
    monthly_limit: ''
  });
  const [showApiKey, setShowApiKey] = useState(false);

  useEffect(() => {
    loadApiKeys();
    loadPreferences();
  }, []);

  const loadApiKeys = async () => {
    try {
      const response = await axios.get('/api-keys');
      setApiKeys(response.data.api_keys);
    } catch (error) {
      console.error('Failed to load API keys:', error);
    }
  };

  const loadPreferences = async () => {
    try {
      const response = await axios.get('/preferences');
      if (response.data.success && response.data.preferences) {
        const prefs = response.data.preferences;
        setPreferences({
          default_provider: prefs.default_provider,
          default_model: prefs.default_model,
          max_tokens: prefs.max_tokens,
          temperature: prefs.temperature,
          auto_save: prefs.auto_save,
          create_backups: prefs.create_backups,
          theme: prefs.theme,
          language: prefs.language,
        });
      }
    } catch (error) {
      console.error('Failed to load preferences:', error);
    }
  };

  const handleAddApiKey = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);

    try {
      await axios.post('/api-keys', {
        provider: newApiKey.provider,
        key_name: newApiKey.key_name,
        api_key: newApiKey.api_key,
        monthly_limit: newApiKey.monthly_limit ? parseInt(newApiKey.monthly_limit) : null
      });

      setNewApiKey({ provider: '', key_name: '', api_key: '', monthly_limit: '' });
      setShowAddKeyModal(false);
      loadApiKeys();
    } catch (error: any) {
      console.error('Failed to add API key:', error);
      alert(error.response?.data?.error || 'Failed to add API key');
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteApiKey = async (keyId: string) => {
    if (!confirm('Are you sure you want to delete this API key?')) {
      return;
    }

    try {
      await axios.delete(`/api-keys/${keyId}`);
      loadApiKeys();
    } catch (error) {
      console.error('Failed to delete API key:', error);
      alert('Failed to delete API key');
    }
  };

  const handleSavePreferences = async () => {
    setLoading(true);
    try {
      const response = await axios.put('/preferences', preferences);
      if (response.data.success) {
        alert('Preferences saved successfully!');
        // Reload preferences to get updated data
        loadPreferences();
      } else {
        throw new Error(response.data.error || 'Failed to save preferences');
      }
    } catch (error: any) {
      console.error('Failed to save preferences:', error);
      alert(error.response?.data?.error || 'Failed to save preferences');
    } finally {
      setLoading(false);
    }
  };

  const tabs = [
    { id: 'api-keys', name: 'API Keys', icon: KeyIcon },
    { id: 'preferences', name: 'Preferences', icon: CogIcon },
    { id: 'profile', name: 'Profile', icon: UserIcon },
    { id: 'security', name: 'Security', icon: ShieldCheckIcon }
  ];

  return (
    <div className="max-w-6xl mx-auto">
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900">Settings</h1>
        <p className="mt-2 text-gray-600">
          Manage your account settings, API keys, and preferences.
        </p>
      </div>

      {/* Tab Navigation */}
      <div className="border-b border-gray-200 mb-8">
        <nav className="-mb-px flex space-x-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center space-x-2 py-2 px-1 border-b-2 font-medium text-sm ${
                activeTab === tab.id
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              <tab.icon className="h-5 w-5" />
              <span>{tab.name}</span>
            </button>
          ))}
        </nav>
      </div>

      {/* API Keys Tab */}
      {activeTab === 'api-keys' && (
        <div className="space-y-6">
          <div className="flex justify-between items-center">
            <div>
              <h2 className="text-xl font-semibold text-gray-900">API Keys</h2>
              <p className="text-gray-600">Manage your AI provider API keys securely.</p>
            </div>
            <button
              onClick={() => setShowAddKeyModal(true)}
              className="flex items-center space-x-2 bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700"
            >
              <PlusIcon className="h-5 w-5" />
              <span>Add API Key</span>
            </button>
          </div>

          <div className="grid gap-4">
            {apiKeys.map((key) => (
              <div key={key.id} className="bg-white border border-gray-200 rounded-lg p-6">
                <div className="flex justify-between items-start">
                  <div className="flex-1">
                    <div className="flex items-center space-x-3">
                      <h3 className="text-lg font-medium text-gray-900">{key.key_name}</h3>
                      <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                        {key.provider}
                      </span>
                      {key.is_active ? (
                        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                          Active
                        </span>
                      ) : (
                        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800">
                          Inactive
                        </span>
                      )}
                    </div>
                    <div className="mt-2 text-sm text-gray-600">
                      <p>Usage: {key.usage_count} requests</p>
                      {key.monthly_limit && (
                        <p>Monthly limit: {key.monthly_limit} requests</p>
                      )}
                      {key.last_used_at && (
                        <p>Last used: {new Date(key.last_used_at).toLocaleDateString()}</p>
                      )}
                      <p>Created: {new Date(key.created_at).toLocaleDateString()}</p>
                    </div>
                  </div>
                  <button
                    onClick={() => handleDeleteApiKey(key.id)}
                    className="text-red-600 hover:text-red-800"
                  >
                    <TrashIcon className="h-5 w-5" />
                  </button>
                </div>
              </div>
            ))}

            {apiKeys.length === 0 && (
              <div className="text-center py-12 bg-gray-50 rounded-lg">
                <KeyIcon className="mx-auto h-12 w-12 text-gray-400" />
                <h3 className="mt-2 text-sm font-medium text-gray-900">No API keys</h3>
                <p className="mt-1 text-sm text-gray-500">
                  Add your first API key to start using AI providers.
                </p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Preferences Tab */}
      {activeTab === 'preferences' && (
        <div className="space-y-6">
          <div>
            <h2 className="text-xl font-semibold text-gray-900">Preferences</h2>
            <p className="text-gray-600">Customize your AI assistant experience.</p>
          </div>

          <div className="bg-white border border-gray-200 rounded-lg p-6 space-y-6">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div>
                <label className="block text-sm font-medium text-gray-700">Default Provider</label>
                <select
                  value={preferences.default_provider}
                  onChange={(e) => setPreferences({ ...preferences, default_provider: e.target.value })}
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                >
                  {PROVIDERS.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.name}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700">Default Model</label>
                <input
                  type="text"
                  value={preferences.default_model}
                  onChange={(e) => setPreferences({ ...preferences, default_model: e.target.value })}
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700">Max Tokens</label>
                <input
                  type="number"
                  value={preferences.max_tokens}
                  onChange={(e) => setPreferences({ ...preferences, max_tokens: parseInt(e.target.value) })}
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700">Temperature</label>
                <input
                  type="number"
                  step="0.1"
                  min="0"
                  max="2"
                  value={preferences.temperature}
                  onChange={(e) => setPreferences({ ...preferences, temperature: parseFloat(e.target.value) })}
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700">Theme</label>
                <select
                  value={preferences.theme}
                  onChange={(e) => setPreferences({ ...preferences, theme: e.target.value })}
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                >
                  <option value="light">Light</option>
                  <option value="dark">Dark</option>
                  <option value="auto">Auto</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700">Language</label>
                <select
                  value={preferences.language}
                  onChange={(e) => setPreferences({ ...preferences, language: e.target.value })}
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                >
                  <option value="en">English</option>
                  <option value="tr">Türkçe</option>
                  <option value="es">Español</option>
                  <option value="fr">Français</option>
                </select>
              </div>
            </div>

            <div className="space-y-4">
              <div className="flex items-center">
                <input
                  id="auto_save"
                  type="checkbox"
                  checked={preferences.auto_save}
                  onChange={(e) => setPreferences({ ...preferences, auto_save: e.target.checked })}
                  className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label htmlFor="auto_save" className="ml-2 block text-sm text-gray-900">
                  Auto-save conversations
                </label>
              </div>

              <div className="flex items-center">
                <input
                  id="create_backups"
                  type="checkbox"
                  checked={preferences.create_backups}
                  onChange={(e) => setPreferences({ ...preferences, create_backups: e.target.checked })}
                  className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label htmlFor="create_backups" className="ml-2 block text-sm text-gray-900">
                  Create automatic backups
                </label>
              </div>
            </div>

            <div className="flex justify-end">
              <button
                onClick={handleSavePreferences}
                disabled={loading}
                className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50"
              >
                {loading ? 'Saving...' : 'Save Preferences'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Add API Key Modal */}
      {showAddKeyModal && (
        <div className="fixed inset-0 bg-gray-600 bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h3 className="text-lg font-medium text-gray-900 mb-4">Add API Key</h3>
            
            <form onSubmit={handleAddApiKey} className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700">Provider</label>
                <select
                  value={newApiKey.provider}
                  onChange={(e) => setNewApiKey({ ...newApiKey, provider: e.target.value })}
                  required
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                >
                  <option value="">Select a provider</option>
                  {PROVIDERS.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.name}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700">Key Name</label>
                <input
                  type="text"
                  value={newApiKey.key_name}
                  onChange={(e) => setNewApiKey({ ...newApiKey, key_name: e.target.value })}
                  placeholder="e.g., My OpenAI Key"
                  required
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700">API Key</label>
                <div className="mt-1 relative">
                  <input
                    type={showApiKey ? 'text' : 'password'}
                    value={newApiKey.api_key}
                    onChange={(e) => setNewApiKey({ ...newApiKey, api_key: e.target.value })}
                    placeholder="sk-..."
                    required
                    className="block w-full pr-10 border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                  />
                  <button
                    type="button"
                    className="absolute inset-y-0 right-0 pr-3 flex items-center"
                    onClick={() => setShowApiKey(!showApiKey)}
                  >
                    {showApiKey ? (
                      <EyeSlashIcon className="h-5 w-5 text-gray-400" />
                    ) : (
                      <EyeIcon className="h-5 w-5 text-gray-400" />
                    )}
                  </button>
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700">Monthly Limit (optional)</label>
                <input
                  type="number"
                  value={newApiKey.monthly_limit}
                  onChange={(e) => setNewApiKey({ ...newApiKey, monthly_limit: e.target.value })}
                  placeholder="1000"
                  className="mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500"
                />
              </div>

              <div className="flex justify-end space-x-3 pt-4">
                <button
                  type="button"
                  onClick={() => setShowAddKeyModal(false)}
                  className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={loading}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50"
                >
                  {loading ? 'Adding...' : 'Add Key'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};