import React from "react";
import { NetworkAnimation } from "../NetworkAnimation";
import { Button } from "@katanemo/ui";
import Link from "next/link";

export function ResearchHero() {
  return (
    <section className="relative pt-8 sm:pt-12 lg:pt-1 pb-12 sm:pb-16 lg:pb-20 px-4 sm:px-6 lg:px-[102px] overflow-hidden">
      <div className="max-w-[81rem] mx-auto relative">
        <div className="hidden lg:block absolute inset-0 pointer-events-none">
          <NetworkAnimation className="!w-[500px] !h-[500px] xl:!w-[600px] xl:!h-[600px] 2xl:!w-[570px] 2xl:!h-[540px] !top-[15%]" />
        </div>
        <div className="lg:hidden absolute inset-0 pointer-events-none">
          <NetworkAnimation className="!w-[300px] !h-[300px] left-77! -top-2! opacity-90! " />
        </div>
        <div className="max-w-3xl relative z-10">
          {/* Badge */}
          <div className="mb-4 sm:mb-6">
            <div className="inline-flex flex-wrap items-center gap-1.5 sm:gap-2 px-3 sm:px-4 py-1 rounded-full bg-[rgba(185,191,255,0.4)] border border-[var(--secondary)] shadow backdrop-blur">
              <span className="text-xs sm:text-sm font-medium text-black/65">
                New!
              </span>
              <span className="text-xs sm:text-sm font-medium text-black hidden sm:inline">
                —
              </span>
              <span className="text-xs sm:text-sm font-[600] tracking-[-0.6px]! text-black leading-tight">
                <span className="">Plano Orchestrator models released</span>
              </span>
            </div>
          </div>

          {/* Main Heading */}
          <h1 className="text-4xl sm:text-4xl md:text-5xl lg:text-7xl font-medium leading-tight tracking-tighter text-black -ml-1 mb-3 mt-4">
            <span className="font-sans">Research</span>
          </h1>
        </div>

        {/* Description */}
        <div className="max-w-70 sm:max-w-2xl relative z-10">
          <p className="text-base sm:text-lg md:text-xl lg:text-[22px] font-sans font-normal tracking-[-1.0px] sm:tracking-[-1.22px]! text-black">
            Our applied research focuses on how to deliver agents safely,
            efficiently, and with improved real-world performance — critical for
            any AI application, but work that sits outside of any agent's core
            product logic.
          </p>
        </div>

        <div className="flex flex-col sm:flex-row items-stretch sm:items-start gap-3 sm:gap-4 mt-6 sm:mt-8 relative z-10">
          <Button asChild className="w-full sm:w-auto">
            <Link href="https://huggingface.co/katanemo">
              Available on Hugging Face
            </Link>
          </Button>
        </div>
      </div>
    </section>
  );
}
