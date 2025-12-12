"use client";

import React from "react";
import Image from "next/image";

export function HowItWorksSection() {
  return (
    <section className="bg-[#1a1a1a] text-white pb-16 sm:pb-20 lg:pb-28 sm:pt-0 pt-20">
      <div className="max-w-312 mx-auto sm:pl-0">
        <div className="flex flex-col gap-8 sm:gap-12 lg:gap-16">
          {/* Header and Description */}
          <div className="max-w-4xl lg:-ml-[102px] lg:pl-[102px] sm:pl-0 pl-4">
            <h2 className="font-sans font-normal text-xl sm:text-2xl lg:text-3xl tracking-[-1.6px] sm:tracking-[-2px]! text-white leading-[1.03] mb-6 sm:mb-8">
              One configuration file to orchestrate
            </h2>
            <div className="text-white w-100 sm:w-full text-sm sm:text-lg lg:text-lg">
              <p className="mb-0">
                Plano offers a delightful developer experience with a simple
                configuration file that describes the types of prompts your
                agentic app supports, a set of APIs that need to be plugged in
                for agentic scenarios (including retrieval queries) and your
                choice of LLMs.
              </p>
            </div>
          </div>

          {/* Large Diagram - Scrollable on mobile, normal on desktop */}
          {/* Mobile: Full-width scrollable container that extends to viewport edges */}
          <div
            className="mt-5 lg:hidden relative left-1/2 right-1/2 -ml-[50vw] -mr-[50vw] w-screen overflow-x-auto overflow-y-visible"
            style={{
              scrollbarWidth: "none",
              msOverflowStyle: "none",
              WebkitOverflowScrolling: "touch",
            }}
          >
            <style jsx>{`
              .diagram-scroll-container::-webkit-scrollbar {
                display: none;
              }
            `}</style>
            <div className="diagram-scroll-container inline-block">
              <Image
                src="/HowItWorks.svg"
                alt="How Plano Works Diagram"
                width={1200}
                height={600}
                className="h-auto"
                style={{ width: "1200px", maxWidth: "none", display: "block" }}
                priority
              />
            </div>
          </div>

          {/* Desktop: Extends to container edges */}
          <div className="hidden lg:block -w-[calc(10%+20px)] -mx-[10px]">
            <Image
              src="/HowItWorks.svg"
              alt="How Plano Works Diagram"
              width={10}
              height={10}
              className="w-full h-auto"
              priority
            />
          </div>
        </div>
      </div>
    </section>
  );
}
