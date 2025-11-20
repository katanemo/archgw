"use client";

import React from "react";
import Image from "next/image";

export function HowItWorksSection() {
  return (
    <section className="bg-[#1a1a1a] text-white pb-28 px-6 lg:px-[102px]">
      <div className="max-w-324 mx-auto">
        <div className="flex flex-col gap-16">
          {/* Header and Description */}
          <div className="max-w-4xl">
            <h2 className="font-sans font-normal text-3xl lg:text-4xl tracking-[-2.4px]! text-white leading-[1.03] mb-8">
              A high-level overview of how Plano works
            </h2>
            <div className="font-mono text-white text-xl lg:text-lg leading-10 tracking-[-1.2px]!">
              <p className="mb-0">
                Plano offers a delightful developer experience with a simple configuration file that describes the types of prompts your agentic app supports, a set of APIs that need to be plugged in for agentic scenarios (including retrieval queries) and your choice of LLMs.
              </p>
            </div>
          </div>

          {/* Large Diagram */}
          <div className="w-full">
             <Image
              src="/HowItWorks.svg"
              alt="How Plano Works Diagram"
              width={1200}
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

