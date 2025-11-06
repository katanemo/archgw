import React from "react";
import Image from "next/image";

const customerLogos = [
  {
    name: "HuggingFace",
    src: "/logos/huggingface.svg"
  },
  {
    name: "T-Mobile",
    src: "/logos/tmobile.svg"
  },
  {
    name: "Chase",
    src: "/logos/chase.svg"
  },
  {
    name: "SanDisk",
    src: "/logos/sandisk.svg"
  },
  {
    name: "Oracle",
    src: "/logos/oracle.svg"
  }
];

export function LogoCloud() {
  return (
    <section className="relative py-8 px-6 lg:px-8">
      <div className="max-w-[81rem] mx-auto">
        <div className="flex items-center justify-center gap-8 lg:gap-12 flex-wrap lg:flex-nowrap">
          {customerLogos.map((logo) => (
            <div
              key={logo.name}
              className="flex items-center justify-center mx-auto opacity-60 hover:opacity-80 transition-opacity duration-300 w-48 h-12"
            >
              <Image
                src={logo.src}
                alt={`${logo.name} logo`}
                width={128}
                height={40}
                className="w-full h-full object-contain filter grayscale hover:grayscale-0 transition-all duration-300"
              />
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
