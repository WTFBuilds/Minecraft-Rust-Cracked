import { Architecture } from "@/components/architecture";
import { Compare } from "@/components/compare";
import { CtaBand } from "@/components/cta-band";
import { Features } from "@/components/features";
import { Footer } from "@/components/footer";
import { Hero } from "@/components/hero";
import { Marquee } from "@/components/marquee";
import { Modes } from "@/components/modes";
import { Roadmap } from "@/components/roadmap";
import { SiteNav } from "@/components/site-nav";

export default function Home() {
  return (
    <div className="relative overflow-x-clip">
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
