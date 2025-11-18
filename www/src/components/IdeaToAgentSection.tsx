"use client";

import React, { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Button } from "./ui/button";

const carouselData = [
  {
    id: 1,
    category: "LAUNCH FASTER",
    title: "Focus on core objectives",
    description: "Building AI agents is hard enough (iterate on prompts and evaluate LLMs, etc), the plumbing work shouldn't add to that complexity. Plano takes care of the critical plumbing work like routing and orchestration to agents that slows you down and locks you into rigid frameworks, freeing developers to innovate on what truly matters.",
    image: "/LaunchFaster.svg"
  },
  {
    id: 2,
    category: "BUILD WITH CHOICE", 
    title: "Rapidly incorporate LLMs",
    description: "Build with multiple LLMs or model versions with a single unified API. Plano centralizes access controls, offers resiliency for traffic to 100+ LLMs -- all without you having to write a single line of code.",
    image: "/BuildWithChoice.svg"
  },
  {
    id: 3,
    category: "RICH LEARNING SIGNALS",
    title: "Hyper-rich agent traces and logs",
    description: "Knowing when agents fail or delight users is a critical signal that feeds into a reinforcement learning and optimization cycle. Plano makes this trivial by sampling hyper-rich information traces from live production agentic interactions so that you can improve agent performance faster.",
    image: "/Telemetry.svg"
  },
  {
    id: 4,
    category: "SHIP CONFIDENTLY",
    title: "Centrally apply guardrail policies",
    description: "Plano comes built-in with a state-of-the-art guardrail model you can use for things like jailbreak detection. But you can easily extend those capabilities via plano's agent filter chain to apply custom policy checks in a centralized way and keep users engaged on topics relevant to your requirements.",
    image: "/ShipConfidently.svg"
  },
  {
    id: 5,
    category: "SCALABLE ARCHITECTURE",
    title: "Protocol-Native Infrastructure",
    description: "Plano's sidecar deployment model avoids library-based abstractions - operating as a protocol-native data plane that integrates seamlessly with your existing agents via agentic APIs (like v1/responses). This decouples your core agent logic from plumbing concerns - run it alongside any framework without code changes, vendor lock-in, or performance overhead.",
    image: "/Contextual.svg"
  }
];

export function IdeaToAgentSection() {
  const [currentSlide, setCurrentSlide] = useState(0);
  const [isAutoPlaying, setIsAutoPlaying] = useState(true);

  // Auto-advance slides
  useEffect(() => {
    if (!isAutoPlaying) return;
    
    const interval = setInterval(() => {
      setCurrentSlide((prev) => (prev + 1) % carouselData.length);
    }, 10000); // 10 seconds per slide

    return () => clearInterval(interval);
  }, [isAutoPlaying]);

  const handleSlideClick = (index: number) => {
    setCurrentSlide(index);
    setIsAutoPlaying(false);
    // Resume auto-play after 10 seconds
    setTimeout(() => setIsAutoPlaying(true), 10000);
  };

  return (
    <section className="relative py-24 px-6 lg:px-[102px]">
      <div className="max-w-[81rem] mx-auto">
        {/* Main Heading */}
        <h2 className="font-sans font-normal text-4xl tracking-[-2.96px]!  text-black mb-10">
          Idea to agent — without overhead
        </h2>

        {/* Progress Indicators */}
        <div className="flex gap-2 mb-12">
          {carouselData.map((_, index) => (
            <button
              key={index}
              onClick={() => handleSlideClick(index)}
              className="relative h-2 rounded-full overflow-hidden transition-all duration-300 hover:opacity-80"
              style={{ width: index === currentSlide ? '292px' : '293px' }}
            >
              {/* Background */}
              <div className="absolute inset-0 bg-black/6 rounded-full" />
              
              {/* Active Progress */}
              {index === currentSlide && (
                <motion.div
                  className="absolute inset-0 bg-[#7780d9] rounded-full"
                  initial={{ width: 0 }}
                  animate={{ width: "100%" }}
                  transition={{ duration: 10, ease: "linear" }}
                  key={currentSlide}
                />
              )}
              
              {/* Completed State */}
              {index < currentSlide && (
                <div className="absolute inset-0 bg-purple-200/90 rounded-full" />
              )}
            </button>
          ))}
        </div>

        {/* Carousel Content */}
        <div className="relative min-h-[300px] lg:min-h-[400px]">
          <AnimatePresence mode="wait">
            <motion.div
              key={currentSlide}
              initial={{ opacity: 0, x: 50 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -50 }}
              transition={{ duration: 0.5, ease: "easeInOut" }}
              className="absolute inset-0"
            >
              <div className="flex flex-col lg:flex-row lg:justify-between lg:items-center lg:gap-12">
                {/* Left Content */}
                <div className="flex-1">
                  <div className="max-w-[692px] mt-6">
                    {/* Category */}
                    <p className="font-mono font-bold text-[#2a3178] text-xl tracking-[1.92px]! mb-4 leading-[1.102]">
                      {carouselData[currentSlide].category}
                    </p>

                    {/* Title */}
                    <h3 className="font-sans font-medium text-[#9797ea] text-5xl tracking-[-2.96px]!  mb-7">
                      {carouselData[currentSlide].title}
                    </h3>

                    {/* Description */}
                    <div className="font-mono text-black text-lg leading-8 max-w-140">
                      <p className="mb-0">
                        {carouselData[currentSlide].description}
                      </p>
                    </div>

                    <Button className="mt-8">
                      Learn more
                    </Button>
                  </div>
                </div>

                {/* Right Image - Only show if current slide has an image */}
                {carouselData[currentSlide].image && (
                  <div className="hidden lg:flex shrink-0 w-[400px] xl:w-[500px] justify-end items-center ">
                    <img
                      src={carouselData[currentSlide].image}
                      alt={carouselData[currentSlide].category}
                      className="w-full h-auto max-h-[450px] object-contain"
                    />
                  </div>
                )}
              </div>
            </motion.div>
          </AnimatePresence>
        </div>
      </div>
    </section>
  );
}
