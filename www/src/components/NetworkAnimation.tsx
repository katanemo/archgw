"use client";

import React, { useEffect, useState } from "react";
import { motion } from "framer-motion";

interface Node {
  id: string;
  x: number;
  y: number;
  size: number;
  delay: number;
}

interface Connection {
  from: string;
  to: string;
}

interface Particle {
  id: string;
  connectionIndex: number;
  progress: number;
}

// 4 separate groups positioned to the right of the heading text, aligned with the text vertically
const nodes: Node[] = [
  // Group 1
  { id: "1", x: 20, y: 35, size: 18, delay: 0 },
  { id: "2", x: 70, y: 42, size: 12, delay: 0.8 },
  { id: "3", x: 76, y: 38, size: 16, delay: 1.6 },
  
  // // Group 2
  // { id: "4", x: 80, y: 30, size: 18, delay: 0.4 },
  // { id: "5", x: 87, y: 36, size: 12, delay: 1.2 },
  // { id: "6", x: 92, y: 33, size: 16, delay: 2.0 },
  
  // Group 3
  { id: "7", x: 62, y: 48, size: 10, delay: 0.6 },
  { id: "8", x: 65, y: 52, size: 18, delay: 1.4 },
  { id: "9", x: 75, y: 48, size: 14, delay: 0.6 },
];

const connections: Connection[] = [
  // Group 1 connections
  { from: "1", to: "2" },
  { from: "2", to: "3" },
  
  // // Group 2 connections
  // { from: "4", to: "5" },
  // { from: "5", to: "6" },
  
  // Group 3 connections
  { from: "7", to: "8" },
  { from: "8", to: "9" },
];

export function NetworkAnimation() {
  const [particles, setParticles] = useState<Particle[]>([]);

  // Create and animate particles along connections - much slower
  useEffect(() => {
    const createParticle = () => {
      const connectionIndex = Math.floor(Math.random() * connections.length);
      const particle: Particle = {
        id: `particle-${Date.now()}-${Math.random()}`,
        connectionIndex,
        progress: 0,
      };
      
      setParticles((prev) => [...prev, particle]);

      // Remove particle after animation completes
      setTimeout(() => {
        setParticles((prev) => prev.filter((p) => p.id !== particle.id));
      }, 5000); // Slower duration
    };

    // Create particles at slower intervals
    const interval = setInterval(createParticle, 2500);
    return () => clearInterval(interval);
  }, []);

  // Animate particles - slower movement
  useEffect(() => {
    const interval = setInterval(() => {
      setParticles((prev) =>
        prev
          .map((p) => ({ ...p, progress: p.progress + 0.008 })) // Much slower
          .filter((p) => p.progress <= 1)
      );
    }, 50);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="absolute inset-0 pointer-events-none z-0 opacity-8 rotate-20 translate-x-40 -translate-y-40 scale-75 lg:scale-100 xl:scale-125 2xl:scale-150 [@media(min-width:1800px)]:scale-[1.60]">
      <svg
        className="absolute inset-0 w-full h-full rotate-10"
        preserveAspectRatio="xMidYMid slice"
        viewBox="790 -90 1020 840"
        style={{ overflow: "visible" }}
      >
        <defs>
          {/* Simple glow filter */}
          <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="3" result="coloredBlur" />
            <feMerge>
              <feMergeNode in="coloredBlur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {/* Render connections - simple lines with lower opacity */}
        {connections.map((conn, index) => {
          const fromNode = nodes.find((n) => n.id === conn.from);
          const toNode = nodes.find((n) => n.id === conn.to);
          if (!fromNode || !toNode) return null;

          const x1 = (fromNode.x / 100) * 1920;
          const y1 = (fromNode.y / 100) * 800;
          const x2 = (toNode.x / 100) * 1920;
          const y2 = (toNode.y / 100) * 800;

          return (
            <line
              key={`line-${conn.from}-${conn.to}`}
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              stroke="#8b91e8"
              strokeWidth="1.5"
              opacity={1}
            />
          );
        })}

        {/* Render data particles with lower opacity */}
        {particles.map((particle) => {
          const conn = connections[particle.connectionIndex];
          const fromNode = nodes.find((n) => n.id === conn.from);
          const toNode = nodes.find((n) => n.id === conn.to);
          if (!fromNode || !toNode) return null;

          const x = ((fromNode.x + (toNode.x - fromNode.x) * particle.progress) / 100) * 1920;
          const y = ((fromNode.y + (toNode.y - fromNode.y) * particle.progress) / 100) * 800;

          return (
            <motion.circle
              key={particle.id}
              cx={x}
              cy={y}
              r={4}
              fill="#b9bfff"
              initial={{ opacity: 0 }}
              animate={{ 
                opacity: [0, 0.4, 0.4, 0]
              }}
              transition={{ 
                duration: 5,
                times: [0, 0.2, 0.8, 1],
                ease: "linear"
              }}
            />
          );
        })}

        {/* Render nodes with subtle floating animation */}
        {nodes.map((node) => {
          const size = node.size;
          const baseX = (node.x / 100) * 1920;
          const baseY = (node.y / 100) * 800;

          return (
            <motion.circle
              key={node.id}
              cx={baseX}
              cy={baseY}
              r={size}
              fill="#8b91e8"
              animate={{
                cx: [baseX - 2, baseX + 2, baseX - 2],
                cy: [baseY - 2, baseY + 2, baseY - 2],
              }}
              transition={{
                duration: 10 + node.delay,
                delay: node.delay,
                repeat: Infinity,
                ease: "easeInOut",
              }}
            />
          );
        })}
      </svg>
    </div>
  );
}
