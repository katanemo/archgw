import React from 'react';
import PreferenceBasedModelSelector from './components/PreferenceBasedModelSelector';

export default function App() {
  return (
    <div className="bg-gray-100 min-h-screen flex items-center justify-center p-4">
      <div className="w-full max-w-6xl">
        <div className="text-center mb-8">
            <h1 className="text-3xl font-bold text-gray-800">RouteGPT</h1>
            <p className="text-gray-600 mt-2">Dynamically route to GPT models based on usage preferences.</p>
            <a target="_blank" href="https://github.com/katanemo/archgw" className="text-blue-500 hover:underline">powered by Arch Router</a>
        </div>
        <PreferenceBasedModelSelector />
      </div>
    </div>
  );
}
