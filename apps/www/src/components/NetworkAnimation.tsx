"use client";

import React, { useId } from "react";
import { motion } from "framer-motion";

// Define the grid of squares with their positions and colors
const squares = [
  // Column 1 (x=3)
  { x: 3, y: 3, color: "#B0B7FF", col: 0, row: 0 },
  { x: 3, y: 6, color: "#B0B7FF", col: 0, row: 1 },
  { x: 3, y: 9, color: "#B0B7FF", col: 0, row: 2 },
  { x: 3, y: 12, color: "#ABB2FA", col: 0, row: 3 },
  { x: 3, y: 15, color: "#ABB2FA", col: 0, row: 4 },
  { x: 3, y: 18, color: "#ABB2FA", col: 0, row: 5 },
  { x: 3, y: 21, color: "#969FF4", col: 0, row: 6 },

  // Column 2 (x=6)
  { x: 6, y: 3, color: "#B0B7FF", col: 1, row: 0 },
  { x: 6, y: 6, color: "#B0B7FF", col: 1, row: 1 },
  { x: 6, y: 9, color: "#ABB2FA", col: 1, row: 2 },
  { x: 6, y: 12, color: "#ABB2FA", col: 1, row: 3 },
  { x: 6, y: 15, color: "#ABB2FA", col: 1, row: 4 },
  { x: 6, y: 18, color: "#969FF4", col: 1, row: 5 },
  { x: 6, y: 21, color: "#969FF4", col: 1, row: 6 },

  // Column 3 (x=9)
  { x: 9, y: 3, color: "#B0B7FF", col: 2, row: 0 },
  { x: 9, y: 6, color: "#ABB2FA", col: 2, row: 1 },
  { x: 9, y: 9, color: "#ABB2FA", col: 2, row: 2 },
  { x: 9, y: 12, color: "#ABB2FA", col: 2, row: 3 },
  { x: 9, y: 15, color: "#969FF4", col: 2, row: 4 },
  { x: 9, y: 18, color: "#969FF4", col: 2, row: 5 },
  { x: 9, y: 21, color: "#969FF4", col: 2, row: 6 },

  // Column 4 (x=12)
  { x: 12, y: 3, color: "#ABB2FA", col: 3, row: 0 },
  { x: 12, y: 6, color: "#ABB2FA", col: 3, row: 1 },
  { x: 12, y: 9, color: "#ABB2FA", col: 3, row: 2 },
  { x: 12, y: 12, color: "#969FF4", col: 3, row: 3 },
  { x: 12, y: 15, color: "#969FF4", col: 3, row: 4 },
  { x: 12, y: 18, color: "#969FF4", col: 3, row: 5 },
  { x: 12, y: 21, color: "#969FF4", col: 3, row: 6 },

  // Column 5 (x=15)
  { x: 15, y: 3, color: "#ABB2FA", col: 4, row: 0 },
  { x: 15, y: 6, color: "#ABB2FA", col: 4, row: 1 },
  { x: 15, y: 9, color: "#969FF4", col: 4, row: 2 },
  { x: 15, y: 12, color: "#969FF4", col: 4, row: 3 },
  { x: 15, y: 15, color: "#969FF4", col: 4, row: 4 },
  { x: 15, y: 18, color: "#969FF4", col: 4, row: 5 },
  { x: 15, y: 21, color: "#969FF4", col: 4, row: 6 },

  // Column 6 (x=18)
  { x: 18, y: 3, color: "#ABB2FA", col: 5, row: 0 },
  { x: 18, y: 6, color: "#969FF4", col: 5, row: 1 },
  { x: 18, y: 9, color: "#969FF4", col: 5, row: 2 },
  { x: 18, y: 12, color: "#969FF4", col: 5, row: 3 },
  { x: 18, y: 15, color: "#969FF4", col: 5, row: 4 },
  { x: 18, y: 18, color: "#969FF4", col: 5, row: 5 },
  { x: 18, y: 21, color: "#969FF4", col: 5, row: 6 },

  // Column 7 (x=21)
  { x: 21, y: 3, color: "#969FF4", col: 6, row: 0 },
  { x: 21, y: 6, color: "#969FF4", col: 6, row: 1 },
  { x: 21, y: 9, color: "#969FF4", col: 6, row: 2 },
  { x: 21, y: 12, color: "#969FF4", col: 6, row: 3 },
  { x: 21, y: 15, color: "#969FF4", col: 6, row: 4 },
  { x: 21, y: 18, color: "#969FF4", col: 6, row: 5 },
  { x: 21, y: 21, color: "#969FF4", col: 6, row: 6 },
];

interface NetworkAnimationProps {
  className?: string;
}

