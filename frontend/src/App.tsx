import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { Navbar } from './components/Navbar';
import { Dashboard } from './pages/Dashboard';
import { Playground } from './pages/Playground';
import { Settings } from './pages/Settings';
import { Documentation } from './pages/Documentation';
import './App.css';

function App() {
  return (
    <Router>
      <div className="min-h-screen bg-gray-50">
        <Navbar />
        <main className="container mx-auto px-4 py-8">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/playground" element={<Playground />} />
            <Route path="/settings" element={<Settings />} />
            <Route path="/docs" element={<Documentation />} />
          </Routes>
        </main>
      </div>
    </Router>
  );
}

export default App;