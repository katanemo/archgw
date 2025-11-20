"use client";

import React, { useState } from "react";
import Image from "next/image";
import { motion, AnimatePresence } from "framer-motion";
import { Button } from "./ui/button";

const verticalCarouselData = [
  {
    id: 1,
    category: "INTRODUCTION",
    title: "Simple to revolutionary",
    description: "Plano is an intelligent (edge and LLM) proxy server designed for agents - to help you focus on core business objectives. Arch handles critical but the pesky tasks related to the handling and processing of prompts, which includes detecting and rejecting jailbreak attempts, intelligent task routing for improved accuracy, mapping user requests into 'backend' functions, and managing the observability of prompts and LLM in a centralized way.",
    diagram: "/Introduction.svg"
  },
  {
    id: 2,
    category: "OPEN SOURCE",
    title: "Freedom to extend & deploy",
    description: "No lock-in. No black boxes. Just an open, intelligent (edge and LLM) proxy for building smarter, agentic AI applications. Created by contributors to Envoy Proxy, Arch brings enterprise-grade reliability to prompt orchestration, while giving you the flexibility to shape, extend, and integrate it into your AI workflows.",
    diagram: "/OpenSource.svg"
  },
  {
    id: 3,
    category: "BUILT ON ENVOY",
    title: "Production-proven infrastructure", 
    description: "Plano takes a dependency on Envoy and is a self-contained process designed to run alongside your application servers. Plano extends Envoy's HTTP connection management subsystem, filtering, and telemetry capabilities exclusively for prompts and LLMs. Use Plano with any application language or framework, and use Plano with any LLM provider.",
    diagram: "/BuiltOnEnvoy.svg"
  },
  {
    id: 4,
    category: "PURPOSE-BUILT",
    title: "Task-optimized, efficient LLMs",
    description: "Unlike generic API gateways, Plano is purpose-built for AI agent workloads. Every feature is designed with prompt processing, model routing, and agent orchestration in mind, providing optimal performance for your AI applications.",
    diagram: "/PurposeBuiltLLMs.svg"
  },
  {
    id: 5,
    category: "PROMPT ROUTING",
    title: "Intelligent request handling",
    description: "Prompt Targets are a core concept in Plano, enabling developers to define how different types of user prompts should get processed and routed. Define prompt targets, so you can seperate business logic from the complexities of processing and handling of prompts, focusing on the quality of your application and a cleaner seperation of concerns in your codebase.",
    diagram: "/PromptRouting.svg"
  }
];

export function VerticalCarouselSection() {
  const [activeSlide, setActiveSlide] = useState(0);

  const handleSlideClick = (index: number) => {
    setActiveSlide(index);
  };

  return (
    <section className="relative bg-[#1a1a1a] text-white py-24 px-6 lg:px-[102px]">
      <div className="max-w-324 mx-auto">
        {/* Main Heading */}
        <h2 className="font-sans font-normal text-3xl lg:text-4xl tracking-[-2.88px]! text-white leading-[1.03] mb-16 max-w-4xl">
          Basic scenarios to powerful agentic apps in minutes
        </h2>

        {/* Vertical Carousel Layout */}
        <div className="flex flex-col lg:flex-row">
          {/* Left Sidebar Navigation */}
          <div className="lg:w-72 shrink-0">
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
          <div className="flex-1 min-h-[600px] relative lg:-ml-8">
            <AnimatePresence mode="wait">
              <motion.div
                key={activeSlide}
                initial={{ opacity: 0, x: 50 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -50 }}
                transition={{ duration: 0.5, ease: "easeInOut" }}
                className="absolute inset-0"
              >
                <div className="flex flex-col lg:flex-row gap-12 items-start h-full">
                  {/* Text Content */}
                  <div className="flex-1 max-w-2xl">
                    {/* Title */}
                    <h3 className="font-sans font-medium text-primary text-2xl lg:text-[34px] tracking-[-2.4px]! leading-[1.03] mb-4">
                      {verticalCarouselData[activeSlide].title}
                    </h3>

                    {/* Description */}
                    <div className="font-mono text-white text-xl lg:text-lg leading-10 tracking-[-1.2px]! max-w-md">
                      <p className="mb-0">
                        {verticalCarouselData[activeSlide].description}
                      </p>
                    </div>
                  </div>

                  {/* Diagram - Right Side */}
                  <div className="flex-1 w-full flex items-start justify-center lg:justify-start pt-2">
                    <Image 
                      src={verticalCarouselData[activeSlide].diagram} 
                      alt={verticalCarouselData[activeSlide].title} 
                      width={600} 
                      height={400} 
                      className="w-full max-w-[600px] h-auto object-contain" 
                      priority 
                    />
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
