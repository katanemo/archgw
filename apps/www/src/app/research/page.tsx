"use client";

import {
  ResearchHero,
  ResearchGrid,
  ResearchTimeline,
  ResearchCTA,
  ResearchCapabilities,
  ResearchBenchmarks,
  ResearchFamily,
} from "@/components/research";
import { UnlockPotentialSection } from "@/components/UnlockPotentialSection";

export default function ResearchPage() {
  return (
    <>
      <ResearchHero />
      <ResearchGrid />
      <ResearchTimeline />
      <ResearchCTA />
      <ResearchCapabilities />
      <ResearchBenchmarks />
      {/* <ResearchFamily /> */}
      <UnlockPotentialSection variant="transparent" />
    </>
  );
}
