import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { 
  HomeIcon, 
  CodeBracketIcon, 
  CogIcon, 
  DocumentTextIcon,
  CpuChipIcon 
} from '@heroicons/react/24/outline';

export const Navbar: React.FC = () => {
  const location = useLocation();

  const navigation = [
    { name: 'Dashboard', href: '/', icon: HomeIcon },
    { name: 'Playground', href: '/playground', icon: CodeBracketIcon },
    { name: 'Settings', href: '/settings', icon: CogIcon },
    { name: 'Documentation', href: '/docs', icon: DocumentTextIcon },
  ];

  return (
    <nav className="bg-white shadow-lg">
      <div className="container mx-auto px-4">
        <div className="flex justify-between items-center py-4">
          <div className="flex items-center space-x-2">
            <CpuChipIcon className="h-8 w-8 text-blue-600" />
            <span className="text-xl font-bold text-gray-900">
              Universal AI Dev Assistant
            </span>
          </div>
          
          <div className="flex space-x-8">
            {navigation.map((item) => {
              const isActive = location.pathname === item.href;
              return (
                <Link
                  key={item.name}
                  to={item.href}
                  className={`flex items-center space-x-2 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                    isActive
                      ? 'bg-blue-100 text-blue-700'
                      : 'text-gray-600 hover:text-gray-900 hover:bg-gray-100'
                  }`}
                >
                  <item.icon className="h-5 w-5" />
                  <span>{item.name}</span>
                </Link>
              );
            })}
          </div>
        </div>
      </div>
    </nav>
  );
};