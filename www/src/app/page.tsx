"use client";

import React from "react";
import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { IntroSection } from "@/components/IntroSection";
import { IdeaToAgentSection } from "@/components/IdeaToAgentSection";
import { UseCasesSection } from "@/components/UseCasesSection";
import { VerticalCarouselSection } from "@/components/VerticalCarouselSection";
import { HowItWorksSection } from "@/components/HowItWorksSection";
import { UnlockPotentialSection } from "@/components/UnlockPotentialSection";
import { Footer } from "@/components/Footer";
import { LogoCloud } from "@/components/LogoCloud";

export default function Home() {
  return (
    <div className="min-h-screen">
      <Navbar />
      <main className="pt-20">
        <Hero />
        <LogoCloud />
        <IntroSection />
        <IdeaToAgentSection />
        <UseCasesSection />
        <VerticalCarouselSection />
        <HowItWorksSection />
        <UnlockPotentialSection variant="transparent" />

        {/* Rest of the sections will be refactored next */}
      </main>
      <Footer />
    </div>
  );
}
