"use client";

import React, { useState } from "react";
import {
  ArrowRightIcon,
  Network,
  Filter,
  TrendingUp,
  Shield,
  Server,
  XIcon,
} from "lucide-react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogClose,
} from "@katanemo/ui";
import { motion, AnimatePresence } from "framer-motion";
import Link from "next/link";

interface UseCase {
  id: number;
  category: string;
  title: string;
  summary: string;
  fullContent: string;
  icon: React.ComponentType<{ className?: string }>;
  gradient: string;
}

const useCasesData: UseCase[] = [
  {
    id: 1,
    category: "AGENT ORCHESTRATION",
    title: "Multi-agent systems without framework lock-in",
    summary:
      "Seamless routing and orchestration for complex agent interactions",
    fullContent:
      "Plano manages agent routing and orchestration without framework dependencies, allowing seamless multi-agent interactions. This is ideal for building complex systems like automated customer support or data processing pipelines, where agents hand off tasks efficiently to deliver end-to-end solutions faster.",
    icon: Network,
    gradient:
      "from-[rgba(119,128,217,0.15)] via-[rgba(119,128,217,0.08)] to-[rgba(17,28,132,0.05)]",
  },
  {
    id: 2,
    category: "CONTEXT ENGINEERING",
    title: "Reusable filters for smarter agents",
    summary:
      "Inject data, reformulate queries, and enforce policies efficiently",
    fullContent:
      "Plano's filter chain encourages reuse and decoupling for context engineering tasks like injecting data, reformulating queries, and enforcing policy before calls reach an agent or LLM. This means faster debugging, cleaner architecture, and more accurate, on-policy agents —without bespoke glue code.",
    icon: Filter,
    gradient:
      "from-[rgba(177,184,255,0.15)] via-[rgba(177,184,255,0.08)] to-[rgba(17,28,132,0.05)]",
  },
  {
    id: 3,
    category: "REINFORCEMENT LEARNING",
    title: "Production signals for continuous improvement",
    summary: "Capture rich traces to accelerate training and refinement",
    fullContent:
      "Plano captures hyper-rich tracing and log samples from production traffic, feeding into reinforcement learning and fine-tuning cycles. This accelerates iteration in areas like recommendation engines, helping teams quickly identify failures, refine prompts, and boost agent effectiveness based on real-user signals.",
    icon: TrendingUp,
    gradient:
      "from-[rgba(185,191,255,0.15)] via-[rgba(185,191,255,0.08)] to-[rgba(17,28,132,0.05)]",
  },
  {
    id: 4,
    category: "CENTRALIZED SECURITY",
    title: "Built-in guardrails and centralized policies",
    summary: "Safe scaling with jailbreak detection and access controls",
    fullContent:
      "With built-in guardrails, centralized policies, and access controls, Plano ensures safe scaling across LLMs, detecting issues like jailbreak attempts. This is critical for deployments in regulated fields like finance or healthcare, and minimizing risks while standardizing reliability and security of agents.",
    icon: Shield,
    gradient:
      "from-[rgba(119,128,217,0.15)] via-[rgba(119,128,217,0.08)] to-[rgba(17,28,132,0.05)]",
  },
  {
    id: 5,
    category: "ON-PREMISES DEPLOYMENT",
    title: "Full data control in regulated environments",
    summary: "Deploy on private infrastructure without compromising features",
    fullContent:
      "Plano's lightweight sidecar model deploys effortlessly on your private infrastructure, empowering teams in regulated sectors to maintain full data control while benefiting from unified LLM access, custom filter chains, and production-grade tracing—without compromising on security or scalability.",
    icon: Server,
    gradient:
      "from-[rgba(177,184,255,0.15)] via-[rgba(177,184,255,0.08)] to-[rgba(17,28,132,0.05)]",
  },
];

