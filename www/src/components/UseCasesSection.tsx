import React from "react";
import { ArrowRightIcon } from "lucide-react";
import { Button } from "./ui/button";

const useCasesData = [
  {
    id: 1,
    category: "FROM SAAS TO AGENTS",
    title: "Transform software into active agents",
    link: "Learn more"
  },
  {
    id: 2,
    category: "AGENTIC TASKS", 
    title: "Train models through live feedback",
    link: "Learn more"
  },
  {
    id: 3,
    category: "AGENTIC ROUTING",
    title: "Route logic across smart agents", 
    link: "Learn more"
  },
  {
    id: 4,
    category: "SAAS INTEGRATIONS",
    title: "Design networks of intelligent agents",
    link: "Learn more"
  }
];

export function UseCasesSection() {
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

        {/* 4 Box Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {useCasesData.map((useCase) => (
            <div
              key={useCase.id}
              className="bg-gradient-to-b from-[rgba(177,184,255,0.16)] to-[rgba(17,28,132,0.035)] border-2 border-[rgba(171,178,250,0.27)] rounded-md p-8 h-87 flex flex-col justify-between"
            >
              {/* Category */}
              <div className="mb-6">
                <p className="font-mono font-bold text-[#2a3178] text-lg tracking-[1.92px]! w-38 mb-6">
                  {useCase.category}
                </p>

                {/* Title */}
                <h3 className="font-sans font-medium text-black text-2xl lg:text-3xl tracking-[-1.2px]! leading-[1.102]">
                  {useCase.title}
                </h3>
              </div>

              {/* Learn More Link */}
              <div className="mt-auto">
                <button className="group flex items-center gap-2 font-mono font-bold text-[var(--primary)] text-lg tracking-[1.92px]! leading-[1.45] hover:text-[var(--primary-dark)] transition-colors">
                  LEARN MORE
                  <ArrowRightIcon className="w-4 h-4" />
                </button>
              </div>
            </div>
          ))}
        </div>

      </div>
    </section>
  );
}
