"use client";

import React, { useState } from "react";
import { ArrowRightIcon } from "lucide-react";
import { Button } from "./ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "./ui/dialog";
import { motion, AnimatePresence } from "framer-motion";

interface UseCase {
  id: number;
  category: string;
  title: string;
  summary: string;
  fullContent: string;
}

const useCasesData: UseCase[] = [
  {
    id: 1,
    category: "AGENT ORCHESTRATION",
    title: "Multi-agent systems without framework lock-in",
    summary: "Seamless routing and orchestration for complex agent interactions",
    fullContent: "Plano manages agent routing and orchestration without framework dependencies, allowing seamless multi-agent interactions. This is ideal for building complex systems like automated customer support or data processing pipelines, where agents hand off tasks efficiently to deliver end-to-end solutions faster."
  },
  {
    id: 2,
    category: "CONTEXT ENGINEERING",
    title: "Reusable filters for smarter agents",
    summary: "Inject data, reformulate queries, and enforce policies efficiently",
    fullContent: "Plano's filter chain encourages reuse and decoupling for context engineering tasks like injecting data, reformulating queries, and enforcing policy before calls reach an agent or LLM. This means faster debugging, cleaner architecture, and more accurate, on-policy agents —without bespoke glue code."
  },
  {
    id: 3,
    category: "REINFORCEMENT LEARNING",
    title: "Production signals for continuous improvement",
    summary: "Capture rich traces to accelerate training and refinement",
    fullContent: "Plano captures hyper-rich tracing and log samples from production traffic, feeding into reinforcement learning and fine-tuning cycles. This accelerates iteration in areas like recommendation engines, helping teams quickly identify failures, refine prompts, and boost agent effectiveness based on real-user signals."
  },
  {
    id: 4,
    category: "SECURITY",
    title: "Built-in guardrails and centralized policies",
    summary: "Safe scaling with jailbreak detection and access controls",
    fullContent: "With built-in guardrails, centralized policies, and access controls, Plano ensures safe scaling across LLMs, detecting issues like jailbreak attempts. This is critical for deployments in regulated fields like finance or healthcare, and minimizing risks while standardizing reliability and security of agents."
  },
  {
    id: 5,
    category: "ON-PREMISES",
    title: "Full data control in regulated environments",
    summary: "Deploy on private infrastructure without compromising features",
    fullContent: "Plano's lightweight sidecar model deploys effortlessly on your private infrastructure, empowering teams in regulated sectors to maintain full data control while benefiting from unified LLM access, custom filter chains, and production-grade tracing—without compromising on security or scalability."
  }
];

export function UseCasesSection() {
  const [selectedUseCase, setSelectedUseCase] = useState<UseCase | null>(null);

  return (
    <section className="relative py-24 px-6 lg:px-[102px]">
      <div className="max-w-[81rem] mx-auto">
        {/* Section Header */}
        <div className="mb-14">
          {/* USE CASES Badge */}
          <div className="mb-6">
            <div className="inline-flex items-center gap-2 px-4 py-1 rounded-full bg-[rgba(185,191,255,0.4)] border border-[var(--secondary)] shadow backdrop-blur">
              <span className="font-mono font-bold text-[#2a3178] text-sm tracking-[1.62px]!">USE CASES</span>
            </div>
          </div>

          {/* Main Heading and CTA Button */}
          <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-6">
            <h2 className="font-sans font-normal text-3xl lg:text-4xl tracking-[-2.88px]! text-black leading-[1.03]">
              What's possible with Plano
            </h2>
            <Button>
              Start building
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
              className="bg-gradient-to-b from-[rgba(177,184,255,0.16)] to-[rgba(17,28,132,0.035)] border-2 border-[rgba(171,178,250,0.27)] rounded-md p-8 h-90 flex flex-col justify-between cursor-pointer"
              onClick={() => setSelectedUseCase(useCase)}
            >
              {/* Category */}
              <div className="mb-6">
                <p className="font-mono font-bold text-[#2a3178] text-base tracking-[1.92px]! mb-4">
                  {useCase.category}
                </p>

                {/* Title */}
                <h3 className="font-sans font-medium text-black text-xl lg:text-2xl tracking-[-1.2px]! leading-[1.102]">
                  {useCase.title}
                </h3>
              </div>

              {/* Learn More Link */}
              <div className="mt-auto">
                <button className="group flex items-center gap-2 font-mono font-bold text-[var(--primary)] text-base tracking-[1.92px]! leading-[1.45] hover:text-[var(--primary-dark)] transition-colors">
                  LEARN MORE
                  <ArrowRightIcon className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
                </button>
              </div>
            </motion.div>
          ))}
        </div>
      </div>

      {/* Modal */}
      <Dialog open={selectedUseCase !== null} onOpenChange={(open) => !open && setSelectedUseCase(null)}>
        <AnimatePresence>
          {selectedUseCase && (
            <DialogContent className="max-w-2xl">
              <motion.div
                initial={{ opacity: 0, scale: 0.95, y: 10 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.95, y: 10 }}
                transition={{ duration: 0.2, ease: "easeOut" }}
              >
                <DialogHeader>
                  <div className="mb-4">
                    <p className="font-mono font-bold text-[#2a3178] text-sm tracking-[1.62px]! mb-3">
                      USE CASE
                    </p>
                    <DialogTitle className="font-sans font-medium text-3xl tracking-[-1.5px]! text-black leading-[1.1]">
                      {selectedUseCase.category}
                    </DialogTitle>
                  </div>
                  <DialogDescription className="font-mono text-[#494949] text-base leading-relaxed pt-2">
                    {selectedUseCase.fullContent}
                  </DialogDescription>
                </DialogHeader>
                <div className="mt-6 flex justify-end">
                  <Button onClick={() => setSelectedUseCase(null)}>
                    Close
                  </Button>
                </div>
              </motion.div>
            </DialogContent>
          )}
        </AnimatePresence>
      </Dialog>
    </section>
  );
}
