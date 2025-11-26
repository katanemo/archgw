import { createFlowDiagram, FlowStep } from "@/utils/asciiBuilder";

/**
 * Easy-to-use diagram templates that automatically handle spacing
 * Perfect for non-coders who just want to define their flow
 */

// Example: Simple 3-step process
export const createSimpleProcess = (steps: string[]) => {
  return createFlowDiagram({
    title: "Process Flow",
    width: 60,
    steps: steps.map((label) => ({
      label,
      type: "regular" as const,
      shadow: true,
    })),
  });
};

// Example: Create a nested container diagram
export const createNestedDiagram = (
  title: string,
  innerContent: FlowStep[],
  width: number = 70,
) => {
  return createFlowDiagram({
    title,
    width,
    steps: innerContent,
  });
};

// Pre-built templates
export const templates = {
  simpleFlow: createSimpleProcess(["Start", "Process", "End"]),

  apiFlow: createFlowDiagram({
    title: "API Request Flow",
    width: 65,
    steps: [
      { label: "Client Request", type: "regular", shadow: true },
      { label: "API Gateway", type: "container", shadow: true },
      { label: "Process", type: "inner", shadow: true },
      { label: "Response", type: "regular", shadow: true },
    ],
  }),

  dataPipeline: createFlowDiagram({
    title: "Data Pipeline",
    width: 70,
    steps: [
      { label: "Input Data", type: "regular", shadow: true },
      { label: "Transform", type: "inner", shadow: true },
      { label: "Validate", type: "regular", shadow: true },
      { label: "Store", type: "regular", shadow: true },
    ],
  }),
};
