/**
 * ASCII Diagram Builder - Auto-spacing and formatting utilities
 *
 * This module provides utilities to ensure consistent spacing across ASCII diagrams
 * similar to the intent detection diagram pattern.
 */

interface BoxDimensions {
  label: string;
  width: number;
  height: number;
}

/**
 * Calculates proper padding to center content within a container width
 */
export function calculateCenterPadding(
  contentWidth: number,
  containerWidth: number,
): number {
  return Math.floor((containerWidth - contentWidth) / 2);
}

/**
 * Creates a horizontal arrow between two positions
 */
export function createArrow(
  length: number,
  direction: "→" | "↓" | "↑" | "←" = "→",
): string {
  return direction.repeat(length);
}

/**
 * Builds a box with specified dimensions, label, and box type
 */
export function buildBox(
  label: string,
  type: "container" | "inner" | "regular" = "regular",
  shadow: boolean = true,
  width?: number,
): string[] {
  const actualWidth = width || Math.max(label.length + 4, 12);
  const paddedLabel = label
    .padStart(Math.floor((actualWidth - 2 + label.length) / 2), " ")
    .padEnd(actualWidth - 2, " ");

  const symbols = {
    container: { tl: "╔", tr: "╗", bl: "╚", br: "╝", h: "═", v: "║" },
    inner: { tl: "┏", tr: "┓", bl: "┗", br: "┛", h: "━", v: "┃" },
    regular: { tl: "┌", tr: "┐", bl: "└", br: "┘", h: "─", v: "│" },
  };

  const s = symbols[type];
  const shadowChar = "░";

  const lines = [
    s.tl + s.h.repeat(actualWidth - 2) + s.tr + (shadow ? shadowChar : ""),
    s.v + paddedLabel + s.v + (shadow ? shadowChar : ""),
    s.bl + s.h.repeat(actualWidth - 2) + s.br + (shadow ? shadowChar : ""),
  ];

  if (shadow) {
    lines.push(" " + shadowChar.repeat(actualWidth));
  }

  return lines;
}

/**
 * Fixes spacing in an existing diagram by analyzing and adjusting alignment
 */
export function fixDiagramSpacing(diagram: string): string {
  const lines = diagram.split("\n");
  if (lines.length === 0) return diagram;

  // Find the container boundaries (look for ╔ and ╚ markers)
  let containerStart = -1;
  let containerEnd = -1;
  let containerWidth = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.includes("╔═") && line.includes("╗")) {
      containerStart = i;
      containerWidth = line.length;
    }
    if (line.includes("╚") && line.includes("╝")) {
      containerEnd = i;
    }
  }

  if (containerStart === -1 || containerEnd === -1) {
    return diagram; // Can't fix if no container found
  }

  // The intent detection pattern shows:
  // Line 2: title line with ╔═ {title} ═{fill}╗
  // Lines 3-19: content with ║ on sides
  // Line 20: bottom ╚══════╝
  // Line 21: shadow line

  // For intent detection, the container content width is about 60 chars
  // Total line width including borders is about 68-70
  // Content starts around position 26

  // Detect pattern by looking at first content line
  const firstContentLine = lines[containerStart + 1];
  if (!firstContentLine) return diagram;

  const leftPadding = firstContentLine.indexOf("║");
  const rightPadding = containerWidth - firstContentLine.lastIndexOf("║") - 1;

  // Now standardize all internal lines
  const fixedLines = [...lines];

  for (let i = containerStart + 1; i < containerEnd; i++) {
    const line = lines[i];
    const shadowIndex = line.indexOf("░");

    if (line.trim().startsWith("║")) {
      // This is a content line inside the container
      // Standardize the padding
      const content = extractContainerContent(line);
      fixedLines[i] = padContainerLine(content, containerWidth, leftPadding);
    }
  }

  return fixedLines.join("\n");
}

function extractContainerContent(line: string): string {
  // Extract content between ║ characters
  const startIdx = line.indexOf("║");
  const endIdx = line.lastIndexOf("║");
  if (startIdx === -1 || endIdx === -1 || startIdx === endIdx) return line;
  return line.substring(startIdx + 1, endIdx);
}

