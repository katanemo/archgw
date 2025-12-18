import React from "react";
import { Button } from "@katanemo/ui";
import Link from "next/link";

export function ResearchCTA() {
  return (
    <section className="relative pt-16 sm:pt-20 lg:pt-24 pb-12 sm:pb-16 lg:pb-20 px-4 sm:px-6 lg:px-[102px] bg-[#1a1a1a]">
      <div className="max-w-[81rem] mx-auto relative z-10">
        <div className="max-w-4xl">
          {/* Main Heading */}
          <h1 className="text-4xl sm:text-4xl md:text-5xl lg:text-5xl font-medium leading-tight tracking-[-0.06em]! text-white -ml-1 mb-3 mt-4">
            <span className="font-sans">
              Meet Plano-Orchestrator. Our latest models.
            </span>
          </h1>
        </div>

        {/* Description with CTA Buttons */}
        <div className="max-w-5xl">
          <p className="leading-relaxed sm:text-lg md:text-lg lg:text-[18px] font-sans font-normal text-white/90 mb-6">
            Plano-Orchestrator is a family of state-of-the-art routing and
            orchestration models that decides which agent(s) or LLM(s) should
            handle each request, and in what sequence. Built for multi-agent
            orchestration systems, Plano-Orchestrator excels at analyzing user
            intent and conversation context to make precise routing and
            orchestration decisions.
          </p>

          {/* CTA Buttons */}
          <div className="flex flex-col sm:flex-row items-stretch sm:items-start gap-3 sm:gap-4">
            <Button asChild className="w-full sm:w-auto">
              <Link href="https://huggingface.co/katanemo">
                Download Plano models
              </Link>
            </Button>
            <Button variant="secondary" asChild className="w-full sm:w-auto">
              <Link href="https://docs.planoai.dev">
                Get Started with Plano
              </Link>
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}
