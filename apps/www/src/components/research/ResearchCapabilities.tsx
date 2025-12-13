"use client";

import React from "react";
import { motion } from "framer-motion";
import { MessagesSquare, GitFork, Route, RefreshCw } from "lucide-react";

interface Capability {
  id: number;
  title: string;
  description: string;
}

const capabilitiesData: Capability[] = [
  {
    id: 1,
    title: "Multi-turn Understanding",
    description:
      "Makes routing decisions based on full conversation history, maintaining contextual awareness across extended dialogues with evolving user needs.",
  },
  {
    id: 2,
    title: "Multi-Intent Detection",
    description:
      "Identifies when a single user message requires multiple agents simultaneously, enabling parallel/sequential routing to fulfill complex requests",
  },
  {
    id: 3,
    title: "Content-Dependency Routing",
    description:
      "Correctly interprets ambiguous or referential messages by leveraging prior conversation context for accurate routing decisions.",
  },
  {
    id: 4,
    title: "Conversational-Flow Handling",
    description:
      "Understands diverse interaction patterns including follow-ups, clarifications, confirmations, and corrections within ongoing conversations.",
  },
];

export function ResearchCapabilities() {
  return (
    <section className="relative py-12 sm:py-16 lg:py-20 px-4 sm:px-6 lg:px-[102px] bg-[#1a1a1a]">
      <div className="max-w-[81rem] mx-auto">
        {/* Section Header */}
        <div className="mb-8 sm:mb-12 lg:mb-10">
          {/* PLANO-4B CAPABILITIES Label */}
          <div className="mb-2 sm:mb-1">
            <div className="font-mono font-bold text-[#9797ea] text-sm sm:text-base lg:text-xl tracking-[1.44px] sm:tracking-[1.92px]! leading-[1.502]">
              PLANO-ORCHESTRATOR CAPABILITIES
            </div>
          </div>

          {/* Title */}
          <h2 className="text-4xl sm:text-4xl md:text-5xl lg:text-4xl font-medium leading-tight tracking-[-0.06em]! text-white mb-4">
            <span className="font-sans">
              Accurately route with confidence with no compromise
            </span>
          </h2>

          <p className="text-white/90 w-full sm:w-[75%] text-sm sm:text-base leading-relaxed">
            Designed for real-world deployments, it delivers strong performance
            across general conversations, coding tasks, and long-context
            multi-turn conversations, while remaining efficient enough for
            low-latency production environments.
          </p>
        </div>

        {/* Mobile: Icon card above title/description, stacked vertically */}
        <div className="lg:hidden grid grid-cols-1 gap-8">
          {capabilitiesData.map((capability) => {
            // Map each capability to its icon
            const iconMap: Record<
              number,
              React.ComponentType<{ className?: string }>
            > = {
              1: MessagesSquare, // Multi-turn Understanding
              2: GitFork, // Multi-Intent Detection
              3: Route, // Content-Dependency Routing
              4: RefreshCw, // Conversational-Flow Handling
            };

            const Icon = iconMap[capability.id];

            return (
              <div key={capability.id} className="flex flex-col">
                {/* Icon Card */}
                <motion.div
                  whileHover={{ y: -4 }}
                  transition={{ duration: 0.2 }}
                  className="bg-gradient-to-b from-[rgba(177,184,255,0.16)] to-[rgba(17,28,132,0.035)] border-2 border-[rgba(171,178,250,0.27)] rounded-md p-6 h-40 flex items-center justify-center mb-4"
                >
                  {Icon && <Icon className="w-24 h-24 text-[#9797ea]" />}
                </motion.div>

                {/* Title */}
                <h3 className="font-sans font-medium text-white text-xl tracking-[-1.2px]! leading-[1.102] mb-3">
                  {capability.title}
                </h3>

                {/* Description */}
                <p className="text-white/90 text-base leading-relaxed">
                  {capability.description}
                </p>
              </div>
            );
          })}
        </div>

        {/* Desktop: Icon cards separate from titles/descriptions */}
        <div className="hidden lg:grid lg:grid-cols-4 gap-6 mb-6">
          {capabilitiesData.map((capability) => {
            // Map each capability to its icon
            const iconMap: Record<
              number,
              React.ComponentType<{ className?: string }>
            > = {
              1: MessagesSquare, // Multi-turn Understanding
              2: GitFork, // Multi-Intent Detection
              3: Route, // Content-Dependency Routing
              4: RefreshCw, // Conversational-Flow Handling
            };

            const Icon = iconMap[capability.id];

            return (
              <motion.div
                key={capability.id}
                whileHover={{ y: -4 }}
                transition={{ duration: 0.2 }}
                className="bg-gradient-to-b from-[rgba(177,184,255,0.16)] to-[rgba(17,28,132,0.035)] border-2 border-[rgba(171,178,250,0.27)] rounded-md p-6 h-52 flex items-center justify-center"
              >
                {Icon && <Icon className="w-24 h-24 text-[#9797ea]" />}
              </motion.div>
            );
          })}
        </div>

        {/* Desktop: Titles and Descriptions Below Boxes */}
        <div className="hidden lg:grid lg:grid-cols-4 gap-6">
          {capabilitiesData.map((capability) => (
            <div key={capability.id}>
              {/* Title */}
              <h3 className="font-sans font-medium text-white text-2xl tracking-[-1.2px]! leading-[1.102] mb-4">
                {capability.title}
              </h3>

              {/* Description */}
              <p className="text-white/90 text-base leading-relaxed">
                {capability.description}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