function padContainerLine(
  content: string,
  containerWidth: number,
  targetLeftPad: number,
): string {
  const padding = " ".repeat(targetLeftPad);
  const contentLength = content.length;
  const rightPadding = containerWidth - targetLeftPad - contentLength - 2; // -2 for two ║
  const rightPad = rightPadding > 0 ? " ".repeat(rightPadding) : "";

  return padding + "║" + content + "║" + rightPad + "░";
}

/**
 * Creates a simple flow diagram programmatically
 * Usage:
 * ```ts
 * const diagram = createFlowDiagram({
 *   title: "My Process",
 *   width: 60,
 *   steps: [
 *     { label: "Step 1", type: "regular" },
 *     { label: "Step 2", type: "inner" },
 *     { label: "Step 3", type: "regular" }
 *   ]
 * });
 * ```
 */
export interface FlowStep {
  label: string;
  type?: "container" | "inner" | "regular";
  shadow?: boolean;
}

export interface FlowDiagramConfig {
  title: string;
  width?: number;
  steps: FlowStep[];
  layout?: "vertical" | "horizontal";
  externalElements?: FlowStep[]; // Elements outside the container (like "agent")
}

export function createFlowDiagram(config: FlowDiagramConfig): string {
  const layout = config.layout || "vertical";

  if (layout === "horizontal") {
    return createHorizontalFlow(config);
  } else {
    return createVerticalFlow(config);
  }
}

function createVerticalFlow(config: FlowDiagramConfig): string {
  const width = config.width || 60;
  const hasExternal =
    config.externalElements && config.externalElements.length > 0;

  // Build external elements first
  let externalBoxes: string[] = [];
  let externalWidth = 0;

  if (hasExternal) {
    externalWidth = 20;
    for (const extEl of config.externalElements!) {
      const extWidth = Math.max(extEl.label.length + 4, 12);
      const extBoxLines = buildBox(
        extEl.label,
        extEl.type || "regular",
        extEl.shadow !== false,
        extWidth,
      );

      for (const extLine of extBoxLines) {
        externalBoxes.push(" ".repeat(2) + extLine);
      }

      // Add vertical arrow if not last
      if (
        extEl !== config.externalElements![config.externalElements!.length - 1]
      ) {
        const arrowPad = 2 + Math.floor(extWidth / 2);
        externalBoxes.push(" ".repeat(arrowPad) + "▼");
      }
    }
  }

  const titleLine = hasExternal
    ? `   ╔═ ${config.title} ${"═".repeat(Math.max(0, width - config.title.length - 5))}╗`
    : `╔═ ${config.title} ${"═".repeat(Math.max(0, width - config.title.length - 5))}╗`;

  const lines: string[] = [];
  lines.push(titleLine);

  // Find max step width
  const maxStepWidth = Math.max(...config.steps.map((s) => s.label.length), 20);
  const stepWidth = maxStepWidth + 4;

  // Build internal steps
  const internalLines: string[] = [];

  for (let i = 0; i < config.steps.length; i++) {
    const step = config.steps[i];
    const boxLines = buildBox(
      step.label,
      step.type || "regular",
      step.shadow !== false,
      stepWidth,
    );

    // Center each box
    const leftPadding = calculateCenterPadding(stepWidth, width);

    for (const boxLine of boxLines) {
      internalLines.push(" ".repeat(leftPadding) + boxLine);
    }

    // Add vertical arrow between steps (except last)
    if (i < config.steps.length - 1) {
      const arrowPad = calculateCenterPadding(1, width);
      internalLines.push(" ".repeat(arrowPad) + "│");
      internalLines.push(" ".repeat(arrowPad) + "▼");
    }
  }

  // Combine external and internal elements
  const maxHeight = Math.max(externalBoxes.length, internalLines.length);

  for (let row = 0; row < maxHeight; row++) {
    let line = "";

    // External part
    if (row < externalBoxes.length) {
      line += externalBoxes[row];

      // Add connecting arrow on middle row
      if (row === Math.floor(externalBoxes.length / 2)) {
        line += "░".repeat(6) + "─".repeat(10) + "─▶║─";
      } else {
        line += "░".repeat(6) + " ".repeat(10) + "  ║ ";
      }
    } else if (hasExternal) {
      line += " ".repeat(externalWidth);
      if (row < internalLines.length) {
        line += "░".repeat(6) + " ".repeat(10) + "  ║ ";
      }
    } else {
      line += " ".repeat(externalWidth);
    }

    // Internal container part
    if (row < internalLines.length) {
      line += internalLines[row];
    } else {
      line += " ".repeat(width);
    }

    line += "║░";
    lines.push(line);
  }

  // Close container
  const bottomPadding = hasExternal ? " ".repeat(externalWidth + 16) : "";
  const bottomLine = bottomPadding + "╚" + "═".repeat(width - 1) + "╝░";
  lines.push(bottomLine);

  const shadowLine =
    (hasExternal ? " ".repeat(externalWidth + 17) : " ") + "░".repeat(width);
  lines.push(shadowLine);

  return lines.join("\n");
}

