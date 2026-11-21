import { Architecture } from "@/components/architecture";
import { Compare } from "@/components/compare";
import { CtaBand } from "@/components/cta-band";
import { Features } from "@/components/features";
import { Footer } from "@/components/footer";
import { Hero } from "@/components/hero";
import { Marquee } from "@/components/marquee";
import { Modes } from "@/components/modes";
import { Roadmap } from "@/components/roadmap";
import { SiteNav, GITHUB_URL } from "@/components/site-nav";

const SITE_URL = "https://melon.wtfbuilds.com";

const jsonLd = {
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "Organization",
      "@id": `${SITE_URL}/#organization`,
      name: "WTFBUILDS",
      url: "https://wtfbuilds.com",
      description:
        "Witness The Future Builds — software, platforms, infrastructure, robotics, and technology beyond conventional boundaries.",
      sameAs: ["https://wtfbuilds.com", "https://github.com/WTFBuilds"],
    },
    {
      "@type": "WebSite",
      "@id": `${SITE_URL}/#website`,
      url: SITE_URL,
      name: "MELON",
      description: "Minecraft Arcade Infrastructure, Reimagined.",
      publisher: { "@id": `${SITE_URL}/#organization` },
      inLanguage: "en-US",
    },
    {
      "@type": "SoftwareApplication",
      "@id": `${SITE_URL}/#engine`,
      name: "MELON",
      applicationCategory: "DeveloperApplication",
      operatingSystem: "Cross-platform",
      description:
        "A high-performance Minecraft Java Edition 1.8.9 arcade server engine, written in Rust. Purpose-built for BedWars and competitive minigames.",
      url: SITE_URL,
      author: { "@id": `${SITE_URL}/#organization` },
      softwareVersion: "0.1.0",
      license: "https://github.com/WTFBuilds/Minecraft-Rust-Cracked/blob/main/LICENSE",
      codeRepository: GITHUB_URL,
      keywords:
        "Minecraft server engine, BedWars, Rust, arcade, Protocol 47, game engine",
      isAccessibleForFree: true,
    },
  ],
};

export default function Home() {
  return (
    <div className="relative overflow-x-clip">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <SiteNav />
      <main>
        <Hero />
        <Marquee />
        <Features />
        <Modes />
        <Compare />
        <Architecture />
        <Roadmap />
        <CtaBand />
      </main>
      <Footer />
    </div>
  );
}
