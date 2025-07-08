/*global chrome*/
import React, { useState, useEffect } from 'react';

// --- Hard‑coded list of ChatGPT models ---
const MODEL_LIST = [
  'gpt-4o',
  'gpt-4.1',
  'gpt-4.1-mini',
  'gpt-4.5-preview',
  'o3',
  'o4-mini',
  'o4-mini-high'
];

// --- Mocked lucide-react icons as SVG components ---
const Trash2 = ({ className }) => (
  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className}>
    <path d="M3 6h18" />
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    <line x1="10" y1="11" x2="10" y2="17" />
    <line x1="14" y1="11" x2="14" y2="17" />
  </svg>
);
const PlusCircle = ({ className }) => (
  <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className}>
    <circle cx="12" cy="12" r="10" />
    <line x1="12" y1="8" x2="12" y2="16" />
    <line x1="8" y1="12" x2="16" y2="12" />
  </svg>
);

// --- Mocked UI Components ---
const Card = ({ children, className = '' }) => (
  <div className={`bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-sm ${className}`}>
    {children}
  </div>
);

const CardContent = ({ children, className = '' }) => (
  <div className={`p-4 text-gray-800 dark:text-gray-100 ${className}`}>
    {children}
  </div>
);

const Input = (props) => (
  <input
    {...props}
    className={`w-full h-9 px-3 text-sm
      text-gray-800 dark:text-white
      bg-white dark:bg-gray-700
      border border-gray-300 dark:border-gray-600
      rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500
      ${props.className || ''}`}
  />
);

const Button = ({ children, variant = 'default', size = 'default', className = '', ...props }) => {
  const baseClasses = `
    inline-flex items-center justify-center
    rounded-md text-sm font-medium
    transition-colors
    focus:outline-none focus:ring-2 focus:ring-offset-2
  `;

  const variantClasses = {
    default: `
      bg-gray-900 text-white
      hover:bg-gray-800
      focus:ring-gray-900
    `,
    outline: `
      border border-gray-300 dark:border-gray-600
      bg-transparent
      text-gray-800 dark:text-white
      hover:bg-gray-100 dark:hover:bg-gray-700
      focus:ring-blue-500
      focus:ring-offset-2
      dark:focus:ring-offset-gray-900
    `,
    ghost: `
      text-gray-800 dark:text-gray-200
      hover:bg-gray-100 dark:hover:bg-gray-700
      focus:ring-gray-400
    `
  };

  const sizeClasses = {
    default: 'h-9 px-3',
    icon: 'h-9 w-9'
  };

  return (
    <button
      {...props}
      className={`
        ${baseClasses}
        ${variantClasses[variant]}
        ${sizeClasses[size]}
        ${className}
      `}
    >
      {children}
    </button>
  );
};

const Switch = ({ checked, onCheckedChange, id }) => (
  <div className="flex items-center gap-2">
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onCheckedChange(!checked)}
      id={id}
      className={`
        relative inline-flex items-center justify-start
        h-6 w-11 rounded-full
        transition-colors duration-200 ease-in-out
        focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
        border-2 border-transparent
        overflow-hidden
        ${checked ? 'bg-blue-600' : 'bg-gray-300 dark:bg-gray-600'}
      `}
    >
      <span
        aria-hidden="true"
        className={`
          pointer-events-none
          inline-block h-5 w-5 transform rounded-full bg-white
          shadow-md ring-0 transition-transform duration-200 ease-in-out
          ${checked ? 'translate-x-[20px]' : 'translate-x-0'}
        `}
      />
    </button>
    <span className="inline-block w-8 text-sm text-gray-700 dark:text-gray-300 text-center select-none">
      {checked ? 'On' : 'Off'}
    </span>
  </div>
);

const Label = (props) => (
  <label {...props} className={`text-sm font-medium leading-none text-gray-700 ${props.className || ''}`} />
);

