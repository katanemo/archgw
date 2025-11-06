"use client";

import React, { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Button } from "./ui/button";

const verticalCarouselData = [
  {
    id: 1,
    category: "INTRODUCTION",
    title: "Simple to revolutionary",
    description: "By handling the critical yet complex tasks of prompt processing, Plano enables you to focus on what truly matters — achieving your business goals. With built-in capabilities like intelligent task routing, jailbreaking prevention, and centralized observability, Plano ensures your system runs smoothly and securely from the start, while continuously scaling with your needs."
  },
  {
    id: 2,
    category: "OPEN SOURCE",
    title: "Built on proven foundations",
    description: "Plano is built on open-source technologies and proven infrastructure patterns. This ensures transparency, community-driven development, and the ability to customize and extend the platform to meet your specific needs while maintaining enterprise-grade reliability."
  },
  {
    id: 3,
    category: "BUILT ON ENVOY",
    title: "Enterprise-grade infrastructure", 
    description: "Leveraging the battle-tested Envoy Proxy, Plano inherits years of production-hardened networking capabilities. This foundation provides unmatched performance, reliability, and scalability for your AI agent infrastructure."
  },
  {
    id: 4,
    category: "PURPOSE-BUILT",
    title: "Designed for AI agents",
    description: "Unlike generic API gateways, Plano is purpose-built for AI agent workloads. Every feature is designed with prompt processing, model routing, and agent orchestration in mind, providing optimal performance for your AI applications."
  },
  {
    id: 5,
    category: "PROMPT ROUTING",
    title: "Intelligent request handling",
    description: "Advanced prompt routing capabilities ensure that each request is directed to the most appropriate model or agent. This intelligent routing optimizes for cost, performance, and accuracy based on your specific requirements and preferences."
  }
];

export function VerticalCarouselSection() {
  const [activeSlide, setActiveSlide] = useState(0);

  const handleSlideClick = (index: number) => {
    setActiveSlide(index);
  };

  return (
    <section className="relative bg-[#1a1a1a] text-white py-24 px-6 lg:px-[102px]">
      <div className="max-w-[81rem] mx-auto">
        {/* Main Heading */}
        <h2 className="font-sans font-normal text-3xl lg:text-4xl tracking-[-2.88px]! text-white leading-[1.03] mb-16 max-w-4xl">
          Basic scenarios to powerful agentic apps in minutes
        </h2>

        {/* Vertical Carousel Layout */}
        <div className="flex flex-col lg:flex-row">
          {/* Left Sidebar Navigation */}
          <div className="lg:w-72 flex-shrink-0">
            <div className="relative space-y-6">
              {/* Sliding Rectangle Indicator */}
              <motion.div
                className="absolute left-0 top-0 w-2 h-4 bg-[#6363d2] z-10 rounded-xs"
                animate={{
                  y: activeSlide * 52 + 6 // Each item is ~28px text + 24px gap = 52px, +10px to center smaller rectangle
                }}
                transition={{
                  type: "spring",
                  stiffness: 300,
                  damping: 30,
                  duration: 0.6
                }}
              />
              
              {verticalCarouselData.map((item, index) => (
                <div
                  key={item.id}
                  onClick={() => handleSlideClick(index)}
                  className="cursor-pointer relative pl-6 transition-all duration-300"
                >
                  {/* Category Text */}
                  <span className={`font-mono font-bold text-lg tracking-[1.69px]! transition-colors duration-300 ${
                    index === activeSlide 
                      ? 'text-[#acb3fe]' 
                      : 'text-[rgba(172,179,254,0.71)]'
                  }`}>
                    {item.category}
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* Right Content Area */}
          <div className="flex-1 min-h-[400px] relative lg:-ml-8">
            <AnimatePresence mode="wait">
              <motion.div
                key={activeSlide}
                initial={{ opacity: 0, x: 50 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -50 }}
                transition={{ duration: 0.5, ease: "easeInOut" }}
                className="absolute inset-0"
              >
                <div className="max-w-2xl">
                  {/* Title */}
                  <h3 className="font-sans font-medium text-[var(--primary)] text-2xl lg:text-[34px] tracking-[-2.4px]! leading-[1.03] mb-4">
                    {verticalCarouselData[activeSlide].title}
                  </h3>

                  {/* Description */}
                  <div className="font-mono text-white text-xl lg:text-lg leading-10 tracking-[-1.2px]! max-w-md">
                    <p className="mb-0">
                      {verticalCarouselData[activeSlide].description}
                    </p>
                  </div>
                </div>
              </motion.div>
            </AnimatePresence>
          </div>
        </div>

      </div>
    </section>
  );
}
