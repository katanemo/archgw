import React from "react";
import Image from "next/image";

export function ResearchTimeline() {
  return (
    <section className="relative py-12 sm:py-16 lg:py-16 px-4 sm:px-6 lg:px-[102px] bg-white border-b border-gray-200">
      <div className="max-w-[81rem] mx-auto">
        {/* Timeline Image */}
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
            .timeline-scroll-container::-webkit-scrollbar {
              display: none;
            }
          `}</style>
          <div className="timeline-scroll-container inline-block pl-4 sm:pl-6">
            <Image
              src="/Timeline.svg"
              alt="Research Timeline"
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
            src="/Timeline.svg"
            alt="Research Timeline"
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
