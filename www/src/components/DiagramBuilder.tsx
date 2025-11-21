import React from "react";
import { createFlowDiagram, FlowDiagramConfig } from "@/utils/asciiBuilder";
import { AsciiDiagram } from "./AsciiDiagram";

interface DiagramBuilderProps {
  config: FlowDiagramConfig;
  title?: string;
}

/**
 * Simple Diagram Builder Component
 *
 * Usage:
 *
 * <DiagramBuilder
 *   config={{
 *     title: "My Process",
 *     width: 60,
 *     steps: [
 *       { label: "Start", type: "regular" },
 *       { label: "Process", type: "inner" },
 *       { label: "End", type: "regular" }
 *     ]
 *   }}
 * />
 */
export const DiagramBuilder: React.FC<DiagramBuilderProps> = ({
  config,
  title,
}) => {
  const asciiDiagram = createFlowDiagram(config);

  return <AsciiDiagram content={asciiDiagram} title={title} />;
};
