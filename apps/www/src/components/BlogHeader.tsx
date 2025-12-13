"use client";

import { motion } from "framer-motion";

export function BlogHeader() {
  return (
    <motion.section
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{
        duration: 0.5,
        ease: "easeOut",
      }}
    >
      <div className="max-w-[85rem] mx-auto px-4 sm:px-6 lg:px-8 pt-8 sm:pt-12 lg:pt-1 pb-8 sm:pb-12 lg:pb-16">
        <h1 className="text-4xl sm:text-5xl lg:text-7xl font-normal leading-tight tracking-tighter text-black mb-3 sm:mb-4">
          <span className="font-sans">What's new with Plano</span>
        </h1>
        <p className="text-base sm:text-lg md:text-xl lg:text-2xl font-sans font-normal tracking-[-1.0px] sm:tracking-[-1.2px] text-black max-w-3xl">
          Building the future of infrastructure and tools for AI developers.
        </p>
      </div>
    </motion.section>
  );
}
