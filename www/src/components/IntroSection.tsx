import React from "react";
import { AsciiDiagram } from "@/components/AsciiDiagram";
import { diagrams } from "@/data/diagrams";

export function IntroSection() {
  return (
    <section className="relative bg-[#1a1a1a] text-white py-20 px-6 lg:px-[102px]">
      <div className="max-w-324 mx-auto">
        <div className="flex flex-col lg:flex-row gap-12 items-start">
          {/* Left Content */}
          <div className="flex-1">
            {/* Heading */}
            <h2 className="font-sans font-medium tracking-[-1.92px]! text-[#9797ea] text-4xl leading-[1.102] mb-6 max-w-[633px]">
              Go beyond AI nascent demos
            </h2>

            {/* Body Text */}
            <div className="font-mono tracking-[-0.96px]! text-white text-lg max-w-[713px]">
              <p className="mb-0 leading-8">
                Plano is the infrastructure layer for building, scaling, and routing AI agents. It sits between your app and your models, enforcing safety guardrails, orchestrating multi-agent workflows, and unifying access across large language models.
              </p>
              <br />
              <p className="mb-0 leading-8">
                Developers use Plano to build faster, platform teams use it to unify governance, and AI leads use it to run continuously learning systems that stay aligned, safe, and performant. From SaaS backends to distributed ecosystems, Plano turns raw agent logic into something you can deploy, monitor, and evolve in production.
              </p>
            </div>
          </div>

          {/* Right Diagram */}
          <div className="lg:w-[660px] relative flex-shrink-0">
            <AsciiDiagram content={diagrams.infrastructureLayer} className="max-w-none mx-0" />
          </div>
        </div>
      </div>
    </section>
  );
}

