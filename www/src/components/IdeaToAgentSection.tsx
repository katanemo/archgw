"use client";

import React, { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Button } from "./ui/button";

const carouselData = [
  {
    id: 1,
    category: "LAUNCH FASTER",
    title: "Focus on core objectives",
    description: "Plano handles the heavy lifting for agentic apps with purpose-built LLMs that automate request clarification, query routing, and data extraction. Ship faster without juggling prompt engineering or infrastructure details.",
    image: "/LaunchFaster.svg"
  },
  {
    id: 2,
    category: "SCALE EFFICIENTLY", 
    title: "Build without limits",
    description: "From prototype to production, Plano scales with your needs. Handle thousands of concurrent requests while maintaining consistent performance and reliability across your agent network.",
    image: "/ShipConfidently.svg"
  },
  {
    id: 3,
    category: "DEPLOY CONFIDENTLY",
    title: "Production-ready infrastructure",
    description: "Enterprise-grade security, monitoring, and governance built-in. Deploy your agents with confidence knowing they're protected by battle-tested infrastructure and compliance frameworks.",
    image: "/BuildWithChoice.svg"
  },
  {
    id: 4,
    category: "INTEGRATE SEAMLESSLY",
    title: "Connect everything",
    description: "Unified API access across all major LLM providers. Switch between models, combine capabilities, and route requests intelligently without vendor lock-in or complex integrations.",
    image: "/Telemetry.svg"
  },
  {
    id: 5,
    category: "OPTIMIZE CONTINUOUSLY",
    title: "Learn and improve",
    description: "Built-in analytics and feedback loops help your agents get smarter over time. Track performance, identify bottlenecks, and optimize workflows with real-time insights and automated improvements.",
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