export default function PreferenceBasedModelSelector() {
  const [routingEnabled, setRoutingEnabled] = useState(false);
  const [preferences, setPreferences] = useState([
    { id: 1, usage: 'generate code snippets', model: 'gpt-4o' }
  ]);
  const [defaultModel, setDefaultModel] = useState('gpt-4o');
  const [modelOptions] = useState(MODEL_LIST); // static list, no dynamic fetch

  // Load saved settings
  useEffect(() => {
    chrome.storage.sync.get(['routingEnabled', 'preferences', 'defaultModel'], (result) => {
      if (result.routingEnabled !== undefined) setRoutingEnabled(result.routingEnabled);

      if (result.preferences) {
        // add ids if they were missing
        const withIds = result.preferences.map((p, i) => ({
          id: p.id ?? i + 1,
          ...p,
        }));
        setPreferences(withIds);
      }

      if (result.defaultModel) setDefaultModel(result.defaultModel);
    });
  }, []);

  const updatePreference = (id, key, value) => {
    setPreferences((prev) => prev.map((p) => (p.id === id ? { ...p, [key]: value } : p)));
  };

  const addPreference = () => {
    const newId = preferences.reduce((max, p) => Math.max(max, p.id ?? 0), 0) + 1;
    setPreferences((prev) => [
      ...prev,
      { id: newId, usage: '', model: defaultModel }
    ]);
  };

  const removePreference = (id) => {
    if (preferences.length > 1) {
      setPreferences((prev) => prev.filter((p) => p.id !== id));
    }
  };

  // Save settings: generate name slug and store tuples
  const handleSave = () => {
    const slugCounts = {};
    const tuples = [];

    preferences
      .filter(p => p.usage?.trim())
      .forEach(p => {
        const baseSlug = p.usage
          .split(/\s+/)
          .slice(0, 3)
          .join('-')
          .toLowerCase()
          .replace(/[^\w-]/g, '');

        const count = slugCounts[baseSlug] || 0;
        slugCounts[baseSlug] = count + 1;

        const dedupedSlug = count === 0 ? baseSlug : `${baseSlug}-${count}`;

        tuples.push({
          name: dedupedSlug,
          usage: p.usage.trim(),
          model: p.model?.trim?.() || ''
        });
      });

    chrome.storage.sync.set({ routingEnabled, preferences: tuples, defaultModel }, () => {
      if (chrome.runtime.lastError) {
        console.error('[PBMS] Storage error:', chrome.runtime.lastError);
      } else {
        console.log('[PBMS] Saved tuples:', tuples);
      }
    });

    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      chrome.tabs.sendMessage(tabs[0].id, { action: 'applyModelSelection', model: defaultModel });
    });

    window.parent.postMessage({ action: 'CLOSE_PBMS_MODAL' }, '*');
  };

  const handleCancel = () => {
    window.parent.postMessage({ action: 'CLOSE_PBMS_MODAL' }, '*');
  };

  return (
  <div className="w-full max-w-[600px] h-[65vh] flex flex-col bg-gray-50 dark:bg-gray-800 p-4 mx-auto">

    {/* Scrollable preferences only */}
   <div className="space-y-4 overflow-y-auto flex-1 pr-1 max-h-[60vh]">
      <Card className="w-full">
        <CardContent>
          <div className="flex items-center justify-between">
            <Label>Enable preference-based routing</Label>
            <Switch checked={routingEnabled} onCheckedChange={setRoutingEnabled} />
          </div>
          {routingEnabled && (
            <div className="pt-4 mt-4 space-y-3 border-t border-gray-200 dark:border-gray-700">
              {preferences.map((pref) => (
                <div key={pref.id} className="grid grid-cols-[3fr_1.5fr_auto] gap-4 items-center">
                  <Input
                    placeholder="(e.g. generating fictional stories or poems)"
                    value={pref.usage}
                    onChange={(e) => updatePreference(pref.id, 'usage', e.target.value)}
                  />
                  <select
                    value={pref.model}
                    onChange={(e) => updatePreference(pref.id, 'model', e.target.value)}
                    className="h-9 w-full px-3 text-sm
                      bg-white dark:bg-gray-700
                      text-gray-800 dark:text-white
                      border border-gray-300 dark:border-gray-600
                      rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  >
                    <option disabled value="">
                      Select Model
                    </option>
                    {modelOptions.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </select>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => removePreference(pref.id)}
                    disabled={preferences.length <= 1}
                  >
                    <Trash2 className="h-4 w-4 text-gray-500 hover:text-red-600" />
                  </Button>
                </div>
              ))}
              <Button
                variant="outline"
                onClick={addPreference}
                className="flex items-center gap-2 text-sm mt-2"
              >
                <PlusCircle className="h-4 w-4" /> Add Preference
              </Button>
            </div>
          )}
        </CardContent>
      </Card>
    </div>

    {/* Default model selector (static) */}
    <Card className="w-full mt-4">
      <CardContent>
        <Label>Default Model</Label>
        <select
          value={defaultModel}
          onChange={(e) => setDefaultModel(e.target.value)}
          className="h-9 w-full mt-2 px-3 text-sm
            bg-white dark:bg-gray-700
            text-gray-800 dark:text-white
            border border-gray-300 dark:border-gray-600
            rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          {modelOptions.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      </CardContent>
    </Card>

    {/* Save/Cancel footer (static) */}
    <div className="flex justify-end gap-2 pt-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 mt-4">
      <Button variant="ghost" onClick={handleCancel}>
        Cancel
      </Button>
      <Button onClick={handleSave}>Save and Apply</Button>
    </div>

  </div>
);
}
