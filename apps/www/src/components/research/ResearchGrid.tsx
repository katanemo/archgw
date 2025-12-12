import React from "react";

interface ResearchItem {
  title: string;
  description: string;
}

const researchItems: ResearchItem[] = [
  {
    title: "Model Routing",
    description:
      "Great agent UX starts with using the best model for a task — fast and cheap when it can be, smarter when it needs to be — and our routing research gives developers the controls to do exactly that. Our policy-based router captures your evals and preferences, while our performance-based router learns from real traffic over time, so you can evolve model choices without retraining.",
  },
  {
    title: "Governance & Learning",
    description:
      "Building an agent is easy — knowing what it does in production and how to improve it is very hard. Our research focuses on making agent behavior observable and governable: studying how agents respond to real and adversarial traffic, policy changes, and turning signals into learning loops that make agents safer and more effective over time.",
  },
  {
    title: "Agentic Performance",
    description:
      "Better system performance comes from directing traffic to the right agents for each task or workflow. We build compact orchestration models that manage traffic between agents — ensuring clean handoffs, preserved context, and reliable multi-agent collaboration across distributed systems.",
  },
];

export function ResearchGrid() {
  return (
    <section className="relative py-12 sm:py-16 lg:py-20 px-4 sm:px-6 lg:px-[102px] bg-white">
      <div className="max-w-[81rem] mx-auto">
        {/* 3 Column Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 sm:gap-8 lg:gap-12">
          {researchItems.map((item, index) => (
            <div key={index} className="flex flex-col">
              {/* Title */}
              <h3 className="font-sans font-medium text-2xl sm:text-3xl lg:text-3xl tracking-[-1.5px] sm:tracking-[-2px]! text-black leading-[1.1] mb-2 sm:mb-4">
                {item.title}
              </h3>

              {/* Description */}
              <p className="text-black text-sm sm:text-base lg:text-lg leading-relaxed">
                {item.description}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
