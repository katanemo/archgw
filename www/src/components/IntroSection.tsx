import React from "react";
import Image from "next/image";

export function IntroSection() {
  return (
    <section className="relative bg-[#1a1a1a] text-white py-20 px-6 lg:px-[102px]">
      <div className="max-w-324 mx-auto">
        <div className="flex flex-col lg:flex-row gap-12 items-center">
          {/* Left Content */}
          <div className="flex-1">
            {/* Heading */}
            <p className="font-mono font-bold text-primary-light text-xl tracking-[1.92px]! mb-4 leading-[1.102]">
              WHY PLANO?
            </p>
            <h2 className="font-sans font-medium tracking-[-1.92px]! text-[#9797ea] text-4xl leading-[1.102] mb-6 max-w-[633px]">
              Ship prototypes to production
              <span className="italic">—fast.</span>
            </h2>

            {/* Body Text */}
            <div className="font-mono tracking-[-0.96px]! text-white text-sm sm:text-base lg:text-lg max-w-[713px]">
              <p className="mb-0">
                Plano is a framework-friendly proxy server and dataplane for
                agents, deployed as a sidecar. Plano handles the critical
                plumbing work in AI like agent routing and orchestration,
                comprehensive traces for agentic interactions, guardrail hooks,
                unified APIs for LLMs —
              </p>
              <p className="mb-0  mt-4">
                <strong>
                  <u>Developers</u>
                </strong>{" "}
                can focus more on modeling workflows,{" "}
                <strong>
                  <u>product teams</u>
                </strong>{" "}
                can accelerate feedback loops for reinforcement learning and{" "}
                <strong>
                  <u>engineering teams</u>
                </strong>{" "}
                can standardize policies and access controls across every agent
                and LLM for safer, more reliable scaling.
              </p>
            </div>
          </div>

          {/* Right Diagram */}
          <div className="flex-1 relative w-full">
            <Image
              src="/IntroDiagram.svg"
              alt="Network Path Diagram"
              width={800}
              height={600}
              className="w-full h-auto"
              priority
            />
          </div>
        </div>
      </div>
    </section>
  );
}