function createHorizontalFlow(config: FlowDiagramConfig): string {
  const width = config.width || 70;
  const hasExternal =
    config.externalElements && config.externalElements.length > 0;
  const lines: string[] = [];

  // Calculate step widths
  const maxStepWidth = Math.max(...config.steps.map((s) => s.label.length), 16);
  const stepWidth = maxStepWidth + 4;
  const arrowGap = 12;
  const totalStepWidth =
    config.steps.length * stepWidth + (config.steps.length - 1) * arrowGap;
  const containerPadding = Math.max(
    4,
    Math.floor((width - totalStepWidth) / 2),
  );

  // Build internal boxes matrix
  const boxMatrix: string[][] = [];
  let maxHeight = 0;
  for (const step of config.steps) {
    const boxLines = buildBox(
      step.label,
      step.type || "regular",
      step.shadow !== false,
      stepWidth,
    );
    boxMatrix.push(boxLines);
    maxHeight = Math.max(maxHeight, boxLines.length);
  }

  // Title line - position based on external elements
  const titleLeftPad = hasExternal ? 26 : 26;
  const titleRepeatCount = Math.max(0, width - config.title.length - 5);
  const titleLine =
    " ".repeat(titleLeftPad) +
    `╔═ ${config.title} ${"═".repeat(titleRepeatCount)}╗`;
  lines.push(titleLine);

  // Build external box for rendering
  let externalBoxLines: string[] = [];
  if (hasExternal) {
    const extEl = config.externalElements![0];
    const extWidth = Math.max(extEl.label.length + 4, 16);
    externalBoxLines = buildBox(
      extEl.label,
      extEl.type || "regular",
      extEl.shadow !== false,
      extWidth,
    );
  }

  // Render content rows
  for (let row = 0; row < maxHeight; row++) {
    let line = "";

    // External elements on left (if present)
    if (hasExternal) {
      const extRow = row < externalBoxLines.length ? row : -1;

      if (extRow >= 0) {
        line += "   " + externalBoxLines[extRow];

        // Add connecting arrow on middle row
        if (row === Math.floor(externalBoxLines.length / 2)) {
          line += "░".repeat(5) + "─".repeat(8) + "─▶║─";
        } else {
          line += "░".repeat(5) + " ".repeat(8) + "  ║ ";
        }
      } else {
        line += " ".repeat(26) + "║ ";
      }
    } else {
      line += " ".repeat(26) + "║ ";
    }

    // Internal container boxes with proper padding
    line += " ".repeat(containerPadding);

    for (let i = 0; i < boxMatrix.length; i++) {
      const boxLines = boxMatrix[i];
      const boxLine =
        row < boxLines.length
          ? boxLines[row]
          : " ".repeat(stepWidth + (config.steps[i].shadow !== false ? 1 : 0));
      line += boxLine;

      // Add horizontal arrow between boxes
      if (i < boxMatrix.length - 1) {
        if (row === Math.floor(maxHeight / 2)) {
          line += "─".repeat(arrowGap) + "►";
        } else {
          line += " ".repeat(arrowGap + 1);
        }
      }
    }

    // Right padding and border
    const usedWidth = containerPadding + totalStepWidth;
    const rightPad = Math.max(0, width - usedWidth);
    line += " ".repeat(rightPad);
    line += "║░";
    lines.push(line);
  }

  // Close container
  const bottomLine = " ".repeat(26) + "╚" + "═".repeat(width - 1) + "╝░";
  lines.push(bottomLine);
  const shadowLine = " ".repeat(27) + "░".repeat(width);
  lines.push(shadowLine);

  return lines.join("\n");
}
