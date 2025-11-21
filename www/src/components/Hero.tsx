import React from "react";
import { Button } from "./ui/button";
import Link from "next/link";
import { NetworkAnimation } from "./NetworkAnimation";

export function Hero() {
  return (
    <section className="relative pt-8 sm:pt-12 lg:pt-15 pb-6 px-4 sm:px-6 lg:px-8">
      <div className="hidden lg:block">
        <NetworkAnimation />
      </div>
      <div className="max-w-[81rem] mx-auto relative z-10">
        <div className="max-w-3xl mb-6 sm:mb-4">
          {/* Version Badge */}
          <div className="mb-4 sm:mb-6">
            <div className="inline-flex flex-wrap items-center gap-1.5 sm:gap-2 px-3 sm:px-4 py-1 rounded-full bg-[rgba(185,191,255,0.4)] border border-[var(--secondary)] shadow backdrop-blur">
              <span className="text-xs sm:text-sm font-medium text-black/65">
                v0.4
              </span>
              <span className="text-xs sm:text-sm font-medium text-black hidden sm:inline">
                —
              </span>
              <span className="text-xs sm:text-sm font-[600] tracking-[-0.6px]! text-black leading-tight">
                <span className="hidden sm:inline">
                  Unified /v1/responses API with state management
                </span>
                <span className="sm:hidden">Unified /v1/responses API</span>
              </span>
            </div>
          </div>

          {/* Main Heading */}
          <h1 className="text-4xl sm:text-4xl md:text-5xl lg:text-7xl font-normal leading-tight tracking-tighter text-black mb-4 sm:mb-6 flex flex-col gap-0 sm:-space-y-2 lg:-space-y-3">
            <span className="font-sans">Models-native </span>
            <span className="font-sans font-medium text-[var(--secondary)]">
              dataplane for agents
            </span>
          </h1>
        </div>

        {/* Subheading with CTA Buttons */}
        <div className="max-w-7xl flex flex-col lg:flex-row lg:items-end lg:justify-between gap-6">
          <p className="text-base sm:text-lg md:text-xl lg:text-2xl font-sans font-[400] tracking-[-1.2px] sm:tracking-[-1.92px]! text-black max-w-4xl">
            Build agents faster, and scale them reliably by offloading the
            plumbing work in AI.
          </p>

          {/* CTA Buttons */}
          <div className="mb-0.5 flex flex-col sm:flex-row items-stretch sm:items-center gap-3 sm:gap-4 w-full sm:w-auto lg:justify-end">
            <Button asChild className="w-full sm:w-auto">
              <Link href="/get-started">Get started</Link>
            </Button>
            <Button variant="secondary" asChild className="w-full sm:w-auto">
              <Link href="/docs">Documentation</Link>
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}