// Deterministic seeded random number generator for consistent SSR/client values
function seededRandom(seed: number): number {
  const x = Math.sin(seed) * 10000;
  return x - Math.floor(x);
}

// Round to fixed precision to avoid floating-point precision differences
function roundToPrecision(value: number, precision: number = 10): number {
  return Math.round(value * Math.pow(10, precision)) / Math.pow(10, precision);
}

// Generate deterministic random values based on index
function getDeterministicValues(index: number) {
  const seed1 = index * 0.1;
  const seed2 = index * 0.2;
  const seed3 = index * 0.3;
  const seed4 = index * 0.4;
  const seed5 = index * 0.5;
  const seed6 = index * 0.6;

  return {
    duration: roundToPrecision(3 + seededRandom(seed1) * 3, 10), // 3-6 seconds
    peakOpacity: roundToPrecision(0.7 + seededRandom(seed2) * 0.3, 10),
    baseOpacity: roundToPrecision(0.3 + seededRandom(seed3) * 0.2, 10),
    midOpacity: roundToPrecision(0.5 + seededRandom(seed4) * 0.2, 10),
    baseBrightness: roundToPrecision(0.85 + seededRandom(seed5) * 0.15, 10),
    peakBrightness: roundToPrecision(1.0 + seededRandom(seed6) * 0.2, 10),
  };
}

export function NetworkAnimation({ className }: NetworkAnimationProps) {
  // Generate unique IDs for gradient and mask to avoid conflicts when multiple instances exist
  const gradientId = useId().replace(/:/g, "-");
  const maskId = useId().replace(/:/g, "-");

  return (
    <div className="absolute inset-0 pointer-events-none opacity-100">
      <motion.div
        className={`absolute
        top-[9%] right-[-3%] w-[380px] h-[380px] ${className || ""}`}
        initial={{
          rotate: 9, // Start at the same rotation as animation to prevent flicker
        }}
        animate={{
          rotate: [9, 10, 9], // Slight breathing rotation
        }}
        transition={{
          duration: 8,
          repeat: Infinity,
          ease: "easeInOut",
        }}
      >
        <svg
          width="100%"
          height="100%"
          viewBox="0 0 32 32"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <defs>
            {/* Gradient mask: transparent at bottom, opaque at top */}
            <linearGradient id={gradientId} x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="white" stopOpacity="1" />
              <stop offset="50%" stopColor="white" stopOpacity="0.5" />
              <stop offset="100%" stopColor="white" stopOpacity="0" />
            </linearGradient>
            <mask id={maskId}>
              <rect width="26" height="26" fill={`url(#${gradientId})`} />
            </mask>
          </defs>

          <g mask={`url(#${maskId})`}>
            {/* Outer border */}
            <rect width="26" height="26" fill="#7780D9" />

            {/* Inner background */}
            <rect x="2" y="2" width="22" height="22" fill="#B9BFFF" />

            {/* Animated squares with wave effect */}
            {squares.map((square, index) => {
              // Use deterministic values based on index for SSR/client consistency
              const {
                duration,
                peakOpacity,
                baseOpacity,
                midOpacity,
                baseBrightness,
                peakBrightness,
              } = getDeterministicValues(index);

              return (
                <motion.path
                  key={`square-${index}`}
                  d={`M${square.x} ${square.y}H${square.x + 2}V${square.y + 2}H${square.x}V${square.y}Z`}
                  fill={square.color}
                  initial={{
                    opacity: roundToPrecision(baseOpacity, 10),
                    filter: `brightness(${roundToPrecision(baseBrightness, 10)})`,
                  }}
                  animate={{
                    opacity: [
                      roundToPrecision(baseOpacity, 10),
                      roundToPrecision(midOpacity, 10),
                      roundToPrecision(peakOpacity, 10),
                      roundToPrecision(midOpacity, 10),
                      roundToPrecision(baseOpacity, 10),
                    ],
                    filter: [
                      `brightness(${roundToPrecision(baseBrightness, 10)})`,
                      `brightness(${roundToPrecision((baseBrightness + peakBrightness) / 2, 10)})`,
                      `brightness(${roundToPrecision(peakBrightness, 10)})`,
                      `brightness(${roundToPrecision((baseBrightness + peakBrightness) / 2, 10)})`,
                      `brightness(${roundToPrecision(baseBrightness, 10)})`,
                    ],
                  }}
                  transition={{
                    duration: roundToPrecision(duration, 10),
                    delay: 0, // No delay - instant start
                    repeat: Infinity,
                    ease: "easeInOut",
                    repeatDelay: 0, // No pause between cycles
                  }}
                />
              );
            })}
          </g>
        </svg>
      </motion.div>
    </div>
  );
}
