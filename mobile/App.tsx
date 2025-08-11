import React from 'react';
import { StatusBar } from 'expo-status-bar';
import { NavigationContainer } from '@react-navigation/native';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { createStackNavigator } from '@react-navigation/stack';
import { Provider as PaperProvider, MD3DarkTheme } from 'react-native-paper';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import Icon from 'react-native-vector-icons/MaterialIcons';

// Screens
import DashboardScreen from './src/screens/DashboardScreen';
import CodeScannerScreen from './src/screens/CodeScannerScreen';
import AIAssistantScreen from './src/screens/AIAssistantScreen';
import CollaborationScreen from './src/screens/CollaborationScreen';
import AnalyticsScreen from './src/screens/AnalyticsScreen';
import SettingsScreen from './src/screens/SettingsScreen';
import CodeEditorScreen from './src/screens/CodeEditorScreen';
import VoiceCommandScreen from './src/screens/VoiceCommandScreen';
import OfflineModeScreen from './src/screens/OfflineModeScreen';

// Services
import { NotificationService } from './src/services/NotificationService';
import { OfflineService } from './src/services/OfflineService';

const Tab = createBottomTabNavigator();
const Stack = createStackNavigator();

// Custom theme
const theme = {
  ...MD3DarkTheme,
  colors: {
    ...MD3DarkTheme.colors,
    primary: '#2196f3',
    secondary: '#f50057',
    background: '#0a0e27',
    surface: '#1a1d3a',
  },
};

// Query client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000, // 5 minutes
      cacheTime: 10 * 60 * 1000, // 10 minutes
    },
  },
});

function TabNavigator() {
  return (
    <Tab.Navigator
      screenOptions={({ route }) => ({
        tabBarIcon: ({ focused, color, size }) => {
          let iconName: string;

          switch (route.name) {
            case 'Dashboard':
              iconName = 'dashboard';
              break;
            case 'Scanner':
              iconName = 'qr-code-scanner';
              break;
            case 'AI Assistant':
              iconName = 'smart-toy';
              break;
            case 'Collaboration':
              iconName = 'group';
              break;
            case 'Analytics':
              iconName = 'analytics';
              break;
            default:
              iconName = 'help';
          }

          return <Icon name={iconName} size={size} color={color} />;
        },
        tabBarActiveTintColor: theme.colors.primary,
        tabBarInactiveTintColor: 'gray',
        tabBarStyle: {
          backgroundColor: theme.colors.surface,
          borderTopColor: theme.colors.outline,
        },
        headerStyle: {
          backgroundColor: theme.colors.surface,
        },
        headerTintColor: theme.colors.onSurface,
      })}
    >
      <Tab.Screen name="Dashboard" component={DashboardScreen} />
      <Tab.Screen name="Scanner" component={CodeScannerScreen} />
      <Tab.Screen name="AI Assistant" component={AIAssistantScreen} />
      <Tab.Screen name="Collaboration" component={CollaborationScreen} />
      <Tab.Screen name="Analytics" component={AnalyticsScreen} />
    </Tab.Navigator>
  );
}

function AppNavigator() {
  return (
    <Stack.Navigator
      screenOptions={{
        headerStyle: {
          backgroundColor: theme.colors.surface,
        },
        headerTintColor: theme.colors.onSurface,
      }}
    >
      <Stack.Screen 
        name="Main" 
        component={TabNavigator} 
        options={{ headerShown: false }}
      />
      <Stack.Screen 
        name="CodeEditor" 
        component={CodeEditorScreen}
        options={{ title: 'Code Editor' }}
      />
      <Stack.Screen 
        name="VoiceCommand" 
        component={VoiceCommandScreen}
        options={{ title: 'Voice Commands' }}
      />
      <Stack.Screen 
        name="OfflineMode" 
        component={OfflineModeScreen}
        options={{ title: 'Offline Mode' }}
      />
      <Stack.Screen 
        name="Settings" 
        component={SettingsScreen}
        options={{ title: 'Settings' }}
      />
    </Stack.Navigator>
  );
}

export default function App() {
  React.useEffect(() => {
    // Initialize services
    NotificationService.initialize();
    OfflineService.initialize();
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <PaperProvider theme={theme}>
        <NavigationContainer>
          <StatusBar style="light" backgroundColor={theme.colors.surface} />
          <AppNavigator />
        </NavigationContainer>
      </PaperProvider>
    </QueryClientProvider>
  );
}