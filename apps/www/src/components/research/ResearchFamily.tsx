"use client";

import React from "react";
import Image from "next/image";
import { Check } from "lucide-react";
import { motion } from "framer-motion";

interface ModelFeature {
  text: string;
}

interface Model {
  id: number;
  name: string;
  logo: string;
  features: ModelFeature[];
}

const modelsData: Model[] = [
  {
    id: 1,
    name: "Plano-4B",
    logo: "/Plano4B-Logo.svg",
    features: [
      { text: "Optimized for production routing with sub-100ms latency" },
      { text: "84-87% accuracy on long-context scenarios" },
      { text: "Cost-effective model selection at scale" },
      { text: "Seamless agent orchestration capabilities" },
      { text: "Frontier-level performance at fraction of cost" },
    ],
  },
  {
    id: 2,
    name: "Plano-30B-A3B",
    logo: "/Plano30B-Logo.svg",
    features: [
      { text: "Advanced routing intelligence for complex workflows" },
      { text: "Enhanced context understanding and preservation" },
      { text: "Superior accuracy for multi-agent coordination" },
      { text: "Enterprise-grade performance and reliability" },
      { text: "Scalable architecture for high-throughput systems" },
    ],
  },
];

export function ResearchFamily() {
  return (
    <section className="relative py-16 sm:py-20 lg:py-24 px-4 sm:px-6 lg:px-[102px] bg-white">
      <div className="max-w-[81rem] mx-auto">
        {/* Section Header */}
        <div className="mb-8 sm:mb-12 lg:mb-10">
          {/* PLANO FAMILY Label */}
          <div className="mb-4 sm:mb-2">
            <div className="font-mono font-bold text-[#2A3178] text-sm sm:text-base lg:text-xl tracking-[1.44px] sm:tracking-[1.92px]! leading-[1.502]">
              PLANO FAMILY
            </div>
          </div>

          {/* Title */}
          <h2 className="text-4xl sm:text-4xl md:text-5xl lg:text-4xl font-medium leading-tight tracking-[-0.06em]! text-black -ml-1">
            <span className="font-sans">Plano Models</span>
          </h2>
        </div>

        {/* 2 Card Grid - Side by Side */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
          {modelsData.map((model) => (
            <motion.div
              key={model.id}
              whileHover={{ y: -4 }}
              transition={{ duration: 0.2 }}
              className="bg-gradient-to-b from-[rgba(177,184,255,0.16)] to-[rgba(17,28,132,0.035)] border-2 border-[rgba(171,178,250,0.27)] rounded-md p-6 sm:p-6 lg:p-6 h-72"
            >
              {/* Empty box - content is below */}
            </motion.div>
          ))}
        </div>

        {/* Titles and Descriptions Below Boxes */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {modelsData.map((model) => (
            <div key={model.id}>
              {/* Logo */}
              <div className="mb-6">
                <Image
                  src={model.logo}
                  alt={model.name}
                  width={200}
                  height={60}
                  className="h-12 w-auto"
                />
              </div>

              {/* Features List */}
              <div>
                {model.features.map((feature, index) => (
                  <div key={index} className="flex items-start gap-3 mb-4">
                    <Check className="w-5 h-5 text-[var(--primary)] flex-shrink-0 mt-0.5" />
                    <p className="font-mono text-black text-sm sm:text-base tracking-[-0.8px] sm:tracking-[-1.2px]! leading-relaxed">
                      {feature.text}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
