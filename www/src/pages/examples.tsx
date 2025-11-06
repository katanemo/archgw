"use client";

import { DiagramBuilder } from "@/components/DiagramBuilder";
import { createSimpleProcess, templates } from "@/data/diagramTemplates";
import { createFlowDiagram } from "@/utils/asciiBuilder";

export default function ExamplesPage() {
  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-black font-sans py-16 px-8">
      <div className="max-w-5xl mx-auto">
        <h1 className="text-4xl font-bold text-gray-900 dark:text-zinc-50 mb-8">
          Programmatic Diagram Examples
        </h1>

        <div className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-zinc-50 mb-4">
            Example 1: Simple Process Flow
          </h2>
          <p className="text-gray-600 dark:text-zinc-400 mb-4">
            Create a simple 3-step process programmatically:
          </p>
          <pre className="bg-gray-900 text-white p-4 rounded-lg mb-4 overflow-x-auto">
{`import { createSimpleProcess } from '@/data/diagramTemplates';

const diagram = createSimpleProcess(['Start', 'Process', 'End']);`}
          </pre>
          <DiagramBuilder
            config={{
              title: "Simple Process",
              steps: [
                { label: 'Start', type: 'regular', shadow: true },
                { label: 'Process', type: 'regular', shadow: true },
                { label: 'End', type: 'regular', shadow: true }
              ]
            }}
          />
        </div>

        <div className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-zinc-50 mb-4">
            Example 2: API Flow with Different Box Types
          </h2>
          <p className="text-gray-600 dark:text-zinc-400 mb-4">
            Mix container, inner, and regular boxes:
          </p>
          <pre className="bg-gray-900 text-white p-4 rounded-lg mb-4 overflow-x-auto">
{`const diagram = createFlowDiagram({
  title: 'API Request Flow',
  width: 65,
  steps: [
    { label: 'Client Request', type: 'regular', shadow: true },
    { label: 'API Gateway', type: 'container', shadow: true },
    { label: 'Process', type: 'inner', shadow: true },
    { label: 'Response', type: 'regular', shadow: true }
  ]
});`}
          </pre>
          <DiagramBuilder
            config={{
              title: 'API Request Flow',
              width: 65,
              steps: [
                { label: 'Client Request', type: 'regular', shadow: true },
                { label: 'API Gateway', type: 'container', shadow: true },
                { label: 'Process', type: 'inner', shadow: true },
                { label: 'Response', type: 'regular', shadow: true }
              ]
            }}
          />
        </div>

        <div className="mb-12">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-zinc-50 mb-4">
            Example 3: Data Pipeline
          </h2>
          <DiagramBuilder
            config={{
              title: 'Data Pipeline',
              width: 70,
              steps: [
                { label: 'Collect', type: 'regular', shadow: true },
                { label: 'Transform', type: 'inner', shadow: true },
                { label: 'Validate', type: 'regular', shadow: true },
                { label: 'Store', type: 'container', shadow: true }
              ]
            }}
          />
        </div>

        <div className="mt-12 bg-white dark:bg-zinc-900 rounded-lg p-6 shadow">
          <h3 className="text-xl font-bold text-gray-900 dark:text-zinc-50 mb-4">
            How to Use Programmatic Diagrams
          </h3>
          <div className="space-y-4 text-gray-700 dark:text-zinc-300">
            <div>
              <h4 className="font-semibold mb-2">1. Import the Builder</h4>
              <pre className="bg-gray-50 dark:bg-zinc-950 p-3 rounded mt-2 text-sm">
{`import { DiagramBuilder } from '@/components/DiagramBuilder';`}
              </pre>
            </div>

            <div>
              <h4 className="font-semibold mb-2">2. Define Your Steps</h4>
              <pre className="bg-gray-50 dark:bg-zinc-950 p-3 rounded mt-2 text-sm overflow-x-auto">
{`<DiagramBuilder
  config={{
    title: "My Process",
    width: 60,
    steps: [
      { label: "Step 1", type: "regular", shadow: true },
      { label: "Step 2", type: "inner", shadow: true },
      { label: "Step 3", type: "container", shadow: true }
    ]
  }}
/>`}
              </pre>
            </div>

            <div>
              <h4 className="font-semibold mb-2">3. Box Types</h4>
              <ul className="list-disc list-inside space-y-1">
                <li><code className="bg-gray-100 dark:bg-zinc-800 px-1 rounded">regular</code> - Thin box borders (┌─┐)</li>
                <li><code className="bg-gray-100 dark:bg-zinc-800 px-1 rounded">inner</code> - Thick borders (┏━┓)</li>
                <li><code className="bg-gray-100 dark:bg-zinc-800 px-1 rounded">container</code> - Extra thick borders (╔═╗)</li>
              </ul>
            </div>

            <div>
              <h4 className="font-semibold mb-2">Benefits</h4>
              <ul className="list-disc list-inside space-y-1">
                <li>✅ No manual spacing calculations</li>
                <li>✅ Automatic alignment</li>
                <li>✅ Easy for non-coders to use</li>
                <li>✅ Consistent formatting</li>
                <li>✅ Type-safe with TypeScript</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

