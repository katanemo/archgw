import React from "react";
import { Button } from "./ui/button";
import Link from "next/link";
import { NetworkAnimation } from "./NetworkAnimation";

export function Hero() {
  return (
    <section className="relative pt-15 pb-6 px-6 lg:px-8">
      <div className="hidden lg:block">
        <NetworkAnimation />
      </div>
      <div className="max-w-[81rem] mx-auto relative z-10">
        <div className="max-w-3xl mb-4">
          {/* Version Badge */}
          <div className="mb-6">
            <div className="inline-flex items-center gap-2 px-4 py-1 rounded-full bg-[rgba(185,191,255,0.4)] border border-[var(--secondary)] shadow backdrop-blur">
              <span className="text-sm font-medium text-black/65">v0.3.12</span>
              <span className="text-sm font-medium text-black">—</span>
              <span className="text-sm font-[600] tracking-[-0.6px]! text-black">RAG Agent Launch!</span>
            </div>
          </div>

          {/* Main Heading */}
          <h1 className="text-5xl lg:text-7xl font-normal leading-tight tracking-tight text-black mb-4 flex flex-col -space-y-3">
            <span className="font-sans">The AI-native </span>
            <span className="font-sans font-medium text-[var(--secondary)]">network for agents</span>
          </h1>
        </div>

        {/* Subheading with CTA Buttons on the right */}
        <div className="max-w-7xl flex flex-col lg:flex-row lg:items-center lg:justify-between gap-6">
          <p className="text-xl lg:text-2xl font-sans font-[500] tracking-[-1.92px]! text-black max-w-2xl">
            Build and scale AI agents without handling the low-level plumbing.
          </p>

          {/* CTA Buttons */}
          <div className="justify-self-end flex justify-end items-center gap-4">
            <Button asChild>
              <Link href="/get-started">Get started</Link>
            </Button>
            <Button variant="secondary" asChild>
              <Link href="/docs">Documentation</Link>
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}

