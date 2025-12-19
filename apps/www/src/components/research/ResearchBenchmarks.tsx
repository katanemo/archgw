import React from "react";
import Image from "next/image";

export function ResearchBenchmarks() {
  return (
    <section className="relative py-12 sm:py-16 lg:py-20 px-4 sm:px-6 lg:px-[102px] bg-[#1a1a1a] border-b-2 border-white/10">
      <div className="max-w-[81rem] mx-auto">
        {/* Section Header */}
        <div className="mb-8 sm:mb-12 lg:mb-6">
          {/* BENCHMARKS Label */}
          <div className="mb-4 sm:mb-2">
            <div className="font-mono font-bold text-[#9797ea] text-sm sm:text-base lg:text-xl tracking-[1.44px] sm:tracking-[1.92px]! leading-[1.502]">
              BENCHMARKS
            </div>
          </div>

          {/* Title */}
          <h2 className="text-4xl sm:text-4xl md:text-5xl lg:text-4xl font-medium leading-tight tracking-[-0.06em]! text-white">
            <span className="font-sans">
              Production excellence, outperforming frontier LLMs
            </span>
          </h2>
        </div>

        {/* Benchmarks Image */}
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
            .benchmarks-scroll-container::-webkit-scrollbar {
              display: none;
            }
          `}</style>
          <div className="benchmarks-scroll-container inline-block pl-4 sm:pl-6">
            <Image
              src="/Benchmarks.svg"
              alt="Benchmarks"
              width={1200}
              height={600}
              className="h-auto"
              style={{ width: "1200px", maxWidth: "none", display: "block" }}
              priority
            />
          </div>
        </div>

        {/* Desktop: Normal display */}
        <div className="hidden lg:block w-full">
          <Image
            src="/Benchmarks.svg"
            alt="Benchmarks"
            width={1200}
            height={600}
            className="w-full h-auto"
            priority
          />
        </div>
      </div>
    </section>
  );
}
