"use client";

import { Hero } from "@/components/Hero";
import { IntroSection } from "@/components/IntroSection";
import { IdeaToAgentSection } from "@/components/IdeaToAgentSection";
import { UseCasesSection } from "@/components/UseCasesSection";
import { VerticalCarouselSection } from "@/components/VerticalCarouselSection";
import { HowItWorksSection } from "@/components/HowItWorksSection";
import { UnlockPotentialSection } from "@/components/UnlockPotentialSection";
import { LogoCloud } from "@/components/LogoCloud";

export default function Home() {
  return (
    <>
      <Hero />
      <LogoCloud />
      <IntroSection />
      <IdeaToAgentSection />
      <UseCasesSection />
      <VerticalCarouselSection />
      <HowItWorksSection />
      <UnlockPotentialSection variant="transparent" />

      {/* Rest of the sections will be refactored next */}
    </>
  );
}
