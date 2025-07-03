import React from 'react';
import PreferenceBasedModelSelector from './components/PreferenceBasedModelSelector';

export default function App() {
  return (
    <div className="bg-gray-100 dark:bg-gray-900 min-h-screen flex items-center justify-center p-4">
      <div className="w-full max-w-6xl">
        <div className="text-center mb-8">
          <div className="flex justify-center items-center gap-3 -ml-12">
            <img src="/logo.png" alt="RouteGPT Logo" className="w-10 h-10" />
            <h1 className="text-3xl font-bold text-gray-800 dark:text-gray-100">RouteGPT</h1>
          </div>
          <p className="text-gray-600 dark:text-gray-300 mt-2">
            Dynamically route to GPT models based on usage preferences.
          </p>
          <a
            target="_blank"
            href="https://github.com/katanemo/archgw"
            className="text-blue-500 dark:text-blue-400 hover:underline"
          >
            powered by Arch Router
          </a>
        </div>
        <PreferenceBasedModelSelector />
      </div>
    </div>
  );
}