export function UseCasesSection() {
  const [selectedUseCase, setSelectedUseCase] = useState<UseCase | null>(null);

  return (
    <section className="relative py-12 sm:py-16 lg:py-10 px-4 sm:px-6 lg:px-[102px]">
      <div className="max-w-[81rem] mx-auto">
        {/* Section Header */}
        <div className="mb-8 sm:mb-12 lg:mb-14">
          {/* USE CASES Badge */}
          <div className="mb-4 sm:mb-6">
            <div className="inline-flex items-center gap-2 px-3 sm:px-4 py-1 rounded-full bg-[rgba(185,191,255,0.4)] border border-[var(--secondary)] shadow backdrop-blur">
              <span className="font-mono font-bold text-[#2a3178] text-xs sm:text-sm tracking-[1.44px] sm:tracking-[1.62px]!">
                USE CASES
              </span>
            </div>
          </div>

          {/* Main Heading and CTA Button */}
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-6 sm:gap-6">
            <h2 className="font-sans font-normal text-2xl sm:text-3xl lg:text-4xl tracking-[-2px] sm:tracking-[-2.88px]! text-black leading-[1.03]">
              What's possible with Plano
            </h2>
            <Button asChild className="hidden lg:block">
              <Link href="https://docs.planoai.dev/get_started/quickstart">
                Start building
              </Link>
            </Button>
          </div>
        </div>

        {/* 5 Card Grid - Horizontal Row */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
          {useCasesData.map((useCase) => (
            <motion.div
              key={useCase.id}
              whileHover={{ y: -4 }}
              transition={{ duration: 0.2 }}
              className="bg-gradient-to-b from-[rgba(177,184,255,0.16)] to-[rgba(17,28,132,0.035)] border-2 border-[rgba(171,178,250,0.27)] rounded-md p-4 sm:p-6 lg:p-6 h-auto sm:h-64 md:h-72 lg:h-90 flex flex-col justify-between cursor-pointer"
              onClick={() => setSelectedUseCase(useCase)}
            >
              {/* Category */}
              <div className="mb-4 sm:mb-6">
                <p className="font-mono font-bold text-[#2a3178] text-sm sm:text-sm tracking-[1.44px] sm:tracking-[1.92px]! mb-3 sm:mb-4">
                  {useCase.category}
                </p>

                {/* Title */}
                <h3 className="font-sans font-normal text-black text-lg sm:text-xl lg:text-2xl tracking-[-1.2px]! leading-[1.102]">
                  {useCase.title}
                </h3>
              </div>

              {/* Learn More Link */}
              <div className="mt-auto">
                <button className="group flex items-center gap-2 font-mono font-bold text-[var(--primary)] text-sm sm:text-base tracking-[1.44px] sm:tracking-[1.92px]! leading-[1.45] hover:text-[var(--primary-dark)] transition-colors">
                  LEARN MORE
                  <ArrowRightIcon className="w-3.5 h-3.5 sm:w-4 sm:h-4 group-hover:translate-x-1 transition-transform" />
                </button>
              </div>
            </motion.div>
          ))}
        </div>

        {/* Start building button - Mobile only, appears last */}
        <div className="lg:hidden mt-8">
          <Button asChild className="w-full">
            <Link href="https://docs.planoai.dev/get_started/quickstart">
              Start building
            </Link>
          </Button>
        </div>
      </div>

      {/* Modal */}
      <Dialog
        open={selectedUseCase !== null}
        onOpenChange={(open) => !open && setSelectedUseCase(null)}
      >
        <AnimatePresence>
          {selectedUseCase &&
            (() => {
              const IconComponent = selectedUseCase.icon;
              return (
                <DialogContent
                  key={selectedUseCase.id}
                  className="max-w-[90rem]! p-0 overflow-hidden"
                  showCloseButton={false}
                >
                  <motion.div
                    initial={{ opacity: 0, scale: 0.98, y: 8 }}
                    animate={{ opacity: 1, scale: 1, y: 0 }}
                    exit={{ opacity: 0, scale: 0.98, y: 8 }}
                    transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
                    className="relative"
                  >
                    {/* Gradient Background */}
                    <div
                      className={`absolute inset-0 bg-gradient-to-br ${selectedUseCase.gradient} opacity-50`}
                    />

                    {/* Decorative Border */}
                    <div className="absolute inset-0 border-2 border-[rgba(171,178,250,0.3)] rounded-lg pointer-events-none" />

                    {/* Custom Close Button */}
                    <DialogClose className="absolute top-4 right-4 z-50 rounded-xs opacity-70 hover:opacity-100 transition-opacity focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-[rgba(171,178,250,0.5)] bg-white/80 backdrop-blur-sm p-2 hover:bg-white/90">
                      <XIcon className="w-5 h-5 text-[#2a3178]" />
                      <span className="sr-only">Close</span>
                    </DialogClose>

                    {/* Content Container */}
                    <div className="relative z-10 p-5 sm:p-8 md:p-10 lg:p-14">
                      {/* Header Section with Icon */}
                      <DialogHeader className="mb-4">
                        <div className="flex flex-col sm:flex-row sm:items-start gap-4 sm:gap-8 mb-8">
                          {/* Icon Container - hidden on mobile */}
                          <motion.div
                            initial={{ opacity: 0, scale: 0.95 }}
                            animate={{ opacity: 1, scale: 1 }}
                            transition={{
                              duration: 0.3,
                              ease: [0.16, 1, 0.3, 1],
                              delay: 0.1,
                            }}
                            className="hidden sm:flex shrink-0 w-14 h-14 sm:w-16 sm:h-16 rounded-xl bg-gradient-to-br from-[rgba(119,128,217,0.2)] to-[rgba(17,28,132,0.1)] border-2 border-[rgba(171,178,250,0.4)] items-center justify-center shadow-lg backdrop-blur-sm mx-0"
                          >
                            <IconComponent className="w-8 h-8 text-[#2a3178]" />
                          </motion.div>

                          {/* Title Section */}
                          <div className="flex-1 text-left mt-4 sm:mt-0">
                            <motion.p
                              initial={{ opacity: 0, x: -8 }}
                              animate={{ opacity: 1, x: 0 }}
                              transition={{
                                duration: 0.3,
                                ease: [0.16, 1, 0.3, 1],
                                delay: 0.15,
                              }}
                              className="font-mono font-bold text-[#2a3178] text-xs tracking-[1.62px]! mb-1 uppercase"
                            >
                              USE CASE
                            </motion.p>
                            <motion.div
                              initial={{ opacity: 0, x: -8 }}
                              animate={{ opacity: 1, x: 0 }}
                              transition={{
                                duration: 0.3,
                                ease: [0.16, 1, 0.3, 1],
                                delay: 0.2,
                              }}
                            >
                              <DialogTitle className="font-sans font-medium text-2xl sm:text-3xl lg:text-4xl xl:text-4xl tracking-[-1.5px]! text-black leading-[1.1] mb-4">
                                {selectedUseCase.title}
                              </DialogTitle>
                              <div className="inline-flex items-center px-3 py-1 rounded-full bg-[rgba(185,191,255,0.3)] border border-[rgba(171,178,250,0.4)] backdrop-blur-sm">
                                <span className="font-mono font-bold text-[#2a3178] text-xs tracking-[1.44px]!">
                                  {selectedUseCase.category}
                                </span>
                              </div>
                            </motion.div>
                          </div>
                        </div>
                      </DialogHeader>

                      <motion.div
                        initial={{ opacity: 0, y: 8 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{
                          duration: 0.3,
                          ease: [0.16, 1, 0.3, 1],
                          delay: 0.3,
                        }}
                        className="mb-10"
                      >
                        <DialogDescription className="text-[#494949] text-base lg:text-base xl:text-lg leading-relaxed max-w-none mb-0">
                          {selectedUseCase.fullContent}
                        </DialogDescription>
                      </motion.div>

                      {/* Footer with CTA - mobile friendly */}
                      <motion.div
                        initial={{ opacity: 0, y: 8 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{
                          duration: 0.3,
                          ease: [0.16, 1, 0.3, 1],
                          delay: 0.35,
                        }}
                        className="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 pt-8 border-t border-[rgba(171,178,250,0.2)]"
                      >
                        {/* "Ready to get started?" is now first in column on mobile */}
                        <div className="flex items-center gap-2 text-sm  text-neutral-500 justify-center sm:justify-start order-0">
                          <span>Ready to get started?</span>
                        </div>
                        <div className="flex flex-col sm:flex-row gap-3 w-full sm:w-auto order-1">
                          <Button asChild className="w-full sm:w-auto">
                            <Link
                              href="https://docs.planoai.dev/get_started/quickstart"
                              className="flex items-center gap-2"
                            >
                              Start building
                              <ArrowRightIcon className="w-4 h-4" />
                            </Link>
                          </Button>
                        </div>
                      </motion.div>
                    </div>
                  </motion.div>
                </DialogContent>
              );
            })()}
        </AnimatePresence>
      </Dialog>
    </section>
  );
}
