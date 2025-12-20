import React from "react";

interface AsciiDiagramProps {
  title?: string;
  content: string;
  className?: string;
}

export const AsciiDiagram: React.FC<AsciiDiagramProps> = ({
  title,
  content,
  className = "",
}) => {
  return (
    <div className={`max-w-4xl mx-auto mb-8 ${className}`}>
      {title && (
        <h2 className="text-2xl font-bold text-gray-900 dark:text-zinc-50 mb-4">
          {title}
        </h2>
      )}
      <div className="bg-gray-900 dark:bg-gray-950 rounded-lg p-6 shadow-xl overflow-x-auto">
        <pre
          className="relative font-mono text-xs leading-none text-white m-0 whitespace-pre"
          style={{ fontFamily: "var(--font-jetbrains-mono), monospace" }}
        >
          <code>{content}</code>
        </pre>
      </div>
    </div>
  );
};

// Programmatic diagram builder for non-coders
interface DiagramStep {
  id: string;
  label: string;
  type?: "input" | "inner" | "regular";
  x: number;
  y: number;
}

interface FlowConnection {
  from: string;
  to: string;
  label?: string;
}

interface DiagramConfig {
  title: string;
  steps: DiagramStep[];
  connections: FlowConnection[];
}

// Simple ASCII diagram generator
export const createDiagram = (config: DiagramConfig): string => {
  // This is a simplified version - you can extend this to automatically generate
  // the ASCII art from the config
  // For now, return the manually created diagrams
  return "";
};

// Helper to create boxes
export const createBox = (
  label: string,
  type: "input" | "inner" | "regular" = "regular",
  width: number = 20,
): string[] => {
  const padding = Math.max(0, Math.floor((width - label.length) / 2));
  const spaces = " ".repeat(padding);
  const remaining = width - label.length - padding;

  let chars;
  switch (type) {
    case "input":
      chars = { tl: "╔", tr: "╗", bl: "╚", br: "╝", h: "═", v: "║" };
      break;
    case "inner":
      chars = { tl: "┏", tr: "┓", bl: "┗", br: "┛", h: "━", v: "┃" };
      break;
    case "regular":
    default:
      chars = { tl: "┌", tr: "┐", bl: "└", br: "┘", h: "─", v: "│" };
  }

  return [
    `${chars.tl}${chars.h.repeat(width)}${chars.tr}`,
    `${chars.v}${spaces}${label}${" ".repeat(remaining)}${chars.v}`,
    `${chars.bl}${chars.h.repeat(width)}${chars.br}`,
  ];
};
